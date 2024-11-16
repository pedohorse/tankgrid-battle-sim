use super::gametime::GameTime;

pub trait LogRepresentable
where Self: Sized {
    fn to_log_repr(self) -> String {
        self.log_repr()
    }
    fn log_repr(&self) -> String;
}

impl LogRepresentable for String {
    fn to_log_repr(self) -> String {
        self
    }

    fn log_repr(&self) -> String {
        self.to_owned()
    }
}

// pub trait ToLogAction<LR>
// where
//     LR: LogRepresentable,
// {
//     fn to_log_action(&self) -> LR;
// }

pub trait LogWriter<LRO, LRA>
where
    LRO: LogRepresentable,
    LRA: LogRepresentable,
{
    fn add_log_data(&mut self, object: LRO, action: LRA, time: GameTime, duration: GameTime);
}
