pub trait MaptileLogic<T> {
    fn passible(tile: T) -> bool;
    fn seethroughable(tile: T) -> bool;
    fn shoot(tile: T) -> T;
    fn move_onto(tile: T) -> T;
    fn move_from(tile: T) -> T;
}