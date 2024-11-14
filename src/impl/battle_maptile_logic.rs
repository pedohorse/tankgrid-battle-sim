use super::tile_types::TileType;
use crate::maptile_logic::MaptileLogic;

pub struct ConfigurableMaptileLogic {}

impl MaptileLogic<TileType> for ConfigurableMaptileLogic {
    fn move_from(&self, tile: TileType) -> TileType {
        tile
    }
    fn move_onto(&self, tile: TileType) -> TileType {
        tile
    }
    fn pass_speed_percentage(&self, tile: TileType) -> u32 {
        match tile {
            TileType::Ground => 100,
            TileType::Mud => 50,
            TileType::Wall => 0,
        }
    }
    fn turn_speed_percentage(&self, tile: TileType) -> u32 {
        match tile {
            TileType::Ground => 100,
            TileType::Mud => 80,
            TileType::Wall => 0,
        }
    }
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
