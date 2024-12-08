use std::sync::atomic::Ordering;

use crate::map_object::MapObject;
use crate::player_state::PlayerControl;
use crate::script_repr::ToScriptRepr;
use crate::log_data::LogRepresentable;

use super::unique_id_counter::NEXT_OBJID;
use super::grid_orientation::GridOrientation;
//use super::tile_types::TileType;
//use super::trivial_object_layer::TrivialObjectLayer;

pub struct GridPlayerState {
    pub row: i64,
    pub col: i64,
    pub orientation: GridOrientation,
    pub ammo: u64,  // TODO: these should net be so specific, no point
    pub health: u64,  // TODO: and below res ids are hardcoded to these values
    last_hit_repr: u64,
    pub name: String,
    unique_id: u64,
}

impl GridPlayerState {
    pub fn new(
        col: i64,
        row: i64,
        orientation: GridOrientation,
        ammo: u64,
        health: u64,
        name: &str,
    ) -> GridPlayerState {
        GridPlayerState {
            row,
            col,
            orientation,
            ammo,
            health,
            last_hit_repr: 0,
            name: name.to_owned(),
            unique_id: NEXT_OBJID.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl PlayerControl for GridPlayerState
{
    fn move_to(&mut self, pos: (i64, i64)) {
        self.col = pos.0;
        self.row = pos.1;
    }

    fn turn_cw(&mut self) {
        match self.orientation {
            GridOrientation::North => {
                self.orientation = GridOrientation::East;
            }
            GridOrientation::East => {
                self.orientation = GridOrientation::South;
            }
            GridOrientation::South => {
                self.orientation = GridOrientation::West;
            }
            GridOrientation::West => {
                self.orientation = GridOrientation::North;
            }
        }
    }

    fn turn_ccw(&mut self) {
        match self.orientation {
            GridOrientation::North => {
                self.orientation = GridOrientation::West;
            }
            GridOrientation::East => {
                self.orientation = GridOrientation::North;
            }
            GridOrientation::South => {
                self.orientation = GridOrientation::East;
            }
            GridOrientation::West => {
                self.orientation = GridOrientation::South;
            }
        }
    }

    fn expend_resource(&mut self, res_id: usize, amount: u64) {
        let res = match res_id {
            0 => &mut self.health,
            1 => &mut self.ammo,
            _ => return,
        };

        *res = if amount >= *res { 0 } else { *res - amount };
    }

    fn gain_resource(&mut self, res_id: usize, amount: u64) {
        let res = match res_id {
            0 => &mut self.health,
            1 => &mut self.ammo,
            _ => return,
        };
        *res += amount;
    }

    fn resource_value(&self, res_id: usize) -> u64 {
        match res_id {
            0 => self.health,
            1 => self.ammo,
            2 => self.last_hit_repr,
            _ => 0,
        }
    }

    fn set_resource(&mut self, res_id: usize, amount: u64) {
        let res = match res_id {
            0 => &mut self.health,
            1 => &mut self.ammo,
            2 => &mut self.last_hit_repr,
            _ => return,
        };
        *res = amount;
    }
}

impl MapObject<GridOrientation> for GridPlayerState {
    fn clone_with_uid(source: &Self, new_uid: u64) -> Self {
        GridPlayerState {
            row: source.row,
            col: source.col,
            orientation: source.orientation,
            ammo: source.ammo,
            health: source.health,
            last_hit_repr: source.last_hit_repr,
            name: source.name.clone(),
            unique_id: new_uid,
        }
    }

    fn unique_id(&self) -> u64 {
        self.unique_id
    }

    fn position(&self) -> (i64, i64) {
        (self.col, self.row)
    }

    fn orientation(&self) -> GridOrientation {
        self.orientation
    }
}

impl ToScriptRepr for GridPlayerState {
    fn to_script_repr(&self) -> String {
        format!("player[{}]", self.name)
    }
}

impl LogRepresentable for GridPlayerState {
    fn log_repr(&self) -> String {
        format!("player[{}]({})", self.name, self.unique_id)
    }
}