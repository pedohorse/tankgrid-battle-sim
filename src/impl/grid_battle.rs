use super::grid_orientation::GridOrientation;
use super::simple_battle_logic::{
    PlayerCommand, PlayerCommandReply, AMMO_RES, HEALTH_RES, MAX_FREE_PRINTS, PRINT_COUNTER_RES,
};
use crate::battle::Battle;

pub use super::player_gridmap_control::GridPlayerState;

pub fn new_player(
    col: i64,
    row: i64,
    orientation: GridOrientation,
    ammo: u64,
    health: u64,
    name: &str,
) -> GridPlayerState {
    let mut res = vec![0 as u64; 4];
    res[HEALTH_RES] = health;
    res[AMMO_RES] = ammo;
    res[PRINT_COUNTER_RES] = MAX_FREE_PRINTS;
    
    GridPlayerState::new(col, row, orientation, res, name)
}

pub type GridBattle<GameLogic, LW> = Battle<
    GridPlayerState,
    GameLogic,
    PlayerCommand<GridOrientation>,
    PlayerCommandReply<GridOrientation>,
    LW,
>;
