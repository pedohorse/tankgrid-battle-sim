pub trait MapObject<R>: Sized {
    fn clone_with_uid(source: &Self, new_uid: u64) -> Self;

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