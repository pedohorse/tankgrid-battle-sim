use crate::battle_logic::BattleLogic;
use crate::battle_state_info::BattleStateInfo;
use crate::command_and_reply::CommandReplyStat;
use crate::gametime::GameTime;
use crate::log_data::{LogRepresentable, MaybeLogRepresentable};
use crate::map::MapReadAccess;
use crate::map_object::MapObject;
use crate::map_prober::MapProber;
use crate::maptile_logic::MaptileLogic;
use crate::object_layer::ObjectLayer;
use crate::orientation::SimpleOrientation;
use crate::player_state::PlayerControl;
use crate::script_repr::{FromScriptRepr, ToScriptRepr};

use super::simple_object::{ObjectCacheType, SimpleObject};

use std::hash::Hash;
use std::marker::PhantomData;
use std::vec;

use rustpython_vm::convert::ToPyObject;
use rustpython_vm::function::FuncArgs;
use rustpython_vm::scope::Scope;
use rustpython_vm::{PyResult, VirtualMachine};

pub const MAX_LOG_LINE_LENGTH: usize = 160;
pub const MAX_FREE_PRINTS: u64 = 6; // 5 prints, 1 for warning

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum PlayerCommand<R> {
    MoveFwd,
    TurnCW,
    TurnCCW,
    Shoot,
    AfterShootCooldown,
    Wait,
    Print(String),
    CheckAmmo,
    CheckHealth,
    CheckHit, // checks from which side was last hit (hit info is reset after check)
    ResetHit, // forcefully ignore last hit info. supposed to be faster than CheckHit
    Look(R),
    Listen,
    AddAmmo(u64),   // generated after picking up ammo crate
    AddHealth(u64), // generated after picking up health
    Time,
}

impl<R> MaybeLogRepresentable for PlayerCommand<R>
where
    R: LogRepresentable,
{
    fn try_log_repr(&self) -> Option<String> {
        match self {
            PlayerCommand::MoveFwd => Some("move-forward".to_owned()),
            PlayerCommand::TurnCW => Some("turn-cw".to_owned()),
            PlayerCommand::TurnCCW => Some("turn-ccw".to_owned()),
            PlayerCommand::Shoot => Some("shoot".to_owned()),
            PlayerCommand::AfterShootCooldown => Some("cooldown".to_owned()),
            PlayerCommand::Wait => Some("wait".to_owned()),
            PlayerCommand::Look(dir) => Some(format!("look[{}]", dir.log_repr())),
            PlayerCommand::Listen => Some(format!("listen")),
            PlayerCommand::AddAmmo(ammo) => Some(format!("add-ammo[{}]", ammo)),
            PlayerCommand::AddHealth(health) => Some(format!("heal[{}]", health)),
            PlayerCommand::CheckAmmo => Some(format!("check-ammo")),
            PlayerCommand::CheckHealth => Some(format!("check-health")),
            PlayerCommand::CheckHit => Some(format!("check-hit")),
            PlayerCommand::ResetHit => Some(format!("reset-hit")),
            PlayerCommand::Print(_) => None,
            PlayerCommand::Time => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayerCommandReply<R> {
    Failed,
    Ok,
    Bool(bool),
    Int(i64),
    Uint(u64),
    HitDirection(Option<R>),
    LookResult(Vec<(String, Option<String>)>),
    ListenResult(Vec<String>),
}

impl<R> CommandReplyStat for PlayerCommandReply<R> {
    fn command_succeeded(&self) -> bool {
        if let PlayerCommandReply::Failed = self {
            false
        } else {
            true
        }
    }
}

pub trait CommandTimer<PC> {
    fn get_base_duration(&self, command: &PC) -> GameTime;

    fn get_reply_delay(&self, command: &PC) -> GameTime {
        let _ = command; // avoid unused warning
        0
    }
}

pub struct SimpleBattleLogic<T, M, L, Pr, R, OLayer, Fdur>
where
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    Pr: MapProber<T, R, M, L, SimpleObject<R>, OLayer>,
    R: Copy,
    OLayer: ObjectLayer<R, SimpleObject<R>>,
    Fdur: CommandTimer<PlayerCommand<R>>,
{
    map: M,
    logic: L,
    map_prober: Pr,
    command_duration: Fdur,
    object_layer: OLayer,
    player_count_to_win: usize,
    _marker0: PhantomData<R>,
    _marker1: PhantomData<T>,
}

pub const HEALTH_RES: usize = 0;
pub const AMMO_RES: usize = 1;
pub const HIT_DIR_RES: usize = 2;
pub const PRINT_COUNTER_RES: usize = 3;

impl<T, M, L, R, P, Pr, OLayer, Fdur>
    BattleLogic<P, PlayerCommand<R>, PlayerCommandReply<R>, String, String>
    for SimpleBattleLogic<T, M, L, Pr, R, OLayer, Fdur>
where
    T: Copy + Clone + Send + ToScriptRepr,
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    R: Copy
        + Clone
        + Eq
        + Hash
        + Send
        + 'static
        + SimpleOrientation
        + FromScriptRepr
        + ToScriptRepr
        + LogRepresentable
        + Into<u64>
        + From<u64>
        + std::fmt::Debug,
    OLayer: ObjectLayer<R, SimpleObject<R>>,
    P: PlayerControl + MapObject<R> + ToScriptRepr + LogRepresentable,
    Pr: MapProber<T, R, M, L, SimpleObject<R>, OLayer>,
    Fdur: CommandTimer<PlayerCommand<R>>,
{
    fn is_player_dead(&self, player: &P) -> bool {
        player.resource_value(HEALTH_RES) <= 0
    }

    fn game_finished(&self, players: &[P]) -> Option<Vec<usize>> {
        // default impl returns true when only single player left
        let maybe_winners: Vec<usize> = players
            .iter()
            .enumerate()
            .filter(|(_, p)| !self.is_player_dead(*p))
            .map(|(i, _)| i)
            .collect();
        if maybe_winners.len() <= self.player_count_to_win {
            Some(maybe_winners)
        } else {
            None
        }
    }

    fn initial_setup<LWF>(&mut self, player_states: &mut [P], logger: &mut LWF)
    where
        LWF: FnMut(String, String),
    {
        // log spawn
        for player in player_states.iter() {
            let (x, y) = player.position();
            let ori = player.orientation();
            logger(
                player.log_repr(),
                format!("spawn[{},{},{}]", x, y, ori.log_repr()),
            );
        }
        // log initial objects and players
        for object in self.object_layer.objects() {
            if let ObjectCacheType::Player(_) = object.obj_type {
                // for now log players and other objects separately
                continue;
            }
            let (x, y) = object.pos;
            let ori = object.orientation();
            logger(
                object.log_repr(),
                format!("spawn[{},{},{}]", x, y, ori.log_repr()),
            );
        }
    }

    fn process_commands<LWF>(
        &mut self,
        player_i: usize,
        com: &PlayerCommand<R>,
        player_states: &mut [P],
        battle_state: &BattleStateInfo,
        logger: &mut LWF,
    ) -> (PlayerCommandReply<R>, Option<Vec<PlayerCommand<R>>>)
    where
        LWF: FnMut(String, String),
    {
        // doing any command other than print resets the print counter
        match com {
            PlayerCommand::Print(_) => (), // do nothing, we expend in the next match
            _ => {
                player_states[player_i].set_resource(PRINT_COUNTER_RES, MAX_FREE_PRINTS);
            }
        }

        match com {
            PlayerCommand::MoveFwd => {
                self.recache_players_to_object_layer(player_states);
                let player_state = &mut player_states[player_i];
                let mut extra_commands = None;

                let (fwd_pos_x, fwd_pos_y) = self
                    .map_prober
                    .step_in_direction(player_state.position(), player_state.orientation());
                let tile = self.map.get_tile_at(fwd_pos_x, fwd_pos_y);
                let reply = if self.logic.passable(tile)
                    && self
                        .object_layer
                        .objects_at_are_passable(fwd_pos_x, fwd_pos_y)
                {
                    player_state.move_to((fwd_pos_x, fwd_pos_y));
                    logger(
                        player_state.log_repr(),
                        format!("move[{},{}]", fwd_pos_x, fwd_pos_y),
                    );

                    // pick up pickable objects

                    let mut objs_to_destroy = Vec::new();
                    for obj in self.object_layer.objects_at(fwd_pos_x, fwd_pos_y) {
                        match obj.obj_type {
                            ObjectCacheType::AmmoCrate(ammo_size) => {
                                objs_to_destroy.push(obj.unique_id());
                                extra_commands = Some(vec![PlayerCommand::AddAmmo(ammo_size)]);
                            }
                            _ => (),
                        }
                    }
                    for obj_id in objs_to_destroy {
                        logger(
                            // unwrap cuz obj must exist as checked in prev loop
                            self.object_layer.object_by_id(obj_id).unwrap().log_repr(),
                            "picked".to_owned(),
                        );
                        self.object_layer.remove_object(obj_id);
                    }

                    PlayerCommandReply::Ok
                } else {
                    PlayerCommandReply::Failed
                };

                (reply, extra_commands)
            }
            PlayerCommand::TurnCW => {
                self.recache_players_to_object_layer(player_states);
                let player_state = &mut player_states[player_i];
                player_state.turn_cw();
                logger(
                    player_state.log_repr(),
                    format!("turn[{}]", player_state.orientation().log_repr()),
                );
                (PlayerCommandReply::Ok, None)
            }
            PlayerCommand::TurnCCW => {
                self.recache_players_to_object_layer(player_states);
                let player_state = &mut player_states[player_i];
                player_state.turn_ccw();
                logger(
                    player_state.log_repr(),
                    format!("turn[{}]", player_state.orientation().log_repr()),
                );
                (PlayerCommandReply::Ok, None)
            }
            PlayerCommand::Shoot => {
                let player_state = &mut player_states[player_i];
                if player_state.resource_value(AMMO_RES) > 0 {
                    player_state.expend_resource(AMMO_RES, 1);
                    self.recache_players_to_object_layer(player_states);
                    let player_state = &mut player_states[player_i];
                    if let Some((hit_x, hit_y)) = self.map_prober.raycast(
                        player_state.position(),
                        &self.map,
                        &self.logic,
                        &self.object_layer,
                        player_state.orientation(),
                        true,
                        false,
                        true,
                    ) {
                        {
                            let (x, y) = player_state.position();
                            logger(
                                player_state.log_repr(),
                                format!("shoot[{x},{y},{hit_x},{hit_y}]"),
                            );
                        }
                        let mut objs_to_destroy = Vec::new();
                        let player_ori = player_state.orientation();
                        for obj in self.object_layer.objects_at(hit_x, hit_y).into_iter() {
                            if !obj.shootable() {
                                continue;
                            }
                            match obj.obj_type {
                                ObjectCacheType::Player(other_player_i) => {
                                    let hit_enemy = &mut player_states[other_player_i];
                                    hit_enemy.expend_resource(HEALTH_RES, 1);
                                    let hit_relative_direction = player_ori
                                        .opposite()
                                        .global_to_relative_to(&hit_enemy.orientation());
                                    hit_enemy // 0 means no hit, we offset orient representation with 1 to not have overlap with 0
                                        .set_resource(
                                            HIT_DIR_RES,
                                            1 + hit_relative_direction.into(),
                                        );
                                }
                                ObjectCacheType::AmmoCrate(_) => {
                                    objs_to_destroy.push(obj.unique_id());
                                }
                            }
                        }
                        for obj_id in objs_to_destroy {
                            logger(
                                self.object_layer.object_by_id(obj_id).unwrap().log_repr(),
                                format!("break"),
                            );
                            self.object_layer.remove_object(obj_id);
                        }
                    };
                    (
                        PlayerCommandReply::Ok,
                        Some(vec![PlayerCommand::AfterShootCooldown]),
                    ) // some wait after shooting
                } else {
                    (PlayerCommandReply::Failed, None)
                }
            }
            PlayerCommand::AfterShootCooldown => (PlayerCommandReply::Ok, None),
            PlayerCommand::Wait => (PlayerCommandReply::Ok, None),
            PlayerCommand::Look(ori) => {
                // note: look command's ori is relative to tank orientation
                // so we need to convert it to global orientation
                let ori = ori.from_relative_to_global(&player_states[player_i].orientation());
                self.recache_players_to_object_layer(player_states);
                let look_result = self
                    .map_prober
                    .look(
                        player_states[player_i].position(),
                        &self.map,
                        &self.logic,
                        &self.object_layer,
                        ori,
                    )
                    .into_iter()
                    .map(|(t, maybe_obj)| {
                        (
                            t.to_script_repr(),
                            maybe_obj.map(|obj| {
                                let obj_ori = if obj.orientation().opposite_of(&ori) {
                                    "front"
                                } else if obj.orientation().same_as(&ori) {
                                    "back"
                                } else if obj.orientation().left_of(&ori) {
                                    "left-side"
                                } else if obj.orientation().right_of(&ori) {
                                    "right-side"
                                } else {
                                    "unknown"
                                };
                                format!("{}[{}]", obj.to_script_repr(), obj_ori)
                            }),
                        )
                    })
                    .collect();

                (PlayerCommandReply::LookResult(look_result), None)
            }
            PlayerCommand::Listen => {
                let mut res = Vec::with_capacity(player_states.len());
                let player_state = &player_states[player_i];
                let my_ori = player_state.orientation();
                let my_pos = player_state.position();
                for (i, enemy_state) in player_states.iter().enumerate() {
                    if i == player_i {
                        continue;
                    }
                    let (to_enemy1, to_enemy2_maybe) =
                        R::direction_to_closest_orientations(my_pos, enemy_state.position());
                    // fucky logic here is to properly assign values to border values
                    let location = if let Some(to_enemy2) = to_enemy2_maybe {
                        // meaning we don't have an exact orientation
                        // NOTE: this logic does not really work for strange, non-uniform and axis-assymetrical kinds of rotation groups!
                        let left = (to_enemy1.same_as(&my_ori) || to_enemy1.opposite_of(&my_ori))
                            && to_enemy2.left_of(&my_ori)
                            || !to_enemy1.opposite_of(&my_ori) && to_enemy1.left_of(&my_ori);
                        let front = to_enemy1.codirected_with(&my_ori)
                            || !to_enemy1.counterdirected_with(&my_ori)
                                && to_enemy2.codirected_with(&my_ori);
                        let thres = 45.0_f64.to_radians().cos();
                        let edot = to_enemy1.dot(&my_ori);
                        let closest_along = if edot > 0.0 {
                            edot > thres || to_enemy1.left_of(&my_ori) && edot == thres
                        } else {
                            edot < -thres || to_enemy1.right_of(&my_ori) && edot == thres
                        };
                        match (closest_along, front, left) {
                            (false, false, false) => "back-right-side",
                            (false, false, true) => "back-left-side",
                            (false, true, false) => "front-right-side",
                            (false, true, true) => "front-left-side",
                            (true, false, false) => "back-right-along",
                            (true, false, true) => "back-left-along",
                            (true, true, false) => "front-right-along",
                            (true, true, true) => "front-left-along",
                        }
                    } else {
                        // meaning we DO have an exact match
                        if to_enemy1.same_as(&my_ori)
                            || to_enemy1.codirected_with(&my_ori) && to_enemy1.right_of(&my_ori)
                        {
                            "front-right-along"
                        } else if to_enemy1.opposite_of(&my_ori)
                            || to_enemy1.counterdirected_with(&my_ori) && to_enemy1.left_of(&my_ori)
                        {
                            "back-left-along"
                        } else if to_enemy1.right_of(&my_ori) {
                            "back-right-side"
                        } else {
                            "front-left-side"
                        }
                    };

                    res.push(location.to_owned());
                }
                (PlayerCommandReply::ListenResult(res), None)
            }
            PlayerCommand::AddAmmo(ammo) => {
                let player_state = &mut player_states[player_i];
                player_state.gain_resource(AMMO_RES, *ammo);
                (PlayerCommandReply::Ok, None)
            }
            PlayerCommand::AddHealth(health) => {
                let player_state = &mut player_states[player_i];
                player_state.gain_resource(HEALTH_RES, *health);
                (PlayerCommandReply::Ok, None)
            }
            PlayerCommand::CheckAmmo => {
                let player_state = &player_states[player_i];
                let val = player_state.resource_value(AMMO_RES);
                (PlayerCommandReply::Int(val as i64), None)
            }
            PlayerCommand::CheckHealth => {
                let player_state = &player_states[player_i];
                let val = player_state.resource_value(HEALTH_RES);
                (PlayerCommandReply::Int(val as i64), None)
            }
            PlayerCommand::CheckHit => {
                let player_state = &mut player_states[player_i];
                let repr_res_value = player_state.resource_value(HIT_DIR_RES);
                let hit_direction = if repr_res_value == 0 {
                    None
                } else {
                    Some(R::from(repr_res_value - 1))
                };
                player_state.set_resource(HIT_DIR_RES, 0); // once read - last hit is set back to None
                (PlayerCommandReply::HitDirection(hit_direction), None)
            }
            PlayerCommand::ResetHit => {
                let player_state = &mut player_states[player_i];
                player_state.set_resource(HIT_DIR_RES, 0);
                (PlayerCommandReply::Ok, None)
            }
            PlayerCommand::Print(ref line) => {
                let player_state = &mut player_states[player_i];
                let mut penalty = None;
                match player_state.resource_value(PRINT_COUNTER_RES) {
                    0 => penalty = Some(vec![PlayerCommand::Wait; 4]), // penalize player for abusing print with forced waits
                    1 => logger(
                        player_state.log_repr(),
                        "log[---next print will be muted and penalized with game time unless a valid game comand called---]".to_owned(),
                    ),
                    _ => logger(player_state.log_repr(), format!("log[{}]", line)),
                }
                player_state.expend_resource(PRINT_COUNTER_RES, 1);
                (PlayerCommandReply::Ok, penalty)
            }
            PlayerCommand::Time => (PlayerCommandReply::Uint(battle_state.game_time), None),
        }
    }

    fn get_command_duration(&self, player_state: &P, com: &PlayerCommand<R>) -> GameTime {
        let dur = self.command_duration.get_base_duration(com);
        let tile = {
            let (x, y) = player_state.position();
            self.map.get_tile_at(x, y)
        };
        let speed_percentage = match com {
            PlayerCommand::MoveFwd => self.logic.pass_speed_percentage(tile),
            PlayerCommand::TurnCW | PlayerCommand::TurnCCW => {
                self.logic.turn_speed_percentage(tile)
            }
            _ => 100,
        };
        // speed = 0 means we misconfigured something
        let speed_percentage = if speed_percentage == 0 {
            eprintln!("[WARNING] tile speed == 0, seems like a misconfiguration, ignoring");
            100
        } else {
            speed_percentage
        };

        (dur * 100) / (speed_percentage as u64)
    }

    fn get_command_reply_delay(&self, _player_state: &P, com: &PlayerCommand<R>) -> GameTime {
        self.command_duration.get_reply_delay(com)
    }

    fn initialize_scope<FSR>(vm: &VirtualMachine, scope: &Scope, comm_chan: FSR)
    where
        FSR: Fn(PlayerCommand<R>) -> Result<PlayerCommandReply<R>, ()> + Clone + 'static,
    {
        macro_rules! add_function {
            ($fname:expr, $fn:block) => {
                scope
                    .globals
                    .set_item($fname, vm.new_function($fname, $fn).into(), vm)
                    .unwrap();
            };
        }

        add_function!("turn_cw", {
            // TODO: figure out why do I have to downgrade refs?
            //  it's as if interpreter is not dropped properly and keeps refs
            let comm_chan = comm_chan.clone();
            move |_vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: turn_cw");
                let _ret = comm_chan(PlayerCommand::TurnCW);
                PyResult::Ok(())
            }
        });
        add_function!("turn_ccw", {
            let comm_chan = comm_chan.clone();
            move |_vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: turn_ccw");
                let _ret = comm_chan(PlayerCommand::TurnCCW);
                PyResult::Ok(())
            }
        });
        add_function!("move_forward", {
            let comm_chan = comm_chan.clone();
            move |_vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: move_forward");
                let _ret = comm_chan(PlayerCommand::MoveFwd);
                PyResult::Ok(())
            }
        });
        add_function!("shoot", {
            let comm_chan = comm_chan.clone();
            move |_vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: shoot");
                let _ret = comm_chan(PlayerCommand::Shoot);
                PyResult::Ok(())
            }
        });
        add_function!("wait", {
            let comm_chan = comm_chan.clone();
            move |_vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: wait");
                let _ret = comm_chan(PlayerCommand::Wait);
                PyResult::Ok(())
            }
        });
        add_function!("print", {
            let comm_chan = comm_chan.clone();
            move |args: FuncArgs, vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: print");
                let line = args
                    .args
                    .into_iter()
                    .map(|arg| -> String {
                        arg.str(vm)
                            .map_or_else(|_| "<unprintable>".to_owned(), |s| s.as_str().to_owned())
                    })
                    .fold(None, |a, b| {
                        Some(if let Some(s) = a {
                            s + " " + b.as_str()
                        } else {
                            b
                        })
                    })
                    .unwrap_or_default();
                // sanitize string!

                let line: String = line
                    .chars()
                    .take(MAX_LOG_LINE_LENGTH)
                    .map(|c| if c.is_control() { '_' } else { c })
                    .collect();
                let _ret = comm_chan(PlayerCommand::Print(line));
                PyResult::Ok(())
            }
        });
        add_function!("look", {
            let comm_chan = comm_chan.clone();
            move |direction: String, vm: &VirtualMachine| -> PyResult<_> {
                println!("TEST: look");
                // note: look command is RELATIVE to tank orientation
                let direction = if let Some(x) = R::from_script_repr(&direction) {
                    x
                } else {
                    return PyResult::Err(vm.new_runtime_error("bad direction value".to_owned()));
                };
                let ret = if let Ok(x) = comm_chan(PlayerCommand::Look(direction)) {
                    x
                } else {
                    return PyResult::Err(vm.new_runtime_error("game closed".to_owned()));
                };
                if let PlayerCommandReply::LookResult(look_result) = ret {
                    PyResult::Ok(
                        look_result
                            .into_iter()
                            .map(|t| t.to_pyobject(&vm))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    PyResult::Err(vm.new_runtime_error(format!("unexpected look reply: {:?}", ret)))
                }
            }
        });
        add_function!("listen", {
            let comm_chan = comm_chan.clone();
            move |vm: &VirtualMachine| {
                println!("TEST: listen");
                let ret = if let Ok(x) = comm_chan(PlayerCommand::Listen) {
                    x
                } else {
                    return PyResult::Err(vm.new_runtime_error("game closed".to_owned()));
                };
                if let PlayerCommandReply::ListenResult(listen_result) = ret {
                    PyResult::Ok(
                        listen_result
                            .into_iter()
                            .map(|t| t.to_pyobject(&vm))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    PyResult::Err(
                        vm.new_runtime_error(format!("unexpected listen reply: {:?}", ret)),
                    )
                }
            }
        });
        add_function!("check_ammo", {
            let comm_chan = comm_chan.clone();
            move |vm: &VirtualMachine| {
                println!("TEST: check_ammo");
                let ret = if let Ok(x) = comm_chan(PlayerCommand::CheckAmmo) {
                    x
                } else {
                    return PyResult::Err(vm.new_runtime_error("game closed".to_owned()));
                };
                if let PlayerCommandReply::Int(val) = ret {
                    PyResult::Ok(val)
                } else {
                    PyResult::Err(
                        vm.new_runtime_error(format!("unexpected check result: {:?}", ret)),
                    )
                }
            }
        });
        add_function!("check_health", {
            let comm_chan = comm_chan.clone();
            move |vm: &VirtualMachine| {
                println!("TEST: check_health");
                let ret = if let Ok(x) = comm_chan(PlayerCommand::CheckHealth) {
                    x
                } else {
                    return PyResult::Err(vm.new_runtime_error("game closed".to_owned()));
                };
                if let PlayerCommandReply::Int(val) = ret {
                    PyResult::Ok(val)
                } else {
                    PyResult::Err(
                        vm.new_runtime_error(format!("unexpected check result: {:?}", ret)),
                    )
                }
            }
        });
        add_function!("check_hit", {
            let comm_chan = comm_chan.clone();
            move |vm: &VirtualMachine| {
                println!("TEST: check_hit");
                let ret = if let Ok(x) = comm_chan(PlayerCommand::CheckHit) {
                    x
                } else {
                    return PyResult::Err(vm.new_runtime_error("game closed".to_owned()));
                };
                if let PlayerCommandReply::HitDirection(ori) = ret {
                    PyResult::Ok(ori.map(|x| x.to_script_repr()))
                } else {
                    PyResult::Err(
                        vm.new_runtime_error(format!("unexpected check result: {:?}", ret)),
                    )
                }
            }
        });
        add_function!("reset_hit", {
            let comm_chan = comm_chan.clone();
            move |_vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: reset_hit");
                let _ret = comm_chan(PlayerCommand::ResetHit);
                PyResult::Ok(())
            }
        });
        add_function!("time", {
            let comm_chan = comm_chan.clone();
            move |vm: &VirtualMachine| {
                println!("TEST: test");
                let ret = if let Ok(x) = comm_chan(PlayerCommand::Time) {
                    x
                } else {
                    return PyResult::Err(vm.new_runtime_error("game closed".to_owned()));
                };
                if let PlayerCommandReply::Uint(time) = ret {
                    PyResult::Ok(time)
                } else {
                    PyResult::Err(
                        vm.new_runtime_error(format!("unexpected time result: {:?}", ret)),
                    )
                }
            }
        });
    }
}

impl<T, M, L, Pr, R, OLayer, Fdur> SimpleBattleLogic<T, M, L, Pr, R, OLayer, Fdur>
where
    T: Copy + Clone + Send + ToScriptRepr,
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    Pr: MapProber<T, R, M, L, SimpleObject<R>, OLayer>,
    R: Copy + Clone + Eq + Hash + Send + 'static + FromScriptRepr,
    OLayer: ObjectLayer<R, SimpleObject<R>>,
    Fdur: CommandTimer<PlayerCommand<R>>,
{
    pub fn new(
        map: M,
        logic: L,
        map_prober: Pr,
        object_layer: OLayer,
        command_duration: Fdur,
        player_count_to_win: usize,
    ) -> SimpleBattleLogic<T, M, L, Pr, R, OLayer, Fdur> {
        SimpleBattleLogic {
            map,
            logic,
            map_prober,
            object_layer,
            command_duration,
            player_count_to_win,
            _marker0: PhantomData,
            _marker1: PhantomData,
        }
    }

    fn recache_players_to_object_layer<P>(&mut self, player_states: &[P])
    where
        // TODO: player does NOT have to impl MapObject
        P: PlayerControl + MapObject<R> + ToScriptRepr,
    {
        // clear player cache
        self.object_layer.clear_by(|m| {
            if let ObjectCacheType::Player(_) = &m.obj_type {
                true
            } else {
                false
            }
        });
        // repopulate player cache
        for (i, player) in player_states.iter().enumerate() {
            if player.resource_value(HEALTH_RES) <= 0 {
                // if dead (TODO: may spawn a corpse object instead)
                continue;
            }
            self.object_layer.add(SimpleObject {
                uid: player.unique_id(),
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
