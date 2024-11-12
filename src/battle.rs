use std::cmp::Eq;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{self, Instant};
use std::{cell::RefCell, rc::Rc, sync::mpsc};

use crate::player_state::PlayerControl;

use super::map::MapReadAccess;
use super::maptile_logic::MaptileLogic;

use rustpython_vm::{compiler, Interpreter, PyResult, VirtualMachine};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerCommand {
    MoveFwd,
    TurnCW,
    TurnCCW,
    Finish,
}

#[derive(Clone, Copy)]
pub enum PlayerCommandReply {
    None,
}

pub struct Battle<T, M, L, R, P>
where
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    P: PlayerControl<R, M, T, L>,
{
    map: M,
    logic: L,
    player_states: Vec<P>,
    player_programs: Vec<String>,
    time: usize,
    command_durations: HashMap<PlayerCommand, usize>,
    _marker0: PhantomData<T>,
    _marker1: PhantomData<R>,
}

pub const DEFAULT_COMMAND_DURATION: usize = 10;

impl<T, M, L, R, P> Battle<T, M, L, R, P>
where
    T: Copy + Clone,
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    P: PlayerControl<R, M, T, L>,
{
    pub fn new(
        map: M,
        logic: L,
        player_initial_states: Vec<P>,
        player_programs: Vec<String>,
        command_durations: HashMap<PlayerCommand, usize>,
    ) -> Battle<T, M, L, R, P> {
        Battle {
            map,
            logic,
            player_states: player_initial_states,
            player_programs,
            time: 0,
            command_durations,
            _marker0: PhantomData,
            _marker1: PhantomData,
        }
    }

    pub fn time(&self) -> usize {
        self.time
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

            let mut next_commands: Vec<Option<(PlayerCommand, &Sender<_>, usize)>> =
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
                            let duration = if let Some(x) = self.command_durations.get(com) {
                                *x
                            } else {
                                DEFAULT_COMMAND_DURATION
                            };
                            (command_start_gametime + duration - self.time, player_i, k)
                        })
                        .min_by_key(|k| k.0)
                    {
                        // process the next command

                        let (com, reply_channel, _) = next_command.take().unwrap();

                        // TODO: process the command
                        let player_state = &mut self.player_states[player_i];
                        let reply = match com {
                            PlayerCommand::MoveFwd => {
                                player_state.move_forward(&mut self.map, &self.logic);
                                PlayerCommandReply::None
                            }
                            PlayerCommand::TurnCW => {
                                player_state.turn_cw(&mut self.map, &self.logic);
                                PlayerCommandReply::None
                            }
                            PlayerCommand::TurnCCW => {
                                player_state.turn_ccw(&mut self.map, &self.logic);
                                PlayerCommandReply::None
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

    fn program_runner(
        program: String,
        command_channel: mpsc::Sender<PlayerCommand>,
        reply_channel: mpsc::Receiver<PlayerCommandReply>,
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
                    let ret =
                        send_command!(vm, command_channel, reply_channel, PlayerCommand::TurnCW);
                    PyResult::Ok(())
                }
            });
            add_function!("turn_ccw", {
                let reply_channel = Rc::downgrade(&reply_channel);
                let command_channel = Rc::downgrade(&command_channel);
                move |vm: &VirtualMachine| -> PyResult<()> {
                    println!("TEST: turn_ccw");
                    let ret =
                        send_command!(vm, command_channel, reply_channel, PlayerCommand::TurnCCW);
                    PyResult::Ok(())
                }
            });
            add_function!("move_forward", {
                let reply_channel = Rc::downgrade(&reply_channel);
                let command_channel = Rc::downgrade(&command_channel);
                move |vm: &VirtualMachine| -> PyResult<()> {
                    println!("TEST: move_forward");
                    let ret =
                        send_command!(vm, command_channel, reply_channel, PlayerCommand::MoveFwd);
                    PyResult::Ok(())
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
