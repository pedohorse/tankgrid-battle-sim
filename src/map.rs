pub trait MapReadAccess<T> {
    fn get_tile_at(&self, x: i64, y: i64) -> T;
    fn is_within_bounds(&self, x: i64, y: i64) -> bool;
}

pub trait MapWriteAccess<T> {
    fn set_tile_at(&mut self, x: i64, y: i64, val: T);
}
