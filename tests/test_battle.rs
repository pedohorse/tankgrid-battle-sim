use battle_sim::battle::{PlayerCommand, DEFAULT_COMMAND_DURATION};
use battle_sim::maptile_logic::MaptileLogic;
use battle_sim::r#impl::grid_battle::{GridBattle, GridPlayerState};
use battle_sim::r#impl::grid_map::GridBattleMap;
use battle_sim::r#impl::player_gridmap_control::GridOrientation;
use std::collections::HashMap;

#[derive(Clone, Copy)]
enum TileType {
    Nothin,
}

struct TestTrivialLogic {}

impl<T> MaptileLogic<T> for TestTrivialLogic {
    // trivial impl
    fn move_from(tile: T) -> T {
        tile
    }
    fn move_onto(tile: T) -> T {
        tile
    }
    fn passible(tile: T) -> bool {
        true
    }
    fn seethroughable(tile: T) -> bool {
        true
    }
    fn shoot(tile: T) -> T {
        tile
    }
}

#[test]
fn testtest() {
    let map = GridBattleMap::new(2, 2, TileType::Nothin, TileType::Nothin);
    let mut b = GridBattle::new(
        map,
        TestTrivialLogic {},
        vec![GridPlayerState::new(0, 0, GridOrientation::Down)],
        vec!["\
        print('hell-o')\n\
        turn_cw()\n\
        print('yeah!')\n\
        "
        .to_owned()],
        HashMap::new(),
    );
    b.run_simulation();
    assert_eq!(DEFAULT_COMMAND_DURATION, b.time());
}

#[test]
fn test2players() {
    let map = GridBattleMap::new(2, 2, TileType::Nothin, TileType::Nothin);
    let mut b = GridBattle::new(
        map,
        TestTrivialLogic {},
        vec![
            GridPlayerState::new(0, 0, GridOrientation::Down),
            GridPlayerState::new(1, 1, GridOrientation::Up),
        ],
        vec![
            "\
        print('hell-o')\n\
        turn_cw()\n\
        print('foo')\n\
        turn_cw()\n\
        print('yeah!')\n\
        "
            .to_owned(),
            "\
        print('second')\n\
        turn_ccw()\n\
        print('second yeah!')\n\
        "
            .to_owned(),
        ],
        HashMap::from([
            (PlayerCommand::MoveFwd, 100),
            (PlayerCommand::TurnCW, 10),
            (PlayerCommand::TurnCCW, 20),
        ]),
    );
    b.run_simulation();
    assert_eq!(DEFAULT_COMMAND_DURATION * 2, b.time());
}
