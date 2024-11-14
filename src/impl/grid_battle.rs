use super::grid_map::GridBattleMap;
use crate::battle::Battle;
use super::grid_orientation::GridOrientation;
use super::grid_map_prober::GridMapProber;
use super::trivial_object_layer::TrivialObjectLayer;

pub use super::player_gridmap_control::GridPlayerState;
pub type GridBattle<T, L, MObj> = Battle<T, GridBattleMap<T>, L, GridOrientation, GridPlayerState, GridMapProber, TrivialObjectLayer<MObj>>;
