use crate::map_object::MapObject;
use crate::player_state::PlayerControl;
use crate::script_repr::ToScriptRepr;
use crate::log_data::LogRepresentable;

use super::grid_orientation::GridOrientation;
//use super::tile_types::TileType;
//use super::trivial_object_layer::TrivialObjectLayer;

pub struct GridPlayerState {
    pub row: i64,
    pub col: i64,
    pub orientation: GridOrientation,
    pub ammo: usize,
    pub health: usize,
    pub name: String,
}

impl GridPlayerState {
    pub fn new(
        col: i64,
        row: i64,
        orientation: GridOrientation,
        ammo: usize,
        health: usize,
        name: &str,
    ) -> GridPlayerState {
        GridPlayerState {
            row,
            col,
            orientation,
            ammo,
            health,
            name: name.to_owned(),
        }
    }
}

impl PlayerControl for GridPlayerState
{
    fn forward_pos(&self) -> (i64, i64) {
        match self.orientation {
            GridOrientation::Up => (self.col, self.row - 1),
            GridOrientation::Right => (self.col + 1, self.row),
            GridOrientation::Down => (self.col, self.row + 1),
            GridOrientation::Left => (self.col - 1, self.row),
        }
    }

    fn move_forward(&mut self) {
        let (x, y) = self.forward_pos();
        self.col = x;
        self.row = y;
    }

    fn turn_cw(&mut self) {
        match self.orientation {
            GridOrientation::Up => {
                self.orientation = GridOrientation::Right;
            }
            GridOrientation::Right => {
                self.orientation = GridOrientation::Down;
            }
            GridOrientation::Down => {
                self.orientation = GridOrientation::Left;
            }
            GridOrientation::Left => {
                self.orientation = GridOrientation::Up;
            }
        }
    }

    fn turn_ccw(&mut self) {
        match self.orientation {
            GridOrientation::Up => {
                self.orientation = GridOrientation::Left;
            }
            GridOrientation::Right => {
                self.orientation = GridOrientation::Up;
            }
            GridOrientation::Down => {
                self.orientation = GridOrientation::Right;
            }
            GridOrientation::Left => {
                self.orientation = GridOrientation::Down;
            }
        }
    }

    fn expend_resource(&mut self, res_id: usize, amount: usize) {
        let res = match res_id {
            0 => &mut self.health,
            1 => &mut self.ammo,
            _ => return,
        };

        *res = if amount >= *res { 0 } else { *res - amount };
    }

    fn gain_resource(&mut self, res_id: usize, amount: usize) {
        let res = match res_id {
            0 => &mut self.health,
            1 => &mut self.ammo,
            _ => return,
        };
        *res += amount;
    }

    fn resource_value(&self, res_id: usize) -> usize {
        match res_id {
            0 => self.health,
            1 => self.ammo,
            _ => 0,
        }
    }

    fn is_dead(&self) -> bool {
        self.health <= 0
    }
}

impl MapObject<GridOrientation> for GridPlayerState {
    fn position(&self) -> (i64, i64) {
        (self.col, self.row)
    }

    fn orientation(&self) -> GridOrientation {
        self.orientation
    }
}

impl ToScriptRepr for GridPlayerState {
    fn to_script_repr(&self) -> String {
        self.name.to_owned()
    }
}

impl LogRepresentable for GridPlayerState {
    fn log_repr(&self) -> String {
        self.name.to_owned()
    }

    fn to_log_repr(self) -> String {
        self.name.to_owned()
    }
}