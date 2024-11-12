use super::tile_types::TileType;
use crate::maptile_logic::MaptileLogic;

pub struct ConfigurableMaptileLogic {}

impl MaptileLogic<TileType> for ConfigurableMaptileLogic {
    fn move_from(tile: TileType) -> TileType {
        tile
    }
    fn move_onto(tile: TileType) -> TileType {
        tile
    }
    fn passible(tile: TileType) -> bool {
        match tile {
            TileType::Ground => true,
        }
    }
    fn seethroughable(tile: TileType) -> bool {
        match tile {
            TileType::Ground => true,
        }
    }
    fn shoot(tile: TileType) -> TileType {
        tile
    }
}
