use super::gametime::GameTime;

pub struct BattleStateInfo {
    pub game_time: GameTime,
}

impl BattleStateInfo {
    pub fn new(game_time: GameTime) -> BattleStateInfo {
        BattleStateInfo{
            game_time
        }
    }
}