pub trait MaptileLogic<T> {
    fn pass_speed_percentage(&self, tile: T) -> u32;
    fn turn_speed_percentage(&self, tile: T) -> u32;
    fn passable(&self, tile: T) -> bool;
    fn seethroughable(&self, tile: T) -> bool;
    fn shoot(&self, tile: T) -> T;
    fn move_onto(&self, tile: T) -> T;
    fn move_from(&self, tile: T) -> T;
}