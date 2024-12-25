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

fn test_base<F>(player_programs: Vec<(GridPlayerState, String)>, checks: F)
where
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
                    (PlayerCommand::TurnCW, 10),
                    (PlayerCommand::TurnCCW, 10),
                    (PlayerCommand::MoveFwd, 20),
                    (PlayerCommand::Look(GridOrientation::North), 5),
                    (PlayerCommand::Look(GridOrientation::East), 5),
                    (PlayerCommand::Look(GridOrientation::South), 5),
                    (PlayerCommand::Look(GridOrientation::West), 5),
                    (PlayerCommand::Shoot, 10), // don't forget that successful shoot adds 3 waits
                    (PlayerCommand::Wait, 5),
                    (PlayerCommand::CheckAmmo, 2),
                    (PlayerCommand::CheckHealth, 2),
                    (PlayerCommand::CheckHit, 2),
                ]),
                0,
            ),
            0,
        ),
        player_programs,
        logger,
    );
    b.run_simulation();
    println!("BATTLE LOG:");
    b.log_writer().print();
    checks(b);
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
        |b| {
            assert_eq!(25, b.time());
            assert_eq!(21, b.player_state(1).col);
            assert_eq!(5, b.player_state(1).row);
        },
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
        |b| {
            assert_eq!(50, b.time());
            assert_eq!(6, b.player_state(1).col);
            assert_eq!(2, b.player_state(1).row);
        },
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
        |b| {
            assert_eq!(50, b.time());
            assert_eq!(6, b.player_state(1).col);
            assert_eq!(6, b.player_state(1).row);
        },
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
        |b| {
            assert_eq!(19, b.player_state(0).col); // it starts looking south, then turns cw
            assert_eq!(7, b.player_state(0).row);
        },
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
        |b| {
            assert_eq!(20, b.player_state(0).col);
            assert_eq!(8, b.player_state(0).row);
            assert_eq!(29, b.player_state(1).col);
            assert_eq!(7, b.player_state(1).row);
        },
    );
}

#[test]
fn test_hear() {
    for ((x, y), ori2, res) in [
        ((25, 2), GridOrientation::North, "front-right"),
        ((24, 1), GridOrientation::North, "front-right"),
        ((26, 10), GridOrientation::North, "back-right"),
        ((25, 5), GridOrientation::North, "back-right"),
        ((22, 6), GridOrientation::North, "back-left"),
        ((24, 9), GridOrientation::North, "back-left"),
        ((20, -4), GridOrientation::North, "front-left"),
        ((23, 5), GridOrientation::North, "front-left"),

        ((26, 8), GridOrientation::East, "front-right"),
        ((27, 5), GridOrientation::East, "front-right"),
        ((22, 6), GridOrientation::East, "back-right"),
        ((24, 9), GridOrientation::East, "back-right"),
        ((21, 4), GridOrientation::East, "back-left"),
        ((13, 5), GridOrientation::East, "back-left"),
        ((27, 2), GridOrientation::East, "front-left"),
        ((24, 1), GridOrientation::East, "front-left"),

        ((23, 8), GridOrientation::South, "front-right"),
        ((24, 9), GridOrientation::South, "front-right"),
        ((21, 1), GridOrientation::South, "back-right"),
        ((22, 5), GridOrientation::South, "back-right"),
        ((35, 1), GridOrientation::South, "back-left"),
        ((24, -1), GridOrientation::South, "back-left"),
        ((30, 8), GridOrientation::South, "front-left"),
        ((25, 5), GridOrientation::South, "front-left"),

        ((21, 2), GridOrientation::West, "front-right"),
        ((19, 5), GridOrientation::West, "front-right"),
        ((28, 3), GridOrientation::West, "back-right"),
        ((24, 0), GridOrientation::West, "back-right"),
        ((32, 11), GridOrientation::West, "back-left"),
        ((33, 5), GridOrientation::West, "back-left"),
        ((17, 11), GridOrientation::West, "front-left"),
        ((24, 15), GridOrientation::West, "front-left"),
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
print(res)\n
if res[0] == '{}':\n
    move_forward()\n
                ",
                        res
                    ),
                ),
            ],
            |b| {
                let fwd = match ori2 {
                    GridOrientation::North => (24, 4),
                    GridOrientation::East => (25, 5),
                    GridOrientation::South => (24, 6),
                    GridOrientation::West => (23, 5),
                };
                assert_eq!(fwd.0, b.player_state(1).col);
                assert_eq!(fwd.1, b.player_state(1).row);
            },
        );
    }
}
