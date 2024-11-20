pub trait MapObject<R>: Sized {
    fn unique_id(&self) -> u64;

    fn position(&self) -> (i64, i64);
    fn orientation(&self) -> R;

    fn seethroughable(&self) -> bool {
        false
    }

    fn passable(&self) -> bool {
        false
    }

    fn shootable(&self) -> bool {
        true
    }
}