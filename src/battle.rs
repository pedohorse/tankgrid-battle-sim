use std::cmp::Eq;
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::{self, Duration, Instant};
use std::{cell::RefCell, rc::Rc, sync::mpsc};

use super::command_logic::BattleLogic;
use super::gametime::GameTime;
use super::log_data::{LogRepresentable, LogWriter};

use super::player_state::PlayerControl;
use super::script_repr::ToScriptRepr;

use rustpython_vm::{
    compiler,
    signal::{user_signal_channel, UserSignalReceiver},
    Interpreter, PyResult,
};

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
            (c, t)
        } else {
            panic!("unwrap failed!");
        }
    }
}

pub struct Battle<P, BLogic, PCom, PComRep, LW>
where
    P: PlayerControl + LogRepresentable,
    PCom: LogRepresentable,
    BLogic: BattleLogic<P, PCom, PComRep>,
    LW: LogWriter<String, String>,
{
    player_states: Vec<P>,
    player_programs: Vec<String>,
    battle_logic: BLogic,
    time: GameTime,
    log_writer: LW,
    player_death_logged: Vec<bool>,
    _marker: PhantomData<(PCom, PComRep)>,
}

pub const DEFAULT_COMMAND_DURATION: GameTime = 10;
pub const VM_THINK_TIMEOUT: time::Duration = time::Duration::from_secs(5);

impl<P, BLogic, PCom, PComRep, LW> Battle<P, BLogic, PCom, PComRep, LW>
where
    P: PlayerControl + ToScriptRepr + LogRepresentable,
    BLogic: BattleLogic<P, PCom, PComRep>,
    PCom: LogRepresentable + Hash + Copy + PartialEq + Eq + Send + 'static,
    PComRep: Send + 'static,
    LW: LogWriter<String, String>,
{
    pub fn new(
        battle_logic: BLogic,
        player_initial_states_and_programs: Vec<(P, String)>,
        log_writer: LW,
    ) -> Battle<P, BLogic, PCom, PComRep, LW> {
        let (player_states, player_programs): (Vec<P>, Vec<String>) =
            player_initial_states_and_programs.into_iter().unzip();
        Battle {
            player_death_logged: vec![false; player_states.len()],
            player_states,
            player_programs,
            battle_logic,
            log_writer,
            time: 0,
            _marker: PhantomData,
        }
    }

    pub fn time(&self) -> usize {
        self.time
    }

    pub fn player_state(&self, i: usize) -> &P {
        &self.player_states[i]
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
                    || {
                        Self::program_runner(
                            program,
                            command_sender,
                            result_receiver,
                            thread_stop_receiver,
                        )
                    }
                });
                handles.push(Some(handle));
                channels.push(Some((command_receiver, result_sender)));
                thread_stop_signal_senders.push(Some(thread_stop_sender));
            }

            let mut next_commands: Vec<PlayerCommandState<PCom>> =
                vec![PlayerCommandState::None; player_count];

            let mut start_timestamps = vec![time::Instant::now(); player_count];
            loop {
                let mut players_that_have_commands = 0;

                // check dead
                for (i, (player, death_logged)) in self.player_states.iter().zip(self.player_death_logged.iter_mut()).enumerate() {
                    if player.is_dead() && !*death_logged {
                        next_commands[i] = PlayerCommandState::Finish;
                        self.log_writer.add_log_data(self.player_states[i].log_repr(), "dies".to_owned(), self.time, 0);
                        *death_logged = true;
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
                for (i, ((next_command, channel), thread_stop_signal_sender)) in next_commands
                    .iter()
                    .zip(channels.iter_mut())
                    .zip(thread_stop_signal_senders.iter_mut())
                    .enumerate()
                {
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
                                            if k % 10 == 0 {
                                                println!(
                                                    "(attempt:{}) trying to stop the vm {} ...",
                                                    k, i
                                                )
                                            };
                                            thread::sleep(Duration::from_nanos(1));
                                        }
                                        Err(_) => {
                                            println!(
                                                "closing vm failed: probably vm {} already stopped",
                                                i
                                            );
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
                    if let Some((remaining_duration, full_duration, player_i, next_command)) = next_commands
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
                            let (com, command_start_gametime) =
                                if let PlayerCommandState::GotCommand(x, y) = k {
                                    (x, y)
                                } else {
                                    // filter above must have filtered Finish out, and None must not happen cuz of prev logic
                                    unreachable!();
                                };
                            let player_state = &mut self.player_states[player_i];
                            // duration may be based in stats or map modifiers
                            let duration =
                                self.battle_logic.get_command_duration(&player_state, com);

                            //
                            (*command_start_gametime + duration - self.time, duration, player_i, k)
                        })
                        .min_by_key(|k| k.0)
                    {
                        // process the next command

                        let (com, _) = next_command.take().unwrap();

                        // ONLY Finished command may have None for reply channel by design
                        //  also we bravely unwrap cuz channels may close only in the start of the loop
                        let reply_channel = &channels[player_i].as_ref().unwrap().1;

                        let reply = self.battle_logic.process_commands(
                            player_i,
                            com,
                            &mut self.player_states,
                        );

                        // send reply
                        if let Err(_) = reply_channel.send(reply) {
                            println!("failed to send reply to the player");
                            // consider player broken
                            *next_command = PlayerCommandState::Finish;
                            continue;
                        }
                        
                        // log
                        self.log_writer.add_log_data(self.player_states[player_i].log_repr(), com.to_log_repr(), self.time, full_duration);
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
        command_channel: mpsc::Sender<PCom>,
        reply_channel: mpsc::Receiver<PComRep>, // PlayerCommandReply<(String, Option<String>)>
        vm_signal_receiver: UserSignalReceiver,
    ) -> Result<(), ()> {
        macro_rules! send_command {
            ($vm:ident, $command_channel:ident, $reply_channel:ident, $cmd:expr) => {{
                let command_channel = if let Some(x) = $command_channel.upgrade() {
                    x
                } else {
                    return Err(())
                    //return PyResult::Err($vm.new_runtime_error("game is closed!".to_owned()));
                };
                let reply_channel = if let Some(x) = $reply_channel.upgrade() {
                    x
                } else {
                    return Err(())
                    //return PyResult::Err($vm.new_runtime_error("game is closed!".to_owned()));
                };

                if let Err(_) = command_channel.borrow().send($cmd) {
                    return Err(())
                    //return PyResult::Err($vm.new_runtime_error("game is closed!".to_owned()));
                };

                let ret = match reply_channel.borrow().recv() {
                    Ok(x) => Ok(x),
                    Err(_) => {
                        Err(())
                        //return PyResult::Err($vm.new_runtime_error("game is closed!!".to_owned()));
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

            BLogic::initialize_scope(vm, &scope, {
                let reply_channel = Rc::downgrade(&reply_channel);
                let command_channel = Rc::downgrade(&command_channel);
                move |com: PCom| -> Result<PComRep, ()> {
                    send_command!(vm, command_channel, reply_channel, com)
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
}
