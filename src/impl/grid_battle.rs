use crate::battle::Battle;
use crate::player_state::PlayerState;
use super::grid_map::GridBattleMap;
//use super::tile_types::TileType;
//use super::battle_maptile_logic::ConfigurableMaptileLogic;
use super::player_gridmap_control::GridOrientation;

pub type GridPlayerState = PlayerState<GridOrientation>;
pub type GridBattle<T, L> = Battle<T, GridBattleMap<T>, L, GridOrientation, GridPlayerState>;

