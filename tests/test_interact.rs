use battle_sim::gametime::GameTime;
use battle_sim::map::MapWriteAccess;
use battle_sim::object_layer::ObjectLayer;
use battle_sim::r#impl::grid_battle::{new_player, GridBattle, GridPlayerState};
use battle_sim::r#impl::grid_map::GridBattleMap;
use battle_sim::r#impl::grid_map_prober::GridMapProber;
use battle_sim::r#impl::grid_orientation::GridOrientation;
use battle_sim::r#impl::simple_battle_logic::{PlayerCommand, SimpleBattleLogic};
use battle_sim::r#impl::simple_battle_object_layer::SimpleBattleObjectLayer;
use battle_sim::r#impl::simple_object::SimpleObject;

use std::collections::HashMap;

mod common;
use common::{HashmapCommandTimer, SimpleTileType, TestSimpleLogic, VecLogWriter};

fn test_base<F>(
    player_programs: Vec<(GridPlayerState, String)>,
    checks: F,
    player_count_to_win: usize,
    live_with_no_hp_time: GameTime,
) where
    F: FnOnce(
        GridBattle<
            SimpleBattleLogic<
                SimpleTileType,
                GridBattleMap<SimpleTileType>,
                TestSimpleLogic,
                GridMapProber,
                GridOrientation,
                SimpleBattleObjectLayer<SimpleObject<GridOrientation>>,
                HashmapCommandTimer<PlayerCommand<GridOrientation>>,
            >,
            VecLogWriter<String, String>,
        >,
        Option<Vec<usize>>,
    ),
{
    let mut map = GridBattleMap::new(10, 10, SimpleTileType::Nothin, SimpleTileType::Nothin);
    map.set_tile_at(0, 0, SimpleTileType::Wall);
    map.set_tile_at(1, 1, SimpleTileType::Wall);
    map.set_tile_at(2, 2, SimpleTileType::Wall);
    map.set_tile_at(3, 3, SimpleTileType::Wall);
    map.set_tile_at(4, 4, SimpleTileType::Wall);

    let logger = VecLogWriter::new();
    let mut b = GridBattle::new(
        SimpleBattleLogic::new(
            map,
            TestSimpleLogic {},
            GridMapProber {},
            SimpleBattleObjectLayer::new(),
            HashmapCommandTimer::new(
                HashMap::from([
                    (PlayerCommand::TurnCW, 5),
                    (PlayerCommand::TurnCCW, 5),
                    (PlayerCommand::MoveFwd, 10),
                    (PlayerCommand::Look(GridOrientation::North), 5),
                    (PlayerCommand::Look(GridOrientation::East), 5),
                    (PlayerCommand::Look(GridOrientation::South), 5),
                    (PlayerCommand::Look(GridOrientation::West), 5),
                    (PlayerCommand::Shoot, 10), // don't forget that successful shoot adds 3*5 wait
                    (PlayerCommand::AfterShootCooldown, 15),
                    (PlayerCommand::Wait, 5),
                    (PlayerCommand::CheckAmmo, 2),
                    (PlayerCommand::CheckHealth, 2),
                    (PlayerCommand::CheckHit, 2),
                ]),
                HashMap::from([
                    (PlayerCommand::TurnCW, 5),
                    (PlayerCommand::TurnCCW, 5),
                    (PlayerCommand::MoveFwd, 10),
                ]),
                0,
                0,
            ),
            player_count_to_win,
            live_with_no_hp_time,
        ),
        player_programs,
        logger,
    );
    let winners = b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();
    checks(b, winners);
}

#[test]
fn test_look_out_bounds() {
    test_base(
        vec![
            (
                new_player(20, 10, GridOrientation::South, 0, 1, "player1"),
                "\
print('hell-o')\n
turn_cw()\n
print('foo')\n
turn_cw()\n
print('yeah!')\n
            "
                .to_owned(),
            ),
            (
                new_player(20, 5, GridOrientation::East, 0, 1, "player2"),
                "\
print('second')\n
looked = look('right')\n
print(looked)\n
print('second yeah!')\n
if looked[-1][1].startswith('player'):\n
    print('oh no, see player')\n
    move_forward()\n
\
            "
                .to_owned(),
            ),
        ],
        |b, _| {
            assert_eq!(25, b.time());
            assert_eq!(21, b.player_state(1).col);
            assert_eq!(5, b.player_state(1).row);
        },
        0,
        0,
    );
}

#[test]
fn test_look_on_map_right() {
    test_base(
        vec![
            (
                new_player(4, 7, GridOrientation::East, 0, 1, "player1"),
                "\
wait()\n
move_forward()\n
print('yeah!')\n
            "
                .to_owned(),
            ),
            (
                new_player(4, 2, GridOrientation::East, 0, 1, "player2"),
                "\
print('second')\n
looked = look('right')\n
print(looked)\n
print('second yeah!')\n
if looked == [('empty_tile', None), ('wall', None)]:\n
    print('wall')\n
    move_forward()\n
    looked = look('right')\n
    print(looked)\n
    if looked == [('empty_tile', None)]*4 + [('empty_tile', 'player[player1][left-side]')]:\n
        move_forward()\n
\
            "
                .to_owned(),
            ),
        ],
        |b, _| {
            assert_eq!(50, b.time());
            assert_eq!(6, b.player_state(1).col);
            assert_eq!(2, b.player_state(1).row);
        },
        0,
        0,
    );
}

#[test]
fn test_look_on_map_left() {
    test_base(
        vec![
            (
                new_player(4, 1, GridOrientation::East, 0, 1, "player1"),
                "\
wait()\n
move_forward()\n
print('yeah!')\n
            "
                .to_owned(),
            ),
            (
                new_player(4, 6, GridOrientation::East, 0, 1, "player2"),
                "\
print('second')\n
looked = look('left')\n
print(looked)\n
print('second yeah!')\n
if looked == [('empty_tile', None), ('wall', None)]:\n
    print('wall')\n
    move_forward()\n
    looked = look('left')\n
    print(looked)\n
    if looked == [('empty_tile', None)]*4 + [('empty_tile', 'player[player1][right-side]')]:\n
        move_forward()\n
\
            "
                .to_owned(),
            ),
        ],
        |b, _| {
            assert_eq!(50, b.time());
            assert_eq!(6, b.player_state(1).col);
            assert_eq!(6, b.player_state(1).row);
        },
        0,
        0,
    );
}

#[test]
fn test_check_hit_dir() {
    test_base(
        vec![
            (
                new_player(20, 7, GridOrientation::South, 2, 10, "player1"),
                "\
start_hit = check_hit()\n
wait()\n
wait()\n
mid_hit = check_hit()\n
turn_cw()\n
wait()\n
wait()\n
wait()\n
end_hit = check_hit()\n
same_end_hit = check_hit()\n
print(start_hit, mid_hit, end_hit, same_end_hit)\n
if start_hit is None and mid_hit == 'left' and end_hit == 'back' and same_end_hit is None:\n
    move_forward()\n
            "
                .to_owned(),
            ),
            (
                new_player(30, 7, GridOrientation::West, 2, 10, "player2"),
                "\
shoot()\n
shoot()\n
            "
                .to_owned(),
            ),
        ],
        |b, _| {
            assert_eq!(19, b.player_state(0).col); // it starts looking south, then turns cw
            assert_eq!(7, b.player_state(0).row);
        },
        0,
        0,
    );
}

#[test]
fn test_check_health_ammo() {
    test_base(
        vec![
            (
                new_player(20, 7, GridOrientation::South, 2, 10, "player1"),
                "\
start_health = check_health()\n
wait()\n
wait()\n
end_health = check_health()\n
if start_health == 10 and end_health == 9:\n
    move_forward()\n
            "
                .to_owned(),
            ),
            (
                new_player(30, 7, GridOrientation::West, 2, 10, "player2"),
                "\
start_ammo = check_ammo()\n
shoot()\n
end_ammo = check_ammo()\n
if start_ammo == 2 and end_ammo == 1:\n
    move_forward()\n
            "
                .to_owned(),
            ),
        ],
        |b, _| {
            assert_eq!(20, b.player_state(0).col);
            assert_eq!(8, b.player_state(0).row);
            assert_eq!(29, b.player_state(1).col);
            assert_eq!(7, b.player_state(1).row);
        },
        0,
        0,
    );
}

#[test]
fn test_hear() {
    // TODO: not all edge cases covered (diagonals)
    for ((x, y), ori2, res) in [
        ((25, 2), GridOrientation::North, "front-right-along"),
        ((24, 1), GridOrientation::North, "front-right-along"),
        ((26, 10), GridOrientation::North, "back-right-along"),
        ((25, 5), GridOrientation::North, "back-right-side"),
        ((22, 6), GridOrientation::North, "back-left-side"),
        ((24, 9), GridOrientation::North, "back-left-along"),
        ((20, -4), GridOrientation::North, "front-left-along"),
        ((23, 5), GridOrientation::North, "front-left-side"),
        //
        ((26, 8), GridOrientation::East, "front-right-side"),
        ((27, 5), GridOrientation::East, "front-right-along"),
        ((22, 6), GridOrientation::East, "back-right-along"),
        ((24, 9), GridOrientation::East, "back-right-side"),
        ((21, 4), GridOrientation::East, "back-left-along"),
        ((13, 5), GridOrientation::East, "back-left-along"),
        ((27, 2), GridOrientation::East, "front-left-along"),
        ((24, 1), GridOrientation::East, "front-left-side"),
        //
        ((23, 8), GridOrientation::South, "front-right-along"),
        ((24, 9), GridOrientation::South, "front-right-along"),
        ((21, 1), GridOrientation::South, "back-right-along"),
        ((22, 5), GridOrientation::South, "back-right-side"),
        ((35, 1), GridOrientation::South, "back-left-side"),
        ((24, -1), GridOrientation::South, "back-left-along"),
        ((30, 8), GridOrientation::South, "front-left-side"),
        ((25, 5), GridOrientation::South, "front-left-side"),
        //
        ((21, 2), GridOrientation::West, "front-right-side"),
        ((19, 5), GridOrientation::West, "front-right-along"),
        ((28, 3), GridOrientation::West, "back-right-along"),
        ((24, 0), GridOrientation::West, "back-right-side"),
        ((32, 11), GridOrientation::West, "back-left-along"),
        ((33, 5), GridOrientation::West, "back-left-along"),
        ((17, 11), GridOrientation::West, "front-left-along"),
        ((24, 15), GridOrientation::West, "front-left-side"),
    ] {
        test_base(
            vec![
                (
                    new_player(x, y, GridOrientation::East, 0, 1, "player1"),
                    "\
wait()\n
                "
                    .to_owned(),
                ),
                (
                    new_player(24, 5, ori2, 0, 1, "player2"),
                    format!(
                        "\
res = listen()\n
assert(len(res) == 1)\n
print('{res} - expecting')
print(res)\n
if res[0] == '{res}':\n
    move_forward()\n
                "
                    ),
                ),
            ],
            |b, _| {
                let fwd = match ori2 {
                    GridOrientation::North => (24, 4),
                    GridOrientation::East => (25, 5),
                    GridOrientation::South => (24, 6),
                    GridOrientation::West => (23, 5),
                };
                assert_eq!(fwd.0, b.player_state(1).col);
                assert_eq!(fwd.1, b.player_state(1).row);
            },
            0,
            0,
        );
    }
}

#[test]
fn test_move_timings1() {
    // here we check hits, next one - misses
    test_base(
        vec![
            (
                new_player(20, 7, GridOrientation::South, 1, 10, "player1"),
                "\
check_ammo() # 2\n
move_forward() # 10+10 =22\n
if check_hit() == 'left': # 2 =24\n
    move_forward() # 10+10 =44\n
    for _ in range(30):\n
        assert check_hit() == None # 30*2 =104\n
    move_forward() # 10+10 = 124\n
    if check_hit() == 'left' and check_health() == 8:\n
        move_forward()\n
            "
                .to_owned(),
            ),
            (
                new_player(30, 7, GridOrientation::West, 2, 1, "player2"),
                "\
shoot() # 10+15\n
turn_ccw() # 5+5 =35\n
move_forward() # 10+10 = 55\n
move_forward() # 10+10 = 75\n
move_forward() # 10+10 = 95\n
turn_cw() # 5+5 =105\n
shoot() # 10 =115\n
            "
                .to_owned(),
            ),
        ],
        |b, _| {
            assert_eq!(20, b.player_state(0).col);
            assert_eq!(11, b.player_state(0).row);
        },
        0,
        0,
    );
}

#[test]
fn test_move_timings2() {
    // here we check misses, prev one - hits
    test_base(
        vec![
            (
                new_player(20, 7, GridOrientation::South, 1, 1, "player1"),
                "\
move_forward() # 10+10 =20\n
if check_hit() == None: # 2 =22\n
    move_forward() # 10+10 =42\n
    for _ in range(33):\n
        assert check_hit() == None # 33*2 =108\n
    move_forward() # 10+10 =128\n
    if check_hit() == None and check_health() == 1:\n
        move_forward()\n
            "
                .to_owned(),
            ),
            (
                new_player(30, 7, GridOrientation::West, 2, 1, "player2"),
                "\
check_ammo() # 2\n
shoot() # 10+15 = 27\n
turn_ccw() # 5+5 =37\n
move_forward() # 10+10 = 57\n
move_forward() # 10+10 = 77\n
move_forward() # 10+10 = 97\n
turn_cw() # 5+5 =107\n
shoot() # 10 =117\n
            "
                .to_owned(),
            ),
        ],
        |b, _| {
            assert_eq!(20, b.player_state(0).col);
            assert_eq!(11, b.player_state(0).row);
        },
        0,
        0,
    );
}

#[test]
fn test_turn_cw_timings1() {
    // here we check hits, next one - misses
    test_base(
        vec![
            (
                new_player(20, 7, GridOrientation::South, 1, 10, "player1"),
                "\
wait() # 5\n
check_ammo() # 2 =7\n
turn_cw() # 5+5 =17\n
if check_hit() == 'left': # 2 =19\n
    move_forward() # 10+10 =39\n
    turn_cw() # 5+5 =49\n
    if check_hit() == 'right' and check_health() == 8:\n
        move_forward()\n
            "
                .to_owned(),
            ),
            (
                new_player(30, 7, GridOrientation::West, 2, 1, "player2"),
                "\
shoot() # 10+15\n
wait() # 5 =30\n
wait() # 5 =35\n
shoot() # 10 =45\n
            "
                .to_owned(),
            ),
        ],
        |b, _| {
            assert_eq!(19, b.player_state(0).col);
            assert_eq!(6, b.player_state(0).row);
        },
        0,
        0,
    );
}

#[test]
fn test_turn_cw_timings2() {
    // here we check misses, prev one - hits
    test_base(
        vec![
            (
                new_player(20, 7, GridOrientation::South, 1, 10, "player1"),
                "\
check_ammo() # 2 =2\n
check_ammo() # 2 =4\n
turn_cw() # 5+5 =14\n
if check_hit() == 'back': # 2 =16\n
    move_forward() # 10+10 =36\n
    turn_cw() # 5+5 =46\n
    if check_hit() == 'back' and check_health() == 8:\n
        move_forward()\n
            "
                .to_owned(),
            ),
            (
                new_player(30, 7, GridOrientation::West, 2, 1, "player2"),
                "\
shoot() # 10+15\n
wait() # 5 =30\n
shoot() # 10 =40\n
            "
                .to_owned(),
            ),
        ],
        |b, _| {
            assert_eq!(19, b.player_state(0).col);
            assert_eq!(6, b.player_state(0).row);
        },
        0,
        0,
    );
}

///
/// test of the simplest case of delayed death
#[test]
fn test_delayed_death_simple() {
    test_base(
        vec![
            (
                new_player(20, 7, GridOrientation::West, 100, 1, "player1"),
                "
while True:
    move_forward()
                "
                .to_owned(),
            ),
            (
                new_player(25, 7, GridOrientation::West, 100, 1, "player2"),
                "
shoot()
                "
                .to_owned(),
            ),
        ],
        |b, _| {
            assert_eq!(37, b.time());
            // ensure that "dying" player was able to make extra move
            assert_eq!(18, b.player_state(0).col);
        },
        1,
        27,
    );
}

///
/// this test tests that multiple shots into the dying player does not reset dying timers
#[test]
fn test_delayed_death_double_shot() {
    test_base(
        vec![
            (
                new_player(20, 7, GridOrientation::West, 100, 1, "player1"),
                "
while True:
    move_forward()
                "
                .to_owned(),
            ),
            (
                new_player(25, 7, GridOrientation::West, 100, 1, "player2"),
                "
shoot()
shoot()
                "
                .to_owned(),
            ),
        ],
        |b, _| {
            assert_eq!(53, b.time());
            // ensure that "dying" player was able to make extra move
            assert_eq!(17, b.player_state(0).col);
        },
        1,
        43,
    );
}

///
/// delayed death, mirror case
#[test]
fn test_delayed_death_mirror() {
    test_base(
        vec![
            (
                new_player(20, 7, GridOrientation::East, 100, 1, "player1"),
                "
shoot()
move_forward()
                "
                .to_owned(),
            ),
            (
                new_player(30, 7, GridOrientation::West, 100, 1, "player2"),
                "
shoot()
move_forward()
                "
                .to_owned(),
            ),
        ],
        |b, winners| {
            assert_eq!(39, b.time());
            // ensure that "dying" player was able to make extra move
            assert_eq!(21, b.player_state(0).col);
            assert_eq!(29, b.player_state(1).col);

            // there should not be a "win" log anywhere
            for log_data in b.log_writer().log_datas.iter() {
                assert_ne!("win", log_data.1);
            }

            // there must be NO winners
            assert_eq!(Some(vec![]), winners);
        },
        1,
        29,
    );
}

///
/// delayed death, delayed mirror case, where one already dead, another still dying
#[test]
fn test_delayed_death_delayed_mirror() {
    test_base(
        vec![
            (
                new_player(20, 7, GridOrientation::East, 100, 1, "player1"),
                "
wait()
wait()
wait()
shoot()
move_forward()
                "
                .to_owned(),
            ),
            (
                new_player(30, 7, GridOrientation::West, 100, 1, "player2"),
                "
shoot()
move_forward()
move_forward()
move_forward()
                "
                .to_owned(),
            ),
        ],
        |b, winners| {
            assert_eq!(54, b.time());
            // ensure that second "dying" player was able to make extra move, first had no time
            assert_eq!(20, b.player_state(0).col);
            assert_eq!(29, b.player_state(1).col);

            // there should not be a "win" log anywhere
            for log_data in b.log_writer().log_datas.iter() {
                assert_ne!("win", log_data.1);
            }

            // there must be NO winners
            assert_eq!(Some(vec![]), winners);
        },
        1,
        29,
    );
}

///
/// delayed death, delayed mirror case, same as prev test (test_delayed_death_delayed_mirror)
/// BUT here player program ends BEFORE death event happens
#[test]
fn test_delayed_death_delayed_mirror_prog_end() {
    test_base(
        vec![
            (
                new_player(20, 7, GridOrientation::East, 100, 1, "player1"),
                "
wait()
wait()
wait()
shoot()
move_forward()
                "
                .to_owned(),
            ),
            (
                new_player(30, 7, GridOrientation::West, 100, 1, "player2"),
                "
shoot()
move_forward()
                "
                .to_owned(),
            ),
        ],
        |b, winners| {
            // current logic is to NOT wait for events,
            // instead quit as soon as all programs end,
            // therefore player2 will be left dying, noone wins
            assert_eq!(45, b.time());
            // ensure that second "dying" player was able to make extra move, first had no time
            assert_eq!(20, b.player_state(0).col);
            assert_eq!(29, b.player_state(1).col);

            // there should not be a "win" log anywhere
            for log_data in b.log_writer().log_datas.iter() {
                assert_ne!("win", log_data.1);
            }

            // there must be NO winners
            assert_eq!(Some(vec![]), winners);
        },
        1,
        29,
    );
}
