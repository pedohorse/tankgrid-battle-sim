use battle_sim::battle::DEFAULT_COMMAND_DURATION;
use battle_sim::map_object::MapObject;
use battle_sim::object_layer::ObjectLayer;
use battle_sim::r#impl::grid_battle::{new_player, GridBattle};
use battle_sim::r#impl::grid_map::GridBattleMap;
use battle_sim::r#impl::grid_map_prober::GridMapProber;
use battle_sim::r#impl::grid_orientation::GridOrientation;
use battle_sim::r#impl::simple_battle_logic::{
    PlayerCommand, SimpleBattleLogic, MAX_LOG_LINE_LENGTH,
};
use battle_sim::r#impl::simple_battle_object_layer::SimpleBattleObjectLayer;
use std::collections::HashMap;

mod common;
use common::{FnCommandTimer, HashmapCommandTimer, SimpleTileType, TestTrivialLogic, VecLogWriter};

struct TestNoObjectCache {}

impl<MObj> ObjectLayer<GridOrientation, MObj> for TestNoObjectCache
where
    MObj: MapObject<GridOrientation>,
{
    fn new() -> Self {
        TestNoObjectCache {}
    }
    fn add(&mut self, obj: MObj) -> u64 {
        0
    }
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
            FnCommandTimer::new(|com| match com {
                PlayerCommand::Print(_) => 0,
                _ => 10,
            }),
            0,
        ),
        vec![(
            new_player(0, 0, GridOrientation::South, 0, 1, "player1"),
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
            FnCommandTimer::new(|com| match com {
                PlayerCommand::TurnCW => 10,
                PlayerCommand::MoveFwd => 20,
                PlayerCommand::Print(_) => 0,
                _ => 10,
            }),
            0,
        ),
        vec![
            (
                new_player(0, 0, GridOrientation::South, 0, 1, "player1"),
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
                new_player(2, 2, GridOrientation::North, 0, 1, "player2"),
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
            HashmapCommandTimer::new(
                HashMap::from([
                    (PlayerCommand::MoveFwd, 100),
                    (PlayerCommand::TurnCW, 10),
                    (PlayerCommand::MoveFwd, 20),
                ]),
                10,
            ),
            0,
        ),
        vec![
            (
                new_player(0, 1, GridOrientation::East, 0, 1, "player1"),
                "\
                move_forward()\n\
                "
                .to_owned(),
            ),
            (
                new_player(2, 1, GridOrientation::West, 0, 1, "player2"),
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
            HashmapCommandTimer::new(
                HashMap::from([
                    (PlayerCommand::MoveFwd, 100),
                    (PlayerCommand::TurnCW, 10),
                    (PlayerCommand::MoveFwd, 20),
                ]),
                10,
            ),
            0,
        ),
        vec![
            (
                new_player(0, 1, GridOrientation::East, 0, 1, "player1"),
                "\
                move_forward()\n\
                "
                .to_owned(),
            ),
            (
                new_player(2, 2, GridOrientation::West, 0, 1, "player2"),
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
            FnCommandTimer::new(|com| match com {
                PlayerCommand::TurnCW => 10,
                PlayerCommand::MoveFwd => 20,
                PlayerCommand::Shoot => 5,
                PlayerCommand::Print(_) => 0,
                _ => 10,
            }),
            0,
        ),
        vec![
            (
                new_player(0, 1, GridOrientation::East, 1, 1, "player1"),
                "\
                print('p0 move')\n\
                move_forward()\n\
                print('p0 done')\n\
                "
                .to_owned(),
            ),
            (
                new_player(2, 1, GridOrientation::West, 1, 1, "player2"),
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
    assert_eq!(45 + 30, b.time()); // 30 - is 3 waits for default 10 each
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
            FnCommandTimer::new(|com| match com {
                PlayerCommand::TurnCW => 10,
                PlayerCommand::MoveFwd => 20,
                PlayerCommand::Print(_) => 0,
                _ => 10,
            }),
            0,
        ),
        vec![
            (
                new_player(0, 0, GridOrientation::South, 0, 1, "player1"),
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
                new_player(2, 2, GridOrientation::North, 0, 1, "[;ayer2"),
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
            FnCommandTimer::new(|com| match com {
                PlayerCommand::TurnCW => 10,
                PlayerCommand::MoveFwd => 20,
                PlayerCommand::Print(_) => 0,
                _ => 10,
            }),
            0,
        ),
        vec![
            (
                new_player(0, 0, GridOrientation::South, 0, 1, "player1"),
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
                new_player(2, 2, GridOrientation::North, 0, 1, "player2"),
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

#[test]
fn test_print_limit() {
    let map = GridBattleMap::new(2, 2, SimpleTileType::Nothin, SimpleTileType::Nothin);
    let logger = VecLogWriter::new();
    let mut b = GridBattle::new(
        SimpleBattleLogic::new(
            map,
            TestTrivialLogic {},
            GridMapProber {},
            SimpleBattleObjectLayer::new(),
            FnCommandTimer::new(|com| {
                match com {
                    PlayerCommand::Print(_) => 0,
                    PlayerCommand::Wait => 7,
                    _ => 10,
                }
            }),
            0,
        ),
        vec![(
            new_player(0, 0, GridOrientation::South, 0, 1, "player1"),
            "\
            print('line1')\n\
            print('line2 very long, must be truncated to the max symbol count. hm... what should i add here, need some water just as for my thesis... maybe discuss weather? nah, weather is too boring and gloomy, aaand i think we are long enough now to get truncated, but if no - just add some shit here, we can talk all night, i have nowhere to be and nothing to do')\n\
            print('line3 \\nnext line should not be allowed\\n\\nfoo!')\n\
            print('line4')\n\
            print('line5')\n\
            print('line6')\n\
            print('line7')\n\
            turn_cw()\n\
            print('2line1')\n\
            print('2line2')\n\
            print('2line3')\n\
            print('2line4')\n\
            print('2line5')\n\
            print('2line6')\n\
            print('2line7')\n\
            "
            .to_owned(),
        )],
        logger,
    );
    b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();
    assert_eq!(10 + 4 * 7 + 4 * 7, b.time());
    assert_eq!(GridOrientation::West, b.player_state(0).orientation);

    assert_eq!("log[line1]", &b.log_writer().log_datas[2].1);
    assert_eq!(
        "log[line3 _next line should not be allowed__foo!]",
        &b.log_writer().log_datas[8].1
    );
    assert_eq!(
        MAX_LOG_LINE_LENGTH + "log[]".len(),
        b.log_writer().log_datas[5].1.len()
    ); // check truncation
    assert_eq!("log[---next print will be muted and penalized with game time unless a valid game comand called---]", &b.log_writer().log_datas[17].1);
    assert_eq!(60, b.log_writer().log_datas.len());
}

#[test]
fn test_print_convert() {
    let map = GridBattleMap::new(2, 2, SimpleTileType::Nothin, SimpleTileType::Nothin);
    let logger = VecLogWriter::new();
    let mut b = GridBattle::new(
        SimpleBattleLogic::new(
            map,
            TestTrivialLogic {},
            GridMapProber {},
            SimpleBattleObjectLayer::new(),
            FnCommandTimer::new(|com| match com {
                PlayerCommand::Print(_) => 0,
                _ => 10,
            }),
            0,
        ),
        vec![(
            new_player(0, 0, GridOrientation::South, 0, 1, "player1"),
            "\
            print('foo', 1, 3.3)\n\
            "
            .to_owned(),
        )],
        logger,
    );
    b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();

    assert_eq!(0, b.time());
    assert_eq!("log[foo 1 3.3]", &b.log_writer().log_datas[2].1);
}

#[test]
fn test_rand() {
    // run 2 sims - rand results must be identical
    let mut bs = Vec::with_capacity(3);
    for i in 0..3 {
        let map = GridBattleMap::new(2, 2, SimpleTileType::Nothin, SimpleTileType::Nothin);
        let logger = VecLogWriter::new();
        let mut b = GridBattle::new(
            SimpleBattleLogic::new(
                map,
                TestTrivialLogic {},
                GridMapProber {},
                SimpleBattleObjectLayer::new(),
                FnCommandTimer::new(|com| match com {
                    PlayerCommand::Print(_) => 0,
                    _ => 10,
                }),
                0,
            ),
            vec![(
                new_player(0, 0, GridOrientation::South, 0, 1, "player1"),
                "\
                r1 = rand()\n\
                r2 = rand()\n\
                r3 = rand()\n\
                r4 = rand()\n\
                r5 = rand()\n\
                print(r1, r2, r3, r4, r5)\n\
                "
                .to_owned()
                    + if i == 2 { "pass\n" } else { "" }, // program for 3rd sim has different text - so should do different rand
            )],
            logger,
        );
        bs.push(b);
    }
    bs[0].run_simulation();
    bs[1].run_simulation();
    bs[2].run_simulation();
    println!("BATTLE LOG1:");
    bs[0].log_writer().print();
    println!("BATTLE LOG2:");
    bs[1].log_writer().print();
    println!("BATTLE LOG3:");
    bs[2].log_writer().print();

    let mut valss = Vec::with_capacity(3);
    for b in bs {
        let line_len = b.log_writer().log_datas[2].1.len();
        let vals: Vec<f64> = b.log_writer().log_datas[2].1[4..line_len - 1]
            .split(' ')
            .map(|x| -> f64 { x.parse().unwrap() })
            .collect();
        // assert that all valuse are different
        for i in 1..vals.len() {
            assert_ne!(vals[i], vals[i - 1]);
        }
        assert!(vals.iter().fold(999_f64, |a, b| a.min(*b)) >= 0_f64);
        assert!(vals.iter().fold(-999_f64, |a, b| a.max(*b)) < 1_f64);
        valss.push(vals);
    }
    // assert that sims with same parameters yield same rand results
    assert!(valss[0].iter().zip(valss[1].iter()).all(|(a, b)| *a == *b));
    // assert  sims with different programs yield different rand results
    assert!(valss[1].iter().zip(valss[2].iter()).all(|(a, b)| *a != *b));
}

#[test]
fn test_2players_shoot_win_stop() {
    let map = GridBattleMap::new(3, 3, SimpleTileType::Nothin, SimpleTileType::Nothin);
    let logger = VecLogWriter::new();
    let mut b = GridBattle::new(
        SimpleBattleLogic::new(
            map,
            TestTrivialLogic {},
            GridMapProber {},
            SimpleBattleObjectLayer::new(),
            FnCommandTimer::new(|com| match com {
                PlayerCommand::TurnCW => 10,
                PlayerCommand::MoveFwd => 20,
                PlayerCommand::Shoot => 5,
                PlayerCommand::Print(_) => 0,
                _ => 10,
            }),
            1,
        ),
        vec![
            (
                new_player(1, 1, GridOrientation::West, 1, 1, "player1"),
                "\
while True:\n
    move_forward()\n
\n\
                "
                .to_owned(),
            ),
            (
                new_player(5, 1, GridOrientation::West, 1, 1, "player2"),
                "\
while True:\n
    shoot()\n
\n\
                "
                .to_owned(),
            ),
        ],
        logger,
    );
    b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();

    assert!(b.is_player_dead(0));
    assert!(!b.is_player_dead(1));

    let log_lines = &b.log_writer().log_datas;
    assert!(8 == log_lines.len());
    assert!(log_lines[0].1.starts_with("spawn"));
    assert!(log_lines[1].1.starts_with("spawn"));
    assert!(
        log_lines[2].1.starts_with("-move-forward") && log_lines[3].1.starts_with("-shoot")
            || log_lines[3].1.starts_with("-move-forward") && log_lines[2].1.starts_with("-shoot")
    );
    assert!(log_lines[4].1.starts_with("shoot"));
    assert!(log_lines[5].1.starts_with("+shoot"));
    assert!(log_lines[6].1.starts_with("die"));
    assert!(log_lines[7].1.starts_with("win"));
}
