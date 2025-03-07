use std::cmp::Eq;
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;
use std::mem;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::{self, Duration, Instant};
use std::{cell::RefCell, rc::Rc, sync::mpsc};

use crate::battle_state_info::BattleStateInfo;

use super::battle_logic::BattleLogic;
use super::command_and_reply::CommandReplyStat;
use super::gametime::GameTime;
use super::log_data::{LogRepresentable, LogWriter, MaybeLogRepresentable};

use super::player_state::PlayerControl;
use super::script_repr::ToScriptRepr;

use rand::distributions::Uniform;
use rand::prelude::*;

use rustpython_vm::Settings;
use rustpython_vm::{
    compiler,
    signal::{user_signal_channel, UserSignalReceiver},
    Interpreter, PyResult,
};

#[derive(Clone, Copy, PartialEq)]
pub enum PlayerCommandState<PC, PR> {
    None,
    GotCommandQueued(PC, bool, GameTime),
    // command, need_to_reply, time_at, duration, reply_duration, command_id or not if not yet logged
    GotCommand(PC, bool, GameTime, GameTime, GameTime, usize),
    GotCommandReply(PC, PR, bool, GameTime, GameTime, usize),
    Finish,
}

#[derive(Clone, Copy, Debug)]
pub struct GameEventItem<GameEvent> {
    time: GameTime,
    event: GameEvent,
}

// note: all comparisons are ONLY based on time, this is for the heap
impl<GameEvent> PartialEq for GameEventItem<GameEvent> {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl<GameEvent> Eq for GameEventItem<GameEvent> {}

impl<GameEvent> PartialOrd for GameEventItem<GameEvent> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<GameEvent> Ord for GameEventItem<GameEvent> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // reverse comparison to make max-heap into a min-heap
        // so now event item comparison sorta shows their relative priority
        other.time.cmp(&self.time)
    }
}

impl<PC, PR> PlayerCommandState<PC, PR> {
    pub fn take(&mut self) -> PlayerCommandState<PC, PR> {
        mem::replace(self, PlayerCommandState::None)
    }

    pub fn unwrap(self) -> (PC, bool, GameTime, GameTime, GameTime, usize) {
        if let PlayerCommandState::GotCommand(c, nr, t, dur, rep_dur, c_id) = self {
            (c, nr, t, dur, rep_dur, c_id)
        } else {
            panic!("unwrap failed!");
        }
    }
}

pub struct Battle<P, BLogic, PCom, PComRep, GameEvent, LW>
where
    P: PlayerControl + LogRepresentable,
    PCom: MaybeLogRepresentable,
    BLogic: BattleLogic<P, PCom, PComRep, GameEvent, String, String>,
    LW: LogWriter<String, String>,
{
    player_states: Vec<P>,
    player_programs: Vec<String>,
    battle_logic: BLogic,
    time: GameTime,
    log_writer: LW,
    player_death_logged: Vec<bool>,
    next_command_id: usize, // each player command will get a unique id for logging
    _marker: PhantomData<(PCom, PComRep, GameEvent)>,
}

pub const DEFAULT_COMMAND_DURATION: GameTime = 10;
pub const VM_THINK_TIMEOUT: time::Duration = time::Duration::from_secs(5);

impl<P, BLogic, PCom, PComRep, GameEvent, LW> Battle<P, BLogic, PCom, PComRep, GameEvent, LW>
where
    P: PlayerControl + ToScriptRepr + LogRepresentable,
    BLogic: BattleLogic<P, PCom, PComRep, GameEvent, String, String>,
    PCom: MaybeLogRepresentable + Hash + Clone + PartialEq + Eq + Send + 'static,
    PComRep: CommandReplyStat + Clone + PartialEq + Send + 'static,
    LW: LogWriter<String, String>,
    GameEvent: Clone + PartialEq,
{
    pub fn new(
        battle_logic: BLogic,
        player_initial_states_and_programs: Vec<(P, String)>,
        log_writer: LW,
    ) -> Battle<P, BLogic, PCom, PComRep, GameEvent, LW> {
        let (player_states, player_programs): (Vec<P>, Vec<String>) =
            player_initial_states_and_programs.into_iter().unzip();
        Battle {
            player_death_logged: vec![false; player_states.len()],
            player_states,
            player_programs,
            battle_logic,
            log_writer,
            time: 0,
            next_command_id: 0,
            _marker: PhantomData,
        }
    }

    pub fn time(&self) -> u64 {
        self.time
    }

    pub fn player_state(&self, i: usize) -> &P {
        &self.player_states[i]
    }

    pub fn is_player_dead(&self, i: usize) -> bool {
        self.battle_logic.is_player_dead(&self.player_states[i])
    }

    pub fn log_writer(&self) -> &LW {
        &self.log_writer
    }

    /// returns indices of winner players
    /// if sim ended in an error - there are no winners, so None is returned
    pub fn run_simulation(&mut self) -> Option<Vec<usize>> {
        self.run_simulation_with_time_limit(None)
    }

    pub fn run_simulation_with_time_limit(
        &mut self,
        game_time_limit: Option<GameTime>,
    ) -> Option<Vec<usize>> {
        self.time = 0;
        let player_count = self.player_programs.len();
        let mut winner_ids = None;

        thread::scope(|scope| {
            let mut handles = Vec::with_capacity(player_count);
            let mut channels = Vec::with_capacity(player_count);
            let mut thread_stop_signal_senders = Vec::with_capacity(player_count);
            let mut player_extra_commands_queues = vec![VecDeque::new(); player_count];
            let mut thread_ready_chans = Vec::with_capacity(player_count);

            for program in self.player_programs.iter() {
                let (command_sender, command_receiver) = mpsc::channel();
                let (result_sender, result_receiver) = mpsc::channel();
                let (thread_stop_sender, thread_stop_receiver) = user_signal_channel();
                let (thead_ready_tx, thread_ready_rx) = mpsc::channel();

                let handle = scope.spawn({
                    let program = program.clone();
                    let mut program_hasher = DefaultHasher::new();
                    program_hasher.write(program.as_bytes());
                    let program_hash = program_hasher.finish();
                    move || {
                        Self::program_runner(
                            program,
                            command_sender,
                            result_receiver,
                            thread_stop_receiver,
                            thead_ready_tx,
                            program_hash, //88284664
                        )
                    }
                });
                handles.push(Some(handle));
                channels.push(Some((command_receiver, result_sender)));
                thread_stop_signal_senders.push(Some(thread_stop_sender));
                thread_ready_chans.push(thread_ready_rx);
            }
            // wait for all threads to initialize
            for thread_ready_rx in thread_ready_chans {
                if let Err(_) = thread_ready_rx.recv() {
                    panic!("failed to initialize player thread, PANIK !!");
                }
            }

            let mut next_commands: Vec<PlayerCommandState<PCom, PComRep>> =
                vec![PlayerCommandState::None; player_count];

            let mut next_events_queue: BinaryHeap<GameEventItem<GameEvent>> = BinaryHeap::new();

            // initial logic setup
            self.battle_logic
                .initial_setup(&mut self.player_states, &mut |obj, act| {
                    self.log_writer.add_log_data(obj, act, self.time, 0);
                });

            //
            //
            // main game loop
            let mut start_timestamps = vec![time::Instant::now(); player_count];
            loop {
                let mut players_that_have_commands = 0;

                // check dead
                for (i, (player, death_logged)) in self
                    .player_states
                    .iter()
                    .zip(self.player_death_logged.iter_mut())
                    .enumerate()
                {
                    if self.battle_logic.is_player_dead(player) && !*death_logged {
                        // TODO: remove is_dead from player - game logic is responsible for that info
                        next_commands[i] = PlayerCommandState::Finish;
                        self.log_writer.add_log_data(
                            self.player_states[i].log_repr(),
                            "die".to_owned(),
                            self.time,
                            0,
                        );
                        *death_logged = true;
                    }
                }

                // check if game ended
                if let None = winner_ids {
                    if let Some(winners) = self.battle_logic.game_finished(&self.player_states) {
                        winner_ids = Some(winners);
                        // log victory
                        for winner_id in winner_ids.as_ref().unwrap() {
                            self.log_writer.add_log_data(
                                self.player_states[*winner_id].log_repr(),
                                "win".to_owned(),
                                self.time,
                                0,
                            );
                        }
                    } else if let Some(time_limit) = game_time_limit {
                        if self.time >= time_limit {
                            winner_ids = Some(Vec::new());
                        }
                    }
                }
                // if game is ended - we allow pending commands to finalize and enforce Finish state
                if let Some(_) = winner_ids {
                    for next_command in next_commands.iter_mut() {
                        // just finalize all players
                        // this will force their stopping and loop safe exit
                        if let PlayerCommandState::None = next_command {
                            *next_command = PlayerCommandState::Finish;
                        }
                        // we should let pending commands finalize
                    }
                }

                for (i, (channels_maybe, extra_commands_queue)) in channels
                    .iter()
                    .zip(player_extra_commands_queues.iter_mut())
                    .enumerate()
                {
                    let (command_receiver, _) = if let Some(x) = channels_maybe {
                        x
                    } else {
                        next_commands[i] = PlayerCommandState::Finish;
                        players_that_have_commands += 1;
                        continue;
                    };
                    // ignore finished ones and set ones. If their channels are not closed yet - without this check they might get a new command.
                    if let PlayerCommandState::None = next_commands[i] {
                    } else {
                        // if next comman is anything BUT None
                        players_that_have_commands += 1;
                        continue;
                    }

                    // first check if there are extra commands queues
                    if extra_commands_queue.len() > 0 {
                        let com = extra_commands_queue.pop_front().unwrap();
                        next_commands[i] =
                            PlayerCommandState::GotCommandQueued(com, false, self.time);
                        players_that_have_commands += 1;
                        continue;
                    }
                    // then get new command from the program
                    match command_receiver.try_recv() {
                        Ok(com) => {
                            next_commands[i] =
                                PlayerCommandState::GotCommandQueued(com, true, self.time);
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
                    thread::sleep(Duration::from_micros(1));
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
                    // first check if all done(stopped/dead) -> meaning game ends
                    if next_commands
                        .iter()
                        .all(|x| PlayerCommandState::Finish == *x)
                    {
                        // If there is no winners set at this point - it is a DRAW
                        if let None = winner_ids {
                            winner_ids = Some(Vec::new());
                        }
                        break;
                    }

                    // LOG command start and ASSIGN COMMAND IDs
                    // we do it separately to ensure consistent ordering
                    for (player_i, (command, player_state)) in next_commands
                        .iter_mut()
                        .zip(self.player_states.iter())
                        .enumerate()
                    {
                        // note - operation logged here MAY not complete, depending on concrete game logic
                        if let PlayerCommandState::GotCommandQueued(_, _, _) = command {
                            // this is an ugly way of moving only on match
                            // TODO: if you find a nicer way of doing this - change accordingly
                            let taken_command = command.take();
                            if let PlayerCommandState::GotCommandQueued(com, need_to_reply, time) =
                                taken_command
                            {
                                let command_id = self.next_command_id;
                                self.next_command_id += 1;

                                let duration =
                                    self.battle_logic.get_command_duration(player_state, &com);
                                let reply_delay_duration = self
                                    .battle_logic
                                    .get_command_reply_delay(player_state, &com);

                                if let Some(log_repr) = com.try_log_repr() {
                                    self.log_writer.add_log_data(
                                        player_state.log_repr(),
                                        format!("-{}({})", log_repr, command_id),
                                        time,
                                        duration + reply_delay_duration, // log full command time
                                    );
                                }

                                // call logic's pre-process
                                // for consistency we call pre-process just after logging, and post-process just before logging
                                let battle_info = BattleStateInfo::new(self.time);
                                self.battle_logic.command_received(
                                    player_i,
                                    &com,
                                    command_id,
                                    &self.player_states,
                                    &battle_info,
                                    &mut |obj, act| {
                                        self.log_writer.add_log_data(obj, act, self.time, 0);
                                    },
                                );

                                *command = PlayerCommandState::GotCommand(
                                    com,
                                    need_to_reply,
                                    time,
                                    duration,
                                    reply_delay_duration,
                                    command_id,
                                );
                            } else {
                                unreachable!();
                            }
                        }
                    }
                    // at this point all GetCommandQueued are changed to GetCommand

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
                            let (command_start_gametime, duration) =
                                if let PlayerCommandState::GotCommand(_, _, y, dur, _, _)
                                | PlayerCommandState::GotCommandReply(_, _, _, y, dur, _) = k
                                {
                                    (y, *dur)
                                } else {
                                    // filter above must have filtered Finish out, and None must not happen cuz of prev logic
                                    unreachable!();
                                };
                            //
                            (*command_start_gametime + duration - self.time, player_i, k)
                        })
                        .min_by_key(|k| k.0)
                    {
                        // either system event goes first, or user
                        if next_events_queue.len() > 0
                            && next_events_queue.peek().unwrap().time
                                < self.time + remaining_duration
                        {
                            // processing game events
                            let next_event = next_events_queue.pop().unwrap(); // we checked len in if condition
                                                                               // advance time first!
                            self.time = next_event.time;
                            let battle_state = BattleStateInfo::new(self.time);

                            if let Some(extra_events) = self.battle_logic.process_events(
                                &next_event.event,
                                &mut self.player_states,
                                &battle_state,
                                &mut |obj, act| {
                                    self.log_writer.add_log_data(obj, act, self.time, 0);
                                },
                            ) {
                                next_events_queue.extend(extra_events.into_iter().map(
                                    |(time_delta, event)| GameEventItem {
                                        time: self.time + time_delta,
                                        event,
                                    },
                                ));
                            };
                        } else {
                            // processing player commands
                            // advance time first!
                            self.time += remaining_duration;

                            // process the next command
                            match next_command.take() {
                                PlayerCommandState::GotCommand(
                                    com,
                                    need_to_reply,
                                    _,
                                    _,
                                    reply_delay_duration,
                                    command_id,
                                ) => {
                                    let battle_state = BattleStateInfo::new(self.time);

                                    let (reply, extra_actions_maybe, extra_events_maybe) =
                                        self.battle_logic.process_commands(
                                            player_i,
                                            &com,
                                            command_id,
                                            &mut self.player_states,
                                            &battle_state,
                                            &mut |obj, act| {
                                                self.log_writer
                                                    .add_log_data(obj, act, self.time, 0);
                                            },
                                        );
                                    if let Some(extra_actions) = extra_actions_maybe {
                                        player_extra_commands_queues[player_i]
                                            .extend(extra_actions);
                                    }
                                    if let Some(extra_events) = extra_events_maybe {
                                        next_events_queue.extend(extra_events.into_iter().map(
                                            |(time_delta, event)| GameEventItem {
                                                time: self.time + time_delta,
                                                event,
                                            },
                                        ));
                                    }

                                    *next_command = PlayerCommandState::GotCommandReply(
                                        com,
                                        reply,
                                        need_to_reply,
                                        self.time,
                                        reply_delay_duration,
                                        command_id,
                                    )
                                }
                                PlayerCommandState::GotCommandReply(
                                    com,
                                    reply,
                                    need_to_reply,
                                    _,
                                    _,
                                    command_id,
                                ) => {
                                    // ONLY Finished command may have None for reply channel by design
                                    //  also we bravely unwrap cuz channels may close only in the start of the loop

                                    // first - call post-processing logic
                                    // we call reply_delivered just before actually delivering it
                                    // for consistency we call pre-process just after logging, and post-process just before logging
                                    let battle_info = BattleStateInfo::new(self.time);
                                    self.battle_logic.command_reply_delivered(
                                        player_i,
                                        &com,
                                        command_id,
                                        &self.player_states,
                                        &battle_info,
                                        &mut |obj, act| {
                                            self.log_writer.add_log_data(obj, act, self.time, 0);
                                        },
                                    );
                                    //
                                    
                                    // now the actual replying
                                    let reply_channel = &channels[player_i].as_ref().unwrap().1;

                                    let command_succeeded = reply.command_succeeded();
                                    // send reply
                                    if need_to_reply {
                                        if let Err(_) = reply_channel.send(reply) {
                                            println!("failed to send reply to the player");
                                            // consider player broken
                                            *next_command = PlayerCommandState::Finish;
                                            continue;
                                        }
                                    }

                                    start_timestamps[player_i] = Instant::now(); // update timeout counter

                                    // log operation finish
                                    if let Some(log_repr) = com.try_log_repr() {
                                        self.log_writer.add_log_data(
                                            self.player_states[player_i].log_repr(),
                                            format!(
                                                "{}{}({})",
                                                if command_succeeded { "+" } else { "!" },
                                                log_repr,
                                                command_id, // at this point command_id is guaranteed not to be None
                                            ),
                                            self.time,
                                            0,
                                        );
                                    }
                                }
                                _ => unreachable!(),
                            }
                        }
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
                    Ok(Err(e)) => {
                        println!("program errored out: {}", e);
                    }
                    Err(_) => {
                        println!("something went funky with the thread!");
                    }
                }
            }
        });
        winner_ids
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
        thread_ready_signal: mpsc::Sender<()>,
        seed: u64,
    ) -> Result<(), String> {
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

        let mut vm_settings: Settings = Default::default();
        vm_settings.install_signal_handlers = false;
        let interpreter = Interpreter::with_init(vm_settings, |vm| {
            vm.set_user_signal_channel(vm_signal_receiver);
        });
        let ret = interpreter.enter(|vm| {
            let scope = vm.new_scope_with_builtins();

            // add some logic-independent functions
            scope
                .globals
                .set_item(
                    "rand",
                    vm.new_function("rand", {
                        let rng = Rc::new(RefCell::new(StdRng::seed_from_u64(seed)));
                        let uniform = Uniform::new(0 as f64, 1 as f64);
                        move || -> PyResult<f64> { PyResult::Ok(rng.borrow_mut().sample(uniform)) }
                    })
                    .into(),
                    vm,
                )
                .unwrap();

            BLogic::initialize_scope(vm, &scope, {
                let reply_channel = Rc::downgrade(&reply_channel);
                let command_channel = Rc::downgrade(&command_channel);
                move |com: PCom| -> Result<PComRep, ()> {
                    send_command!(vm, command_channel, reply_channel, com)
                }
            });

            //
            // ready to run player code
            thread_ready_signal.send(()).unwrap();
            drop(thread_ready_signal);

            let code_obj = match vm.compile(&program, compiler::Mode::Exec, "<embedded>".to_owned())
            {
                Ok(x) => x,
                Err(e) => {
                    return Err(e.to_string());
                }
            };

            // run player code
            if let PyResult::Err(e) = vm.run_code_obj(code_obj, scope) {
                let mut exc_str = String::new();
                vm.write_exception(&mut exc_str, &e).unwrap_or_else(|_| {
                    exc_str.push_str("unknown error");
                });
                return Err(exc_str);
            }

            Ok(())
        });
        interpreter.finalize(None);
        println!("program runner completed");
        ret
    }
}
