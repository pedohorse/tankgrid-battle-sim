use super::grid_map::GridBattleMap;
use super::grid_map_prober::GridMapProber;
use super::grid_orientation::GridOrientation;
use super::simple_command_logic::{ObjectCacheRepr, SimpleBattleLogic, PlayerCommand, PlayerCommandReply};
use super::trivial_object_layer::TrivialObjectLayer;
use crate::battle::Battle;
use crate::command_logic::BattleLogic;
use crate::map_object::MapObject;
use crate::maptile_logic::MaptileLogic;
use crate::script_repr::ToScriptRepr;

pub use super::player_gridmap_control::GridPlayerState;

pub type GridBattle<T, L, GameLogic> = Battle<
    T,
    GridBattleMap<T>,
    L,
    GridOrientation,
    ObjectCacheRepr<GridOrientation>,
    GridPlayerState,
    GridMapProber,
    TrivialObjectLayer<ObjectCacheRepr<GridOrientation>>,
    GameLogic,
    PlayerCommand<GridOrientation>, PlayerCommandReply
>;
