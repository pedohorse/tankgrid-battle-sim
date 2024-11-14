use crate::map::MapReadAccess;
use crate::map_object::MapObject;
use crate::maptile_logic::MaptileLogic;
use crate::player_state::PlayerControl;
use crate::script_repr::ToScriptRepr;
use crate::object_layer::ObjectLayer;

use super::grid_orientation::GridOrientation;
//use super::tile_types::TileType;
//use super::trivial_object_layer::TrivialObjectLayer;

pub struct GridPlayerState {
    pub row: i64,
    pub col: i64,
    pub orientation: GridOrientation,
    pub ammo: usize,
}

impl GridPlayerState {
    pub fn new(row: i64, col: i64, orientation: GridOrientation) -> GridPlayerState {
        GridPlayerState {
            row,
            col,
            orientation,
            ammo: 0,
        }
    }
}

impl<T, C, M, MObj, OL> PlayerControl<GridOrientation, M, T, C, MObj, OL> for GridPlayerState
where
    C: MaptileLogic<T>,
    M: MapReadAccess<T>,
    MObj: MapObject<GridOrientation>,
    OL: ObjectLayer<GridOrientation, MObj>,
{
    fn move_forward(&mut self, map: &mut M, logic: &C, object_layer: &OL) {
        match self.orientation {
            GridOrientation::Up => {
                let tile = map.get_tile_at(self.col, self.row - 1);
                if logic.passable(tile) && object_layer.objects_at_are_passable(self.col, self.row - 1) {
                    self.row -= 1;
                }
            }
            GridOrientation::Right => {
                let tile = map.get_tile_at(self.col + 1, self.row);
                if logic.passable(tile) && object_layer.objects_at_are_passable(self.col + 1, self.row) {
                    self.col += 1;
                }
            }
            GridOrientation::Down => {
                let tile = map.get_tile_at(self.col, self.row + 1);
                if logic.passable(tile) && object_layer.objects_at_are_passable(self.col, self.row + 1) {
                    self.row += 1;
                }
            }
            GridOrientation::Left => {
                let tile = map.get_tile_at(self.col - 1, self.row);
                if logic.passable(tile) && object_layer.objects_at_are_passable(self.col - 1, self.row) {
                    self.col -= 1;
                }
            }
        }
    }

    fn turn_cw(&mut self, _map: &mut M, _logic: &C, _object_layer: &OL) {
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

    fn turn_ccw(&mut self, _map: &mut M, _logic: &C, _object_layer: &OL) {
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
        if res_id != 0 {
            return;
        }
        self.ammo = if amount >= self.ammo {
            0
        } else {
            self.ammo - amount
        };
    }
    fn gain_resource(&mut self, res_id: usize, amount: usize) {
        if res_id != 0 {
            return;
        }
        self.ammo += amount
    }

    fn resource_value(&self, res_id: usize) -> usize {
        if res_id != 0 {
            return 0;
        }
        self.ammo
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
        "enemy".to_owned()
    }
}