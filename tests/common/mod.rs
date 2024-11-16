use battle_sim::gametime::GameTime;
use battle_sim::log_data::{LogRepresentable, LogWriter};
use battle_sim::script_repr::ToScriptRepr;

#[derive(Clone, Copy)]
pub enum SimpleTileType {
    Nothin,
}

impl ToScriptRepr for SimpleTileType {
    fn to_script_repr(&self) -> String {
        "empty_tile".to_owned()
    }
}

pub struct VecLogWriter<LO, LA>
where
    LO: LogRepresentable,
    LA: LogRepresentable,
{
    pub log_datas: Vec<(LO, LA, GameTime, GameTime)>,
}

impl<LO, LA> LogWriter<LO, LA> for VecLogWriter<LO, LA>
where
    LO: LogRepresentable,
    LA: LogRepresentable,
{
    fn add_log_data(&mut self, object: LO, action: LA, time: GameTime, duration: GameTime) {
        self.log_datas.push((object, action, time, duration));
    }
}

impl<LO, LA> VecLogWriter<LO, LA>
where
    LO: LogRepresentable,
    LA: LogRepresentable,
{
    pub fn new() -> VecLogWriter<LO, LA> {
        VecLogWriter {
            log_datas: Vec::new(),
        }
    }
}
