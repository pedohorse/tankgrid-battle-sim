use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::mpsc::{Sender, TryRecvError};
use std::thread;
use std::time::{self, Instant};
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
use rustpython_vm::{compiler, Interpreter, PyResult, VirtualMachine};

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
    LookResult(Vec<T>),
}

pub struct ObjectCacheRepr<R> {
    pos: (i64, i64),
    rot: R,
    seethroughable: bool,
    passable: bool,
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

            for program in self.player_programs.iter() {
                let (command_sender, command_receiver) = mpsc::channel();
                let (result_sender, result_receiver) = mpsc::channel();
                let handle = scope.spawn(|| {
                    Self::program_runner(program.clone(), command_sender, result_receiver)
                });
                handles.push(Some(handle));
                channels.push((command_receiver, result_sender));
            }

            let mut next_commands: Vec<Option<(PlayerCommand<R>, &Sender<_>, GameTime)>> =
                vec![None; player_count];

            let mut start_timestamps = vec![time::Instant::now(); player_count];
            let timeout = time::Duration::from_secs(10);
            loop {
                let mut players_that_have_commands = 0;
                for (i, (command_receiver, result_sender)) in channels.iter().enumerate() {
                    if let Some(_) = next_commands[i] {
                        players_that_have_commands += 1;
                        continue;
                    }

                    match command_receiver.try_recv() {
                        Ok(com) => {
                            next_commands[i] = Some((com, &result_sender, self.time));
                            players_that_have_commands += 1;
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            next_commands[i] =
                                Some((PlayerCommand::Finish, &result_sender, self.time));
                            players_that_have_commands += 1;
                            continue;
                        }
                        _ => (),
                    }
                    // so we are still waiting for a command
                    // check for timeout
                    if time::Instant::now() - start_timestamps[i] > timeout {
                        // need to kill thread...
                        // cannot kill the thread, so just detach it
                        // TODO: think of how to handle this better
                        handles[i].take();
                        next_commands[i] = Some((PlayerCommand::Finish, &result_sender, self.time));
                        players_that_have_commands += 1;
                    }
                }

                // check if everyone is ready
                if players_that_have_commands == player_count {
                    // first check if all done
                    if next_commands
                        .iter()
                        .all(|x| PlayerCommand::Finish == x.unwrap().0)
                    {
                        break;
                    }

                    // if not done - select command to execute and advance time
                    if let Some((remaining_duration, player_i, next_command)) = next_commands
                        .iter_mut()
                        .enumerate()
                        .filter(|(_, k)| {
                            // filter out Finish commands
                            if let (PlayerCommand::Finish, _, _) = k.as_ref().unwrap() {
                                false
                            } else {
                                true
                            }
                        })
                        .map(|(player_i, k)| {
                            // calc remaining duration and other useful things
                            let (com, _, command_start_gametime) = k.as_ref().unwrap();
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
                            (command_start_gametime + duration - self.time, player_i, k)
                        })
                        .min_by_key(|k| k.0)
                    {
                        // process the next command

                        let (com, reply_channel, _) = next_command.take().unwrap();

                        // TODO: process the command
                        let reply = match com {
                            PlayerCommand::MoveFwd => {
                                let player_state = &mut self.player_states[player_i];
                                player_state.move_forward(&mut self.map, &self.logic, &self.object_layer);
                                // TODO: interact with the object if moved onto one
                                PlayerCommandReply::None
                            }
                            PlayerCommand::TurnCW => {
                                let player_state = &mut self.player_states[player_i];
                                player_state.turn_cw(&mut self.map, &self.logic, &self.object_layer);
                                PlayerCommandReply::None
                            }
                            PlayerCommand::TurnCCW => {
                                let player_state = &mut self.player_states[player_i];
                                player_state.turn_ccw(&mut self.map, &self.logic, &self.object_layer);
                                PlayerCommandReply::None
                            }
                            PlayerCommand::Shoot => {
                                // TODO: SHOOT !!
                                PlayerCommandReply::None
                            }
                            PlayerCommand::Wait => {
                                PlayerCommandReply::None
                            }
                            PlayerCommand::Look(ori) => {
                                let position = self.player_states[player_i].position();
                                self.object_layer.clear();
                                for player in self.player_states.iter() {
                                    self.object_layer.add(ObjectCacheRepr {
                                        pos: player.position(),
                                        rot: player.orientation(),
                                        seethroughable: player.seethroughable(),
                                        passable: player.passable(),
                                        script_repr: player.to_script_repr(),
                                    });
                                }
                                //self.recreate_objects_layer();
                                let look_result = self.map_prober.look(position, &self.map, &self.logic, &self.object_layer, ori).into_iter().map(|(t, maybe_obj)| {
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
                            *next_command = Some((PlayerCommand::Finish, reply_channel, self.time));
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

        let interpreter = Interpreter::without_stdlib(Default::default());
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
        for player in self.player_states.iter() {
            self.object_layer.add(ObjectCacheRepr {
                pos: player.position(),
                rot: player.orientation(),
                seethroughable: player.seethroughable(),
                passable: player.passable(),
                script_repr: player.to_script_repr(),
            });
        }
    }
}
