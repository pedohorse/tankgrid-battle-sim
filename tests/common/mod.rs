use battle_sim::gametime::GameTime;
use battle_sim::log_data::{LogRepresentable, LogWriter};
use battle_sim::maptile_logic::MaptileLogic;
use battle_sim::script_repr::ToScriptRepr;

#[derive(Clone, Copy)]
pub enum SimpleTileType {
    Nothin,
    Wall,
}

impl ToScriptRepr for SimpleTileType {
    fn to_script_repr(&self) -> String {
        match self {
            SimpleTileType::Nothin => "empty_tile",
            SimpleTileType::Wall => "wall",
        }.to_owned()
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

    pub fn print(&self) {
        for (lobject, laction, time, duration) in &self.log_datas {
            println!(
                "{}  {}  {}  {}",
                time,
                duration,
                lobject.log_repr(),
                laction.log_repr()
            );
        }
    }
}

pub struct TestTrivialLogic {}

impl<T> MaptileLogic<T> for TestTrivialLogic {
    // trivial impl
    fn move_from(&self, tile: T) -> T {
        tile
    }
    fn move_onto(&self, tile: T) -> T {
        tile
    }
    fn pass_speed_percentage(&self, _tile: T) -> u32 {
        100
    }
    fn turn_speed_percentage(&self, _tile: T) -> u32 {
        100
    }
    fn passable(&self, _tile: T) -> bool {
        true
    }
    fn seethroughable(&self, _tile: T) -> bool {
        true
    }
    fn shoot(&self, tile: T) -> T {
        tile
    }
}

pub struct TestSimpleLogic {}

impl MaptileLogic<SimpleTileType> for TestSimpleLogic {
    // trivial impl
    fn move_from(&self, tile: SimpleTileType) -> SimpleTileType {
        tile
    }
    fn move_onto(&self, tile: SimpleTileType) -> SimpleTileType {
        tile
    }
    fn pass_speed_percentage(&self, _tile: SimpleTileType) -> u32 {
        100
    }
    fn turn_speed_percentage(&self, _tile: SimpleTileType) -> u32 {
        100
    }
    fn passable(&self, tile: SimpleTileType) -> bool {
        match tile {
            SimpleTileType::Nothin => true,
            SimpleTileType::Wall => false,
        }
    }
    fn seethroughable(&self, tile: SimpleTileType) -> bool {
        match tile {
            SimpleTileType::Nothin => true,
            SimpleTileType::Wall => false,
        }
    }
    fn shoot(&self, tile: SimpleTileType) -> SimpleTileType {
        tile
    }
}
