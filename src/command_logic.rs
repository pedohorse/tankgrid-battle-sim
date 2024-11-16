use crate::gametime::GameTime;

use super::map::MapReadAccess;
use super::map_object::MapObject;
use super::maptile_logic::MaptileLogic;
use super::object_layer::ObjectLayer;
use super::player_state::PlayerControl;
use super::map_prober::MapProber;
use super::script_repr::{FromScriptRepr, ToScriptRepr};
use std::hash::Hash;
use rustpython_vm::function::IntoPyNativeFn;
use rustpython_vm::scope::Scope;
use rustpython_vm::vm::VirtualMachine;

pub trait BattleLogic<T, M, L, R, P, Pr, MObj, OLayer, PCom, PComRep>
where
    MObj: MapObject<R>,
    L: MaptileLogic<T>,
    M: MapReadAccess<T>,
    OLayer: ObjectLayer<R, MObj>,
    P: PlayerControl<R, M, T, L, MObj, OLayer>,
{
    fn process_commands(
        &mut self,
        player_i: usize,
        com: PCom,
        player_states: &mut [P],
    ) -> PComRep;

    // fn scripting_commands<'a, Res, FSR>() -> &'a[(String, Box<dyn Fn(FSR) -> Box<dyn IntoPyNativeFn<>>)]
    // where
    //     FSR: Fn(PCom) -> Result<PComRep, ()>;

    fn get_command_duration(&self, player_state: &P, com: &PCom) -> GameTime;

    fn initialize_scope<FSR>(vm: &VirtualMachine, scope: &Scope, comm_chan: FSR)
    where
         FSR: Fn(PCom) -> Result<PComRep, ()> + Clone + 'static;
}
