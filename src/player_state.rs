use super::map::MapReadAccess;
use super::maptile_logic::MaptileLogic;

pub struct PlayerState<R> {
    pub row: usize,
    pub col: usize,
    pub orientation: R,
}

impl<R> PlayerState<R> {
    pub fn new(row: usize, col: usize, orientation: R) -> PlayerState<R> {
        PlayerState {
            row,
            col,
            orientation,
        }
    }
}

pub trait PlayerControl<R, M, T, C>
where
    M: MapReadAccess<T>,
    C: MaptileLogic<T>,
{
    fn move_forward(&mut self, map: &mut M, logic: &C);
    fn turn_cw(&mut self, map: &mut M, logic: &C);
    fn turn_ccw(&mut self, map: &mut M, logic: &C);
}
