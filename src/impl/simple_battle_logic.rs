use crate::battle_logic::BattleLogic;
use crate::command_and_reply::CommandReplyStat;
use crate::gametime::GameTime;
use crate::log_data::LogRepresentable;
use crate::map::MapReadAccess;
use crate::map_object::MapObject;
use crate::map_prober::MapProber;
use crate::maptile_logic::MaptileLogic;
use crate::object_layer::ObjectLayer;
use crate::orientation::SimpleOrientation;
use crate::player_state::PlayerControl;
use crate::script_repr::{FromScriptRepr, ToScriptRepr};

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use rustpython_vm::convert::ToPyObject;
use rustpython_vm::scope::Scope;
use rustpython_vm::{PyResult, VirtualMachine};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerCommand<R> {
    MoveFwd,
    TurnCW,
    TurnCCW,
    Shoot,
    Wait,
    Look(R),
    AddAmmo(usize),   // generated after picking up ammo crate
    AddHealth(usize), // generated after picking up health
}

impl<R> LogRepresentable for PlayerCommand<R>
where
    R: LogRepresentable,
{
    fn log_repr(&self) -> String {
        match self {
            PlayerCommand::MoveFwd => "move-forward".to_owned(),
            PlayerCommand::TurnCW => "turn-cw".to_owned(),
            PlayerCommand::TurnCCW => "turn-ccw".to_owned(),
            PlayerCommand::Shoot => "shoot".to_owned(),
            PlayerCommand::Wait => "wait".to_owned(),
            PlayerCommand::Look(dir) => format!("look[{}]", dir.log_repr()),
            PlayerCommand::AddAmmo(ammo) => format!("add-ammo[{}]", ammo),
            PlayerCommand::AddHealth(health) => format!("heal[{}]", health),
        }
    }
}

#[derive(Clone)]
pub enum PlayerCommandReply {
    Failed,
    Ok,
    Bool(bool),
    LookResult(Vec<(String, Option<String>)>),
}

impl CommandReplyStat for PlayerCommandReply {
    fn command_succeeded(&self) -> bool {
        if let PlayerCommandReply::Failed = self {
            false
        } else {
            true
        }
    }
}

pub enum ObjectCacheType {
    Player(usize),
    AmmoCrate(usize),
    //Stuff, // TODO: add stuff like pickable items
}

impl LogRepresentable for ObjectCacheType {
    fn log_repr(&self) -> String {
        match self {
            ObjectCacheType::Player(_) => "player",
            ObjectCacheType::AmmoCrate(_) => "ammocrate",
        }
        .to_owned()
    }
}

pub trait CommandTimer<PC> {
    fn get_base_duration(&self, command: &PC) -> GameTime;
}

pub struct ObjectCacheRepr<R> {
    uid: u64,
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
    fn unique_id(&self) -> u64 {
        self.uid
    }

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

impl<R> LogRepresentable for ObjectCacheRepr<R> {
    fn log_repr(&self) -> String {
        format!("obj[{}]({})", self.obj_type.log_repr(), self.uid)
    }
}

pub struct SimpleBattleLogic<T, M, L, Pr, R, OLayer, Fdur>
where
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    Pr: MapProber<T, R, M, L, ObjectCacheRepr<R>, OLayer>,
    R: Copy,
    OLayer: ObjectLayer<R, ObjectCacheRepr<R>>,
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

const HEALTH_RES: usize = 0;
const AMMO_RES: usize = 1;

impl<T, M, L, R, P, Pr, OLayer, Fdur>
    BattleLogic<P, PlayerCommand<R>, PlayerCommandReply, String, String>
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
        + LogRepresentable,
    OLayer: ObjectLayer<R, ObjectCacheRepr<R>>,
    P: PlayerControl + MapObject<R> + ToScriptRepr + LogRepresentable,
    Pr: MapProber<T, R, M, L, ObjectCacheRepr<R>, OLayer>,
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
    }

    fn process_commands<LWF>(
        &mut self,
        player_i: usize,
        com: PlayerCommand<R>,
        player_states: &mut [P],
        logger: &mut LWF,
    ) -> (PlayerCommandReply, Option<Vec<PlayerCommand<R>>>)
    where
        LWF: FnMut(String, String),
    {
        match com {
            PlayerCommand::MoveFwd => {
                self.recreate_objects_layer(player_states);
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
                    logger(player_state.log_repr(), format!("move[{},{}]", fwd_pos_x, fwd_pos_y));

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
                self.recreate_objects_layer(player_states);
                let player_state = &mut player_states[player_i];
                player_state.turn_cw();
                logger(player_state.log_repr(), format!("turn[{}]", player_state.orientation().log_repr()));
                (PlayerCommandReply::Ok, None)
            }
            PlayerCommand::TurnCCW => {
                self.recreate_objects_layer(player_states);
                let player_state = &mut player_states[player_i];
                player_state.turn_ccw();
                logger(player_state.log_repr(), format!("turn[{}]", player_state.orientation().log_repr()));
                (PlayerCommandReply::Ok, None)
            }
            PlayerCommand::Shoot => {
                let player_state = &mut player_states[player_i];
                if player_state.resource_value(AMMO_RES) > 0 {
                    player_state.expend_resource(AMMO_RES, 1);
                    self.recreate_objects_layer(player_states);
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
                            logger(player_state.log_repr(), format!("shoot[{x},{y},{hit_x},{hit_y}]"));
                        }
                        let mut objs_to_destroy = Vec::new();
                        for obj in self.object_layer.objects_at(hit_x, hit_y).into_iter() {
                            if !obj.shootable() {
                                continue;
                            }
                            match obj.obj_type {
                                ObjectCacheType::Player(other_player_i) => {
                                    let hit_enemy = &mut player_states[other_player_i];
                                    hit_enemy.expend_resource(HEALTH_RES, 1);
                                }
                                ObjectCacheType::AmmoCrate(_) => {
                                    objs_to_destroy.push(obj.unique_id());
                                }
                            }
                        }
                        for obj_id in objs_to_destroy {
                            logger(self.object_layer.object_by_id(obj_id).unwrap().log_repr(), format!("break"));
                            self.object_layer.remove_object(obj_id);
                        }
                    };
                    (PlayerCommandReply::Ok, None)
                } else {
                    (PlayerCommandReply::Failed, None)
                }
            }
            PlayerCommand::Wait => (PlayerCommandReply::Ok, None),
            PlayerCommand::Look(ori) => {
                // note: look command's ori is relative to tank orientation
                // so we need to convert it to global orientation
                let ori = ori.from_relative_to_global(&player_states[player_i].orientation());
                self.recreate_objects_layer(player_states);
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
                                } else {
                                    "side"
                                };
                                format!("{}({})", obj.to_script_repr(), obj_ori)
                            }),
                        )
                    })
                    .collect();

                (PlayerCommandReply::LookResult(look_result), None)
            }
            PlayerCommand::AddAmmo(ammo) => {
                let player_state = &mut player_states[player_i];
                player_state.gain_resource(AMMO_RES, ammo);
                (PlayerCommandReply::Ok, None)
            }
            PlayerCommand::AddHealth(health) => {
                let player_state = &mut player_states[player_i];
                player_state.gain_resource(HEALTH_RES, health);
                (PlayerCommandReply::Ok, None)
            }
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

        (dur * 100) / (speed_percentage as usize)
    }

    fn initialize_scope<FSR>(vm: &VirtualMachine, scope: &Scope, comm_chan: FSR)
    where
        FSR: Fn(PlayerCommand<R>) -> Result<PlayerCommandReply, ()> + Clone + 'static,
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
            move |vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: turn_cw");
                let _ret = comm_chan(PlayerCommand::TurnCW);
                PyResult::Ok(())
            }
        });
        add_function!("turn_ccw", {
            let comm_chan = comm_chan.clone();
            move |vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: turn_ccw");
                let _ret = comm_chan(PlayerCommand::TurnCCW);
                PyResult::Ok(())
            }
        });
        add_function!("move_forward", {
            let comm_chan = comm_chan.clone();
            move |vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: move_forward");
                let _ret = comm_chan(PlayerCommand::MoveFwd);
                PyResult::Ok(())
            }
        });
        add_function!("shoot", {
            let comm_chan = comm_chan.clone();
            move |vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: shoot");
                let _ret = comm_chan(PlayerCommand::Shoot);
                PyResult::Ok(())
            }
        });
        add_function!("wait", {
            let comm_chan = comm_chan.clone();
            move |vm: &VirtualMachine| -> PyResult<()> {
                println!("TEST: wait");
                let _ret = comm_chan(PlayerCommand::Wait);
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
                    PyResult::Err(vm.new_runtime_error("unexpected look reply".to_owned()))
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
    Pr: MapProber<T, R, M, L, ObjectCacheRepr<R>, OLayer>,
    R: Copy + Clone + Eq + Hash + Send + 'static + FromScriptRepr,
    OLayer: ObjectLayer<R, ObjectCacheRepr<R>>,
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

    fn recreate_objects_layer<P>(&mut self, player_states: &[P])
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
            self.object_layer.add(ObjectCacheRepr {
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
