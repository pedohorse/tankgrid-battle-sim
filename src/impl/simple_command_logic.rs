use crate::command_logic::BattleLogic;
use crate::gametime::GameTime;
use crate::map::MapReadAccess;
use crate::map_object::MapObject;
use crate::map_prober::MapProber;
use crate::maptile_logic::MaptileLogic;
use crate::object_layer::ObjectLayer;
use crate::player_state::PlayerControl;
use crate::script_repr::{FromScriptRepr, ToScriptRepr};
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::Rc;

use rustpython_vm::convert::ToPyObject;
use rustpython_vm::scope::Scope;
use rustpython_vm::{PyResult, VirtualMachine};

pub const DEFAULT_COMMAND_DURATION: GameTime = 10;

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
pub enum PlayerCommandReply {
    None,
    Bool(bool),
    LookResult(Vec<(String, Option<String>)>),
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

pub struct SimpleBattleLogic<T, M, L, Pr, R, OLayer>
where
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    Pr: MapProber<T, R, M, L, ObjectCacheRepr<R>, OLayer>,
    R: Copy,
    OLayer: ObjectLayer<R, ObjectCacheRepr<R>>,
{
    map: M,
    logic: L,
    map_prober: Pr,
    command_durations: HashMap<PlayerCommand<R>, usize>,
    object_layer: OLayer,
    _marker0: PhantomData<R>,
    _marker1: PhantomData<T>,
}

impl<T, M, L, R, P, Pr, OLayer>
    BattleLogic<T, M, L, R, P, Pr, ObjectCacheRepr<R>, OLayer, PlayerCommand<R>, PlayerCommandReply>
    for SimpleBattleLogic<T, M, L, Pr, R, OLayer>
where
    T: Copy + Clone + Send + ToScriptRepr,
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    R: Copy + Clone + Eq + Hash + Send + 'static + FromScriptRepr,
    OLayer: ObjectLayer<R, ObjectCacheRepr<R>>,
    P: PlayerControl<R, M, T, L, ObjectCacheRepr<R>, OLayer> + MapObject<R> + ToScriptRepr,
    Pr: MapProber<T, R, M, L, ObjectCacheRepr<R>, OLayer>,
{
    fn process_commands(
        &mut self,
        player_i: usize,
        com: PlayerCommand<R>,
        player_states: &mut [P],
    ) -> PlayerCommandReply {
        match com {
            PlayerCommand::MoveFwd => {
                self.recreate_objects_layer(player_states);
                let player_state = &mut player_states[player_i];
                player_state.move_forward(&mut self.map, &self.logic, &self.object_layer);
                // TODO: interact with the object if moved onto one
                PlayerCommandReply::None
            }
            PlayerCommand::TurnCW => {
                self.recreate_objects_layer(player_states);
                let player_state = &mut player_states[player_i];
                player_state.turn_cw(&mut self.map, &self.logic, &self.object_layer);
                PlayerCommandReply::None
            }
            PlayerCommand::TurnCCW => {
                self.recreate_objects_layer(player_states);
                let player_state = &mut player_states[player_i];
                player_state.turn_ccw(&mut self.map, &self.logic, &self.object_layer);
                PlayerCommandReply::None
            }
            PlayerCommand::Shoot => {
                let player_state = &mut player_states[player_i];
                if player_state.resource_value(1) > 0 {
                    player_state.expend_resource(1, 1);
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
                        for obj_ref in self.object_layer.objects_at(hit_x, hit_y).into_iter() {
                            if !obj_ref.shootable() {
                                continue;
                            }
                            match obj_ref.obj_type {
                                ObjectCacheType::Player(other_player_i) => {
                                    let hit_enemy = &mut player_states[other_player_i];
                                    hit_enemy.expend_resource(0, 1);
                                }
                            }
                        }
                    };
                }

                PlayerCommandReply::None
            }
            PlayerCommand::Wait => PlayerCommandReply::None,
            PlayerCommand::Look(ori) => {
                self.recreate_objects_layer(player_states);
                let look_result = self.map_prober
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
                            maybe_obj.map(|obj| obj.to_script_repr()),
                        )
                    })
                    .collect();

                PlayerCommandReply::LookResult(look_result)
            }
            PlayerCommand::Finish => {
                unreachable!();
            }
        }
    }

    fn get_command_duration(&self, player_state: &P, com: &PlayerCommand<R>) -> GameTime {
        if let Some(dur) = self.command_durations.get(com) {
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

            (*dur * 100) / (speed_percentage as usize)
        } else {
            DEFAULT_COMMAND_DURATION
        }
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

impl<T, M, L, Pr, R, OLayer> SimpleBattleLogic<T, M, L, Pr, R, OLayer>
where
    T: Copy + Clone + Send + ToScriptRepr,
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    Pr: MapProber<T, R, M, L, ObjectCacheRepr<R>, OLayer>,
    R: Copy + Clone + Eq + Hash + Send + 'static + FromScriptRepr,
    OLayer: ObjectLayer<R, ObjectCacheRepr<R>>,
{
    pub fn new(
        map: M,
        logic: L,
        map_prober: Pr,
        object_layer: OLayer,
        command_durations: HashMap<PlayerCommand<R>, GameTime>,
    ) -> SimpleBattleLogic<T, M, L, Pr, R, OLayer> {
        SimpleBattleLogic {
            map,
            logic,
            map_prober,
            object_layer,
            command_durations,
            _marker0: PhantomData,
            _marker1: PhantomData,
        }
    }

    fn recreate_objects_layer<P>(&mut self, player_states: &[P])
    where
        P: PlayerControl<R, M, T, L, ObjectCacheRepr<R>, OLayer> + MapObject<R> + ToScriptRepr,
        
    {
        self.object_layer.clear();
        for (i, player) in player_states.iter().enumerate() {
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
