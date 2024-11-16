use std::mem;
use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::mpsc::{Sender, TryRecvError};
use std::thread;
use std::time::{self, Duration, Instant};
use std::{cell::RefCell, rc::Rc, sync::mpsc};

use super::gametime::GameTime;
use super::map::MapReadAccess;
use super::map_object::MapObject;
use super::map_prober::MapProber;
use super::maptile_logic::MaptileLogic;
use super::object_layer::ObjectLayer;
use super::player_state::PlayerControl;
use super::script_repr::{FromScriptRepr, ToScriptRepr};

use rustpython_vm::convert::ToPyObject;
use rustpython_vm::{compiler, Interpreter, PyResult, VirtualMachine, signal::{UserSignalReceiver, UserSignalSendError, user_signal_channel}};

#[derive(Clone, Copy, PartialEq)]
pub enum PlayerCommandState<PC> {
    None,
    GotCommand(PC, GameTime),
    Finish,
}

impl<PC> PlayerCommandState<PC> {
    pub fn take(&mut self) -> PlayerCommandState<PC> {
        mem::replace(self, PlayerCommandState::None)
    }

    pub fn unwrap(self) -> (PC, GameTime) {
        if let PlayerCommandState::GotCommand(c, t) = self {
            (c ,t)
        } else {
            panic!("unwrap failed!");
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerCommand<R> {
    MoveFwd,
    TurnCW,
    TurnCCW,
    Shoot,
    Wait,
    Look(R),
    Finish,
}

#[derive(Clone)]
pub enum PlayerCommandReply<T>
where
    T: Send + 'static,
{
    None,
    Bool(bool),
    LookResult(Vec<T>),
}

pub enum ObjectCacheType {
    Player(usize),
    //Stuff, // TODO: add stuff like pickable items
}

pub struct ObjectCacheRepr<R> {
    obj_type: ObjectCacheType,
    pos: (i64, i64),
    rot: R,
    seethroughable: bool,
    passable: bool,
    shootable: bool,
    script_repr: String,
}

impl<R> MapObject<R> for ObjectCacheRepr<R>
where
    R: Copy,
{
    fn orientation(&self) -> R {
        self.rot
    }

    fn position(&self) -> (i64, i64) {
        self.pos
    }

    fn passable(&self) -> bool {
        self.passable
    }

    fn seethroughable(&self) -> bool {
        self.seethroughable
    }
}

impl<R> ToScriptRepr for ObjectCacheRepr<R> {
    fn to_script_repr(&self) -> String {
        self.script_repr.clone()
    }
}

pub struct Battle<T, M, L, R, P, Pr, OCache>
where
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    R: Copy,
    P: PlayerControl<R, M, T, L, ObjectCacheRepr<R>, OCache>,
    Pr: MapProber<T, R, M, L, ObjectCacheRepr<R>, OCache>,
    OCache: ObjectLayer<R, ObjectCacheRepr<R>>,
{
    map: M,
    logic: L,
    map_prober: Pr,
    player_states: Vec<P>,
    player_programs: Vec<String>,
    time: GameTime,
    command_durations: HashMap<PlayerCommand<R>, GameTime>,
    object_layer: OCache,
    _marker0: PhantomData<T>,
    _marker1: PhantomData<R>,
}

pub const DEFAULT_COMMAND_DURATION: GameTime = 10;
pub const VM_THINK_TIMEOUT: time::Duration = time::Duration::from_secs(5);

impl<T, M, L, R, P, Pr, OCache> Battle<T, M, L, R, P, Pr, OCache>
where
    T: Copy + Clone + Send + ToScriptRepr,
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    R: Copy + Clone + Eq + Hash + Send + 'static + FromScriptRepr,
    P: PlayerControl<R, M, T, L, ObjectCacheRepr<R>, OCache> + MapObject<R> + ToScriptRepr,
    Pr: MapProber<T, R, M, L, ObjectCacheRepr<R>, OCache>,
    OCache: ObjectLayer<R, ObjectCacheRepr<R>>,
{
    pub fn new(
        map: M,
        logic: L,
        map_prober: Pr,
        player_initial_states_and_programs: Vec<(P, String)>,
        command_durations: HashMap<PlayerCommand<R>, usize>,
    ) -> Battle<T, M, L, R, P, Pr, OCache> {
        let (player_states, player_programs): (Vec<P>, Vec<String>) =
            player_initial_states_and_programs.into_iter().unzip();
        Battle {
            map,
            logic,
            map_prober,
            player_states,
            player_programs,
            time: 0,
            command_durations,
            object_layer: OCache::new(),
            _marker0: PhantomData,
            _marker1: PhantomData,
        }
    }

    pub fn time(&self) -> usize {
        self.time
    }

    pub fn player_state(&self, i: usize) -> &P {
        &self.player_states[i]
    }

    pub fn map(&self) -> &M {
        &self.map
    }

    pub fn run_simulation(&mut self) {
        self.time = 0;
        let player_count = self.player_programs.len();

        thread::scope(|scope| {
            let mut handles = Vec::with_capacity(player_count);
            let mut channels = Vec::with_capacity(player_count);
            let mut thread_stop_signal_senders = Vec::with_capacity(player_count);

            for program in self.player_programs.iter() {
                let (command_sender, command_receiver) = mpsc::channel();
                let (result_sender, result_receiver) = mpsc::channel();
                let (thread_stop_sender, thread_stop_receiver) = user_signal_channel();

                let handle = scope.spawn({
                    let program = program.clone();
                    || Self::program_runner(program, command_sender, result_receiver, thread_stop_receiver)
                });
                handles.push(Some(handle));
                channels.push(Some((command_receiver, result_sender)));
                thread_stop_signal_senders.push(Some(thread_stop_sender));
            }

            let mut next_commands: Vec<PlayerCommandState<PlayerCommand<R>>> =
                vec![PlayerCommandState::None; player_count];

            let mut start_timestamps = vec![time::Instant::now(); player_count];
            loop {
                let mut players_that_have_commands = 0;

                // check dead
                for (i, player) in self.player_states.iter().enumerate() {
                    if player.resource_value(0) <= 0 {
                        next_commands[i] = PlayerCommandState::Finish;
                    }
                }

                for (i, channels_maybe) in channels.iter().enumerate() {
                    let (command_receiver, _) = if let Some(x) = channels_maybe {
                        x
                    } else {
                        next_commands[i] = PlayerCommandState::Finish;
                        players_that_have_commands += 1;
                        continue;
                    };
                    if let PlayerCommandState::GotCommand(_, _) = next_commands[i] {
                        players_that_have_commands += 1;
                        continue;
                    }

                    match command_receiver.try_recv() {
                        Ok(com) => {
                            next_commands[i] = PlayerCommandState::GotCommand(com, self.time);
                            players_that_have_commands += 1;
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            next_commands[i] = PlayerCommandState::Finish;
                            players_that_have_commands += 1;
                            continue;
                        }
                        _ => (),
                    }
                    // so we are still waiting for a command
                    // check for timeout
                    if time::Instant::now() - start_timestamps[i] > VM_THINK_TIMEOUT {
                        next_commands[i] = PlayerCommandState::Finish;
                        players_that_have_commands += 1;
                    }
                }
                let players_that_have_commands = players_that_have_commands; // remove mut

                // ensure closed channels for Finished players
                for (i, ((next_command, channel), thread_stop_signal_sender)) in next_commands.iter().zip(channels.iter_mut()).zip(thread_stop_signal_senders.iter_mut()).enumerate() {
                    match (next_command, &channel) {
                        (PlayerCommandState::Finish, Some(_)) => {
                            channel.take();
                            // need to kill thread...
                            if let Some(chan) = thread_stop_signal_sender.take() {
                                // spray and pray
                                // in case there are many generic try-except clauses - we just send a shit ton of exceptions to except from exception handlers
                                for k in 0..999999 {
                                    match chan.send(Box::new(|vm| {
                                        Err(vm.new_runtime_error("program stopped".to_owned()))
                                    })) {
                                        Ok(_) => {
                                            if k%10 == 0 {println!("(attempt:{}) trying to stop the vm {} ...", k, i)};
                                            thread::sleep(Duration::from_nanos(1));
                                        },
                                        Err(_) => {
                                            println!("closing vm failed: probably vm {} already stopped", i);
                                            break;
                                        }
                                    };
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // check if everyone is ready
                if players_that_have_commands == player_count {
                    // first check if all done
                    if next_commands
                        .iter()
                        .all(|x| PlayerCommandState::Finish == *x)
                    {
                        break;
                    }

                    // if not done - select command to execute and advance time
                    if let Some((remaining_duration, player_i, next_command)) = next_commands
                        .iter_mut()
                        .enumerate()
                        .filter(|(_, k)| {
                            // filter out Finish commands
                            if let PlayerCommandState::Finish = k {
                                false
                            } else {
                                true
                            }
                        })
                        .map(|(player_i, k)| {
                            // calc remaining duration and other useful things
                            let (com, command_start_gametime) = if let PlayerCommandState::GotCommand(x, y) = k {
                                (x, y)
                            } else {
                                // filter above must have filtered Finish out, and None must not happen cuz of prev logic
                                unreachable!(); 
                            };
                            let player_state = &mut self.player_states[player_i];
                            // duration is based on map modifiers
                            let duration = if let Some(dur) = self.command_durations.get(com) {
                                let tile = {
                                    let (x, y) = player_state.position();
                                    self.map.get_tile_at(x, y)
                                };
                                let speed_percentage = match com {
                                    PlayerCommand::MoveFwd => {
                                        self.logic.pass_speed_percentage(tile)
                                    }
                                    PlayerCommand::TurnCW | PlayerCommand::TurnCCW => {
                                        self.logic.turn_speed_percentage(tile)
                                    }
                                    _ => 100,
                                };
                                // speed = 0 means we misconfigured something
                                let speed_percentage = if speed_percentage == 0 {
                                    eprintln!("[WARNING] tile speed == 0, seems like a misconfiguration, ignoring");
                                    100
                                } else { speed_percentage };

                                (*dur * 100) / (speed_percentage as usize)
                            } else {
                                DEFAULT_COMMAND_DURATION
                            };
                            //
                            (*command_start_gametime + duration - self.time, player_i, k)
                        })
                        .min_by_key(|k| k.0)
                    {
                        // process the next command

                        let (com, _) = next_command.take().unwrap();
                        
                        // ONLY Finished command may have None for reply channel by design
                        //  also we bravely unwrap cuz channels may close only in the start of the loop
                        let reply_channel = &channels[player_i].as_ref().unwrap().1;

                        let reply = match com {
                            PlayerCommand::MoveFwd => {
                                self.recreate_objects_layer();
                                let player_state = &mut self.player_states[player_i];
                                player_state.move_forward(&mut self.map, &self.logic, &self.object_layer);
                                // TODO: interact with the object if moved onto one
                                PlayerCommandReply::None
                            }
                            PlayerCommand::TurnCW => {
                                self.recreate_objects_layer();
                                let player_state = &mut self.player_states[player_i];
                                player_state.turn_cw(&mut self.map, &self.logic, &self.object_layer);
                                PlayerCommandReply::None
                            }
                            PlayerCommand::TurnCCW => {
                                self.recreate_objects_layer();
                                let player_state = &mut self.player_states[player_i];
                                player_state.turn_ccw(&mut self.map, &self.logic, &self.object_layer);
                                PlayerCommandReply::None
                            }
                            PlayerCommand::Shoot => {
                                let player_state = &mut self.player_states[player_i];
                                if player_state.resource_value(1) > 0 {
                                    player_state.expend_resource(1, 1);
                                    self.recreate_objects_layer();
                                    let player_state = &mut self.player_states[player_i];
                                    if let Some((hit_x, hit_y)) = self.map_prober.raycast(player_state.position(), &self.map, &self.logic, &self.object_layer, player_state.orientation(), true, false, true) {
                                        for obj_ref in self.object_layer.objects_at(hit_x, hit_y).into_iter() {
                                            if !obj_ref.shootable() {
                                                continue;
                                            }
                                            match obj_ref.obj_type {
                                                ObjectCacheType::Player(other_player_i) => {
                                                    let hit_enemy = &mut self.player_states[other_player_i];
                                                    hit_enemy.expend_resource(0, 1);
                                                }
                                            }
                                        }
                                    };
                                }

                                PlayerCommandReply::None
                            }
                            PlayerCommand::Wait => {
                                PlayerCommandReply::None
                            }
                            PlayerCommand::Look(ori) => {
                                self.recreate_objects_layer();
                                let look_result = self.map_prober.look(self.player_states[player_i].position(), &self.map, &self.logic, &self.object_layer, ori).into_iter().map(|(t, maybe_obj)| {
                                    (t.to_script_repr(), maybe_obj.map(|obj| {obj.to_script_repr()}))
                                }).collect();

                                PlayerCommandReply::LookResult(look_result)
                            }
                            PlayerCommand::Finish => {
                                unreachable!();
                            }
                        };
                        // send reply
                        if let Err(_) = reply_channel.send(reply) {
                            println!("failed to send reply to the player");
                            // consider player broken
                            *next_command = PlayerCommandState::Finish;
                            continue;
                        }

                        start_timestamps[player_i] = Instant::now(); // update timeout counter
                        self.time += remaining_duration;
                    } else {
                        // no min - means all commands are Finish, but that must have been checked before, so
                        unreachable!("should not be reached");
                    };
                }
            } // inf loop end

            for handle in handles.into_iter() {
                let handle = if let Some(h) = handle {
                    h
                } else {
                    continue;
                };

                match handle.join() {
                    Ok(Ok(_)) => {
                        println!("program finished fine");
                    }
                    Ok(Err(_)) => {
                        println!("program errored out");
                    }
                    Err(_) => {
                        println!("something went funky with the thread!");
                    }
                }
            }
        });
    }

    ///
    /// this is ran in a dedicated thread
    /// this represents a single tank AI,
    /// and runs a python interpreter with player ai code
    ///
    /// returns success or error if code produced an exception
    fn program_runner(
        program: String,
        command_channel: mpsc::Sender<PlayerCommand<R>>,
        reply_channel: mpsc::Receiver<PlayerCommandReply<(String, Option<String>)>>,
        vm_signal_receiver: UserSignalReceiver,
    ) -> Result<(), ()> {
        macro_rules! send_command {
            ($vm:ident, $command_channel:ident, $reply_channel:ident, $cmd:expr) => {{
                let command_channel = if let Some(x) = $command_channel.upgrade() {
                    x
                } else {
                    return PyResult::Err($vm.new_runtime_error("game is closed!".to_owned()));
                };
                let reply_channel = if let Some(x) = $reply_channel.upgrade() {
                    x
                } else {
                    return PyResult::Err($vm.new_runtime_error("game is closed!".to_owned()));
                };

                if let Err(_) = command_channel.borrow().send($cmd) {
                    return PyResult::Err($vm.new_runtime_error("game is closed!".to_owned()));
                };

                let ret = match reply_channel.borrow().recv() {
                    Ok(x) => x,
                    Err(_) => {
                        return PyResult::Err($vm.new_runtime_error("game is closed!!".to_owned()));
                    }
                };
                ret
            }};
        }

        let reply_channel = Rc::new(RefCell::new(reply_channel));
        let command_channel = Rc::new(RefCell::new(command_channel));

        let interpreter = Interpreter::with_init(Default::default(), |vm| {
            vm.set_user_signal_channel(vm_signal_receiver);
        });
        let ret = interpreter.enter(|vm| {
            let scope = vm.new_scope_with_builtins();

            macro_rules! add_function {
                ($fname:literal, $fn:block) => {
                    scope
                        .globals
                        .set_item($fname, vm.new_function($fname, $fn).into(), vm)
                        .unwrap();
                };
            }

            add_function!("turn_cw", {
                // TODO: figure out why do I have to downgrade refs?
                //  it's as if interpreter is not dropped properly and keeps refs
                let reply_channel = Rc::downgrade(&reply_channel);
                let command_channel = Rc::downgrade(&command_channel);
                move |vm: &VirtualMachine| -> PyResult<()> {
                    println!("TEST: turn_cw");
                    let _ret =
                        send_command!(vm, command_channel, reply_channel, PlayerCommand::TurnCW);
                    PyResult::Ok(())
                }
            });
            add_function!("turn_ccw", {
                let reply_channel = Rc::downgrade(&reply_channel);
                let command_channel = Rc::downgrade(&command_channel);
                move |vm: &VirtualMachine| -> PyResult<()> {
                    println!("TEST: turn_ccw");
                    let _ret =
                        send_command!(vm, command_channel, reply_channel, PlayerCommand::TurnCCW);
                    PyResult::Ok(())
                }
            });
            add_function!("move_forward", {
                let reply_channel = Rc::downgrade(&reply_channel);
                let command_channel = Rc::downgrade(&command_channel);
                move |vm: &VirtualMachine| -> PyResult<()> {
                    println!("TEST: move_forward");
                    let _ret =
                        send_command!(vm, command_channel, reply_channel, PlayerCommand::MoveFwd);
                    PyResult::Ok(())
                }
            });
            add_function!("shoot", {
                let reply_channel = Rc::downgrade(&reply_channel);
                let command_channel = Rc::downgrade(&command_channel);
                move |vm: &VirtualMachine| -> PyResult<()> {
                    println!("TEST: shoot");
                    let _ret =
                        send_command!(vm, command_channel, reply_channel, PlayerCommand::Shoot);
                    PyResult::Ok(())
                }
            });
            add_function!("wait", {
                let reply_channel = Rc::downgrade(&reply_channel);
                let command_channel = Rc::downgrade(&command_channel);
                move |vm: &VirtualMachine| -> PyResult<()> {
                    println!("TEST: wait");
                    let _ret =
                        send_command!(vm, command_channel, reply_channel, PlayerCommand::Wait);
                    PyResult::Ok(())
                }
            });
            add_function!("look", {
                let reply_channel = Rc::downgrade(&reply_channel);
                let command_channel = Rc::downgrade(&command_channel);
                move |direction: String, vm: &VirtualMachine| -> PyResult<_> {
                    println!("TEST: look");
                    let direction = if let Some(x) = R::from_script_repr(&direction) {
                        x
                    } else {
                        return PyResult::Err(
                            vm.new_runtime_error("bad direction value".to_owned()),
                        );
                    };
                    let ret = send_command!(
                        vm,
                        command_channel,
                        reply_channel,
                        PlayerCommand::Look(direction)
                    );
                    if let PlayerCommandReply::LookResult(look_result) = ret {
                        PyResult::Ok(
                            look_result
                                .into_iter()
                                .map(|t| t.to_pyobject(&vm))
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        PyResult::Err(vm.new_runtime_error("unexpected look reply".to_owned()))
                    }
                }
            });

            let code_obj = match vm.compile(&program, compiler::Mode::Exec, "<embedded>".to_owned())
            {
                Ok(x) => x,
                Err(e) => return Err(()),
            };

            if let PyResult::Err(_) = vm.run_code_obj(code_obj, scope) {
                return Err(());
            }

            Ok(())
        });
        interpreter.finalize(None);
        println!("program runner completed");
        ret
    }

    fn recreate_objects_layer(&mut self) {
        self.object_layer.clear();
        for (i, player) in self.player_states.iter().enumerate() {
            if player.resource_value(0) <= 0 {
                // if dead (TODO: may spawn a corpse object instead)
                continue;
            }
            self.object_layer.add(ObjectCacheRepr {
                obj_type: ObjectCacheType::Player(i),
                pos: player.position(),
                rot: player.orientation(),
                seethroughable: player.seethroughable(),
                passable: player.passable(),
                shootable: player.shootable(),
                script_repr: player.to_script_repr(),
            });
        }
    }
}
