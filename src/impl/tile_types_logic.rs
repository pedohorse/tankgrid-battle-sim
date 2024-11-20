use super::tile_types::TileType;
use crate::maptile_logic::MaptileLogic;

pub struct TileTypeLogic {}

impl MaptileLogic<TileType> for TileTypeLogic {
    fn passable(&self, tile: TileType) -> bool {
        match tile {
            TileType::Ground => true,
            TileType::Mud => true,
            TileType::Wall => false,
        }
    }
    fn seethroughable(&self, tile: TileType) -> bool {
        match tile {
            TileType::Ground => true,
            TileType::Mud => true,
            TileType::Wall => false,
        }
    }
    fn shoot(&self, tile: TileType) -> TileType {
        tile
    }
}

impl TileTypeLogic {
    pub fn new() -> TileTypeLogic {
        TileTypeLogic {}
    }
}