use crate::gametime::GameTime;

use super::player_state::PlayerControl;
use rustpython_vm::scope::Scope;
use rustpython_vm::vm::VirtualMachine;

pub trait BattleLogic<P, PCom, PComRep>
where
    P: PlayerControl,
{
    fn process_commands(&mut self, player_i: usize, com: PCom, player_states: &mut [P]) -> PComRep;

    fn get_command_duration(&self, player_state: &P, com: &PCom) -> GameTime;

    fn initialize_scope<FSR>(vm: &VirtualMachine, scope: &Scope, comm_chan: FSR)
    where
        FSR: Fn(PCom) -> Result<PComRep, ()> + Clone + 'static;
}
