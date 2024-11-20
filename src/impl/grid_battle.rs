use super::grid_orientation::GridOrientation;
use super::simple_battle_logic::{PlayerCommand, PlayerCommandReply};
use crate::battle::Battle;

pub use super::player_gridmap_control::GridPlayerState;

pub type GridBattle<GameLogic, LW> =
    Battle<GridPlayerState, GameLogic, PlayerCommand<GridOrientation>, PlayerCommandReply, LW>;
