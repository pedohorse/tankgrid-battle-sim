use super::gametime::GameTime;
use super::log_data::LogRepresentable;
use super::player_state::PlayerControl;
use super::battle_state_info::BattleStateInfo;

use rustpython_vm::scope::Scope;
use rustpython_vm::vm::VirtualMachine;

pub trait BattleLogic<P, PCom, PComRep, GameEvent, LO, LA>
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
        let _ = player_states; // avoid unused var warning
        let _ = logger; // avoid unused var warning
    }

    fn process_events<LWF>(
        &mut self,
        event: &GameEvent,
        players_states: &mut [P],
        battle_info: &BattleStateInfo,
        logger: &mut LWF,
    ) -> Option<Vec<(GameTime, GameEvent)>>; // time offset till event

    /// called when command is received from player,
    /// but is not yet to be processed
    fn command_received<LWF>(
        &mut self,
        player_i: usize,
        command: &PCom,
        command_id: usize,
        player_states: &[P],
        battle_info: &BattleStateInfo,
        logger: &mut LWF,
    ) -> Option<Vec<(GameTime, GameEvent)>> {
        // avoid unused var warning
        let _ = player_i;
        let _ = command;
        let _ = command_id;
        let _ = player_states;
        let _ = battle_info;
        let _ = logger;
        None
    }

    fn process_commands<LWF>(
        &mut self,
        player_i: usize,
        command: &PCom,
        command_id: usize,
        player_states: &mut [P],
        battle_info: &BattleStateInfo,
        logger: &mut LWF,
    ) -> (PComRep, Option<Vec<PCom>>, Option<Vec<(GameTime, GameEvent)>>) // last one is time till event, not abs time
    where
        LWF: FnMut(LO, LA);

    /// called when command is about to be delivered to the player
    fn command_reply_delivered<LWF>(
        &mut self,
        player_i: usize,
        command: &PCom,
        command_id: usize,
        player_states: &[P],
        battle_info: &BattleStateInfo,
        logger: &mut LWF,
    ) -> Option<Vec<(GameTime, GameEvent)>> {
        // avoid unused var warning
        let _ = player_i;
        let _ = command;
        let _ = command_id;
        let _ = player_states;
        let _ = battle_info;
        let _ = logger;
        None
    }

    fn get_command_duration(&self, player_state: &P, com: &PCom) -> GameTime;
    fn get_command_reply_delay(&self, player_state: &P, com: &PCom) -> GameTime;

    fn initialize_scope<FSR>(vm: &VirtualMachine, scope: &Scope, comm_chan: FSR)
    where
        FSR: Fn(PCom) -> Result<PComRep, ()> + Clone + 'static;
}
