use crate::map::MapReadAccess;

use crate::player_state::{PlayerControl, PlayerState};
use crate::maptile_logic::MaptileLogic;

pub enum GridOrientation {
    Up,
    Right,
    Down,
    Left,
}

impl<T, C, M> PlayerControl<GridOrientation, M, T, C> for PlayerState<GridOrientation>
where
    T: Copy + Clone,
    C: MaptileLogic<T>,
    M: MapReadAccess<T>,
{
    fn move_forward(&mut self, map: &mut M, logic: &C) {
        match self.orientation {
            GridOrientation::Up => {
                self.row -= 1;
            }
            GridOrientation::Right => {
                self.col += 1;
            }
            GridOrientation::Down => {
                self.row += 1;
            }
            GridOrientation::Left => {
                self.col -= 1;
            }
        }
    }

    fn turn_cw(&mut self, map: &mut M, logic: &C) {
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

    fn turn_ccw(&mut self, map: &mut M, logic: &C) {
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
}
