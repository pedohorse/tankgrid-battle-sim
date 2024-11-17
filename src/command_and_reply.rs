pub trait CommandReplyStat {
    fn command_succeeded(&self) -> bool;
}