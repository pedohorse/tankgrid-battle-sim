pub trait MapObject<R>: Sized {
    fn position(&self) -> (i64, i64);
    fn orientation(&self) -> R;

    fn seethroughable(&self) -> bool {
        false
    }

    fn passable(&self) -> bool {
        false
    }
}