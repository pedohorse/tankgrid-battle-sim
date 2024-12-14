use super::gametime::GameTime;
use super::log_data::{LogRepresentable, LogWriter};
use super::player_state::PlayerControl;

use rustpython_vm::scope::Scope;
use rustpython_vm::vm::VirtualMachine;

pub trait BattleLogic<P, PCom, PComRep, LO, LA>
where
    P: PlayerControl,
    LO: LogRepresentable,
    LA: LogRepresentable,
{
    fn is_player_dead(&self, player: &P) -> bool;

    /// return None if game still goes on
    /// or Some(winner ids) if game is finished
    fn game_finished(&self, players: &[P]) -> Option<Vec<usize>>;

    fn initial_setup<LWF>(&mut self, player_states: &mut [P], logger: &mut LWF)
    where
        LWF: FnMut(LO, LA),
    {
    }

    fn process_commands<LWF>(
        &mut self,
        player_i: usize,
        com: &PCom,
        player_states: &mut [P],
        logger: &mut LWF,
    ) -> (PComRep, Option<Vec<PCom>>)
    where
        LWF: FnMut(LO, LA);

    fn get_command_duration(&self, player_state: &P, com: &PCom) -> GameTime;

    fn initialize_scope<FSR>(vm: &VirtualMachine, scope: &Scope, comm_chan: FSR)
    where
        FSR: Fn(PCom) -> Result<PComRep, ()> + Clone + 'static;
}
