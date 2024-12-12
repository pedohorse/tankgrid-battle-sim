use battle_sim::battle::DEFAULT_COMMAND_DURATION;
use battle_sim::map_object::MapObject;
use battle_sim::maptile_logic::MaptileLogic;
use battle_sim::object_layer::ObjectLayer;
use battle_sim::player_state::PlayerControl;
use battle_sim::r#impl::grid_battle::{GridBattle, GridPlayerState};
use battle_sim::r#impl::grid_map::GridBattleMap;
use battle_sim::r#impl::grid_map_prober::GridMapProber;
use battle_sim::r#impl::grid_orientation::GridOrientation;
use battle_sim::r#impl::simple_battle_logic::{PlayerCommand, SimpleBattleLogic};
use battle_sim::r#impl::simple_battle_object_layer::SimpleBattleObjectLayer;
use std::collections::HashMap;

mod common;
use common::{SimpleTileType, TestTrivialLogic, VecLogWriter, HashmapCommandTimer};

struct TestNoObjectCache {}

impl<MObj> ObjectLayer<GridOrientation, MObj> for TestNoObjectCache
where
    MObj: MapObject<GridOrientation>,
{
    fn new() -> Self {
        TestNoObjectCache {}
    }
    fn add(&mut self, obj: MObj) -> u64 {0}
    fn clear(&mut self) {}
    fn objects(&self) -> &[MObj] {
        &[]
    }
    fn clear_by<F>(&mut self, f: F)
    where
        F: Fn(&MObj) -> bool,
    {
    }
    fn object_by_id(&self, uid: u64) -> Option<&MObj> {
        None
    }
    fn remove_object(&mut self, uid: u64) -> bool {
        false
    }
    fn objects_at(&self, x: i64, y: i64) -> Vec<&MObj> {
        Vec::new()
    }
}

#[test]
fn testtest() {
    let map = GridBattleMap::new(2, 2, SimpleTileType::Nothin, SimpleTileType::Nothin);
    let logger = VecLogWriter::new();
    let mut b = GridBattle::new(
        SimpleBattleLogic::new(
            map,
            TestTrivialLogic {},
            GridMapProber {},
            SimpleBattleObjectLayer::new(),
            HashmapCommandTimer::new(HashMap::new(), 10),
            0,
        ),
        vec![(
            GridPlayerState::new(0, 0, GridOrientation::South, 0, 1, "player1"),
            "\
            print('hell-o')\n\
            turn_cw()\n\
            print('yeah!')\n\
            "
            .to_owned(),
        )],
        logger,
    );
    b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();
    assert_eq!(DEFAULT_COMMAND_DURATION, b.time());
    assert_eq!(GridOrientation::West, b.player_state(0).orientation)
}

#[test]
fn test2players() {
    let map = GridBattleMap::new(2, 2, SimpleTileType::Nothin, SimpleTileType::Nothin);
    let logger = VecLogWriter::new();
    let mut b = GridBattle::new(
        SimpleBattleLogic::new(
            map,
            TestTrivialLogic {},
            GridMapProber {},
            SimpleBattleObjectLayer::new(),
            HashmapCommandTimer::new(HashMap::from([
                (PlayerCommand::MoveFwd, 100),
                (PlayerCommand::TurnCW, 10),
                (PlayerCommand::MoveFwd, 20),
            ]), 10),
            0,
        ),
        vec![
            (
                GridPlayerState::new(0, 0, GridOrientation::South, 0, 1, "player1"),
                "\
                print('hell-o')\n\
                turn_cw()\n\
                print('foo')\n\
                turn_cw()\n\
                print('yeah!')\n\
                "
                .to_owned(),
            ),
            (
                GridPlayerState::new(2, 2, GridOrientation::North, 0, 1, "player2"),
                "\
                print('second')\n\
                move_forward()\n\
                print('second yeah!')\n\
                "
                .to_owned(),
            ),
        ],
        logger,
    );
    b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();
    assert_eq!(20, b.time());
    assert_eq!(GridOrientation::North, b.player_state(0).orientation);
    assert_eq!(GridOrientation::North, b.player_state(1).orientation);
    assert_eq!(0, b.player_state(0).row);
    assert_eq!(0, b.player_state(0).col);
    assert_eq!(1, b.player_state(1).row);
    assert_eq!(2, b.player_state(1).col);
    assert!(!b.is_player_dead(0));
    assert!(!b.is_player_dead(1));
}

#[test]
fn test_2players_move_into_each_other() {
    let map = GridBattleMap::new(3, 3, SimpleTileType::Nothin, SimpleTileType::Nothin);
    let logger = VecLogWriter::new();
    let mut b = GridBattle::new(
        SimpleBattleLogic::new(
            map,
            TestTrivialLogic {},
            GridMapProber {},
            SimpleBattleObjectLayer::new(),
            HashmapCommandTimer::new(HashMap::from([
                (PlayerCommand::MoveFwd, 100),
                (PlayerCommand::TurnCW, 10),
                (PlayerCommand::MoveFwd, 20),
            ]), 10),
            0,
        ),
        vec![
            (
                GridPlayerState::new(0, 1, GridOrientation::East, 0, 1, "player1"),
                "\
                move_forward()\n\
                "
                .to_owned(),
            ),
            (
                GridPlayerState::new(2, 1, GridOrientation::West, 0, 1, "player2"),
                "\
                move_forward()\n\
                "
                .to_owned(),
            ),
        ],
        logger,
    );
    b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();
    println!(
        "{}: p0({},{}), p1({},{})",
        b.time(),
        b.player_state(0).col,
        b.player_state(0).row,
        b.player_state(1).col,
        b.player_state(1).row
    );
    assert_eq!(20, b.time());
    assert_eq!(GridOrientation::East, b.player_state(0).orientation);
    assert_eq!(GridOrientation::West, b.player_state(1).orientation);
    assert_eq!(1, b.player_state(0).row);
    assert_eq!(1, b.player_state(1).row);
    assert!(b.player_state(0).col != b.player_state(1).col);
    assert!(!b.is_player_dead(0));
    assert!(!b.is_player_dead(1));
}

#[test]
fn test_2players_move_past_each_other() {
    let map = GridBattleMap::new(3, 3, SimpleTileType::Nothin, SimpleTileType::Nothin);
    let logger = VecLogWriter::new();
    let mut b = GridBattle::new(
        SimpleBattleLogic::new(
            map,
            TestTrivialLogic {},
            GridMapProber {},
            SimpleBattleObjectLayer::new(),
            HashmapCommandTimer::new(HashMap::from([
                (PlayerCommand::MoveFwd, 100),
                (PlayerCommand::TurnCW, 10),
                (PlayerCommand::MoveFwd, 20),
            ]), 10),
            0,
        ),
        vec![
            (
                GridPlayerState::new(0, 1, GridOrientation::East, 0, 1, "player1"),
                "\
                move_forward()\n\
                "
                .to_owned(),
            ),
            (
                GridPlayerState::new(2, 2, GridOrientation::West, 0, 1, "player2"),
                "\
                move_forward()\n\
                "
                .to_owned(),
            ),
        ],
        logger,
    );
    b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();
    println!(
        "{}: p0({},{}), p1({},{})",
        b.time(),
        b.player_state(0).col,
        b.player_state(0).row,
        b.player_state(1).col,
        b.player_state(1).row
    );
    assert_eq!(20, b.time());
    assert_eq!(GridOrientation::East, b.player_state(0).orientation);
    assert_eq!(GridOrientation::West, b.player_state(1).orientation);
    assert_eq!(1, b.player_state(0).row);
    assert_eq!(2, b.player_state(1).row);
    assert!(b.player_state(0).col == b.player_state(1).col);
    assert!(!b.is_player_dead(0));
    assert!(!b.is_player_dead(1));
}

#[test]
fn test_2players_move_into_each_other_but_shoot() {
    let map = GridBattleMap::new(3, 3, SimpleTileType::Nothin, SimpleTileType::Nothin);
    let logger = VecLogWriter::new();
    let mut b = GridBattle::new(
        SimpleBattleLogic::new(
            map,
            TestTrivialLogic {},
            GridMapProber {},
            SimpleBattleObjectLayer::new(),
            HashmapCommandTimer::new(HashMap::from([
                (PlayerCommand::MoveFwd, 100),
                (PlayerCommand::TurnCW, 10),
                (PlayerCommand::MoveFwd, 20),
                (PlayerCommand::Shoot, 5),
            ]), 10),
            0,
        ),
        vec![
            (
                GridPlayerState::new(0, 1, GridOrientation::East, 1, 1, "player1"),
                "\
                print('p0 move')\n\
                move_forward()\n\
                print('p0 done')\n\
                "
                .to_owned(),
            ),
            (
                GridPlayerState::new(2, 1, GridOrientation::West, 1, 1, "player2"),
                "\
                print('p1 shoot')\n\
                shoot()\n\
                print('p1 move')\n\
                move_forward()\n\
                print('p1 move')\n\
                move_forward()\n\
                print('p1 done')\n\
                "
                .to_owned(),
            ),
        ],
        logger,
    );
    b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();
    println!(
        "{}: p0({},{}), p1({},{})",
        b.time(),
        b.player_state(0).col,
        b.player_state(0).row,
        b.player_state(1).col,
        b.player_state(1).row
    );
    assert_eq!(45 + 30, b.time());  // 30 - is 3 waits for default 10 each
    assert_eq!(GridOrientation::East, b.player_state(0).orientation);
    assert_eq!(GridOrientation::West, b.player_state(1).orientation);
    assert_eq!(0, b.player_state(0).col);
    assert_eq!(1, b.player_state(0).row);
    assert_eq!(0, b.player_state(0).col);
    assert_eq!(1, b.player_state(1).row);
    assert!(b.is_player_dead(0));
    assert!(!b.is_player_dead(1));
}

#[test]
fn test2players_inf_loop() {
    let map = GridBattleMap::new(2, 2, SimpleTileType::Nothin, SimpleTileType::Nothin);
    let logger = VecLogWriter::new();
    let mut b = GridBattle::new(
        SimpleBattleLogic::new(
            map,
            TestTrivialLogic {},
            GridMapProber {},
            SimpleBattleObjectLayer::new(),
            HashmapCommandTimer::new(HashMap::from([
                (PlayerCommand::MoveFwd, 100),
                (PlayerCommand::TurnCW, 10),
                (PlayerCommand::MoveFwd, 20),
            ]), 10),
            0,
        ),
        vec![
            (
                GridPlayerState::new(0, 0, GridOrientation::South, 0, 1, "player1"),
                "\
                print('hell-o')\n\
                turn_cw()\n\
                print('foo')\n\
                turn_cw()\n\
                print('yeah!')\n\
                "
                .to_owned(),
            ),
            (
                GridPlayerState::new(2, 2, GridOrientation::North, 0, 1, "[;ayer2"),
                "\
                print('second loops')\n\
                while True:
                    pass
                "
                .to_owned(),
            ),
        ],
        logger,
    );
    b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();
    assert_eq!(20, b.time());
}

#[test]
fn test2players_bad_inf_loop() {
    let map = GridBattleMap::new(2, 2, SimpleTileType::Nothin, SimpleTileType::Nothin);
    let logger = VecLogWriter::new();
    let mut b = GridBattle::new(
        SimpleBattleLogic::new(
            map,
            TestTrivialLogic {},
            GridMapProber {},
            SimpleBattleObjectLayer::new(),
            HashmapCommandTimer::new(HashMap::from([
                (PlayerCommand::MoveFwd, 100),
                (PlayerCommand::TurnCW, 10),
                (PlayerCommand::MoveFwd, 20),
            ]), 10),
            0,
        ),
        vec![
            (
                GridPlayerState::new(0, 0, GridOrientation::South, 0, 1, "player1"),
                "\
                print('hell-o')\n\
                turn_cw()\n\
                print('foo')\n\
                turn_cw()\n\
                print('yeah!')\n\
                "
                .to_owned(),
            ),
            (
                GridPlayerState::new(2, 2, GridOrientation::North, 0, 1, "player2"),
                "\
                print('second loops')\n\
                while True:
                    try:
                        while True: pass
                    except:
                        continue
                "
                .to_owned(),
            ),
        ],
        logger,
    );
    b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();
    assert_eq!(20, b.time());
}
