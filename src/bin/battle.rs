use battle_sim::gametime::GameTime;
use battle_sim::map::MapReadAccess;
use battle_sim::maptile_logic::MaptileLogic;
use battle_sim::object_layer::ObjectLayer;
use battle_sim::r#impl::buf_battle_logger::BufferLogWriter;
use battle_sim::r#impl::grid_battle::{new_player, GridBattle};
use battle_sim::r#impl::grid_map::GridBattleMap;
use battle_sim::r#impl::grid_map_prober::GridMapProber;
use battle_sim::r#impl::grid_orientation::GridOrientation;
use battle_sim::r#impl::simple_battle_logic::PlayerCommand;
use battle_sim::r#impl::simple_battle_logic::{CommandTimer, SimpleBattleLogic};
use battle_sim::r#impl::simple_battle_object_layer::SimpleBattleObjectLayer;
use battle_sim::r#impl::simple_object::{ObjectCacheType, SimpleObject};
use battle_sim::r#impl::tile_types_logic::TileTypeLogic;
use battle_sim::serialization::FromFile;

use rand::prelude::*;
use std::env::args;
use std::fs::File;
use std::io::{self, stdout, Error, ErrorKind, Read, Result, Write};
use std::path::PathBuf;
use std::process::ExitCode;

struct CommandTimings {}

impl CommandTimer<PlayerCommand<GridOrientation>> for CommandTimings {
    fn get_base_duration(
        &self,
        command: &PlayerCommand<GridOrientation>,
    ) -> battle_sim::gametime::GameTime {
        match command {
            PlayerCommand::MoveFwd => 5, // half, half after
            PlayerCommand::TurnCW => 8,  // half, half after
            PlayerCommand::TurnCCW => 8, // half, half after
            PlayerCommand::Shoot => 5,
            PlayerCommand::AfterShootCooldown => 20,
            PlayerCommand::ShotHitSound => 30,
            PlayerCommand::Look(_) => 4,
            PlayerCommand::Listen => 3, // start listening fast, delay reply by long
            PlayerCommand::Wait => 5,
            PlayerCommand::AddAmmo(_) => 2,
            PlayerCommand::AddHealth(_) => 2,
            PlayerCommand::CheckAmmo => 2,
            PlayerCommand::CheckHealth => 2,
            PlayerCommand::CheckHit => 2,
            PlayerCommand::ResetHit => 1,
            PlayerCommand::Print(_) => 0,
            PlayerCommand::Time => 0,
        }
    }
    fn get_reply_delay(
        &self,
        command: &PlayerCommand<GridOrientation>,
    ) -> battle_sim::gametime::GameTime {
        match command {
            PlayerCommand::MoveFwd => 5,
            PlayerCommand::TurnCW => 8,
            PlayerCommand::TurnCCW => 8,
            PlayerCommand::Shoot => 5,
            PlayerCommand::Listen => 12,
            _ => 0,
        }
    }
}

struct Config {
    map_path: PathBuf,
    player_programs: Vec<PathBuf>,
    log_path: Option<PathBuf>,
    time_limit: Option<GameTime>,
}

fn main() -> ExitCode {
    let config = match parse_args() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Error parsing arguments: {e}");
            return ExitCode::from(2);
        }
    };

    let map = match GridBattleMap::load_from_file(&config.map_path) {
        Ok(x) => x,
        Err(e) => {
            eprintln!(
                "failed to load map at '{}': {}",
                config.map_path.to_string_lossy(),
                e
            );
            return ExitCode::from(1);
        }
    };
    let map_logic = TileTypeLogic::new();

    let logger = BufferLogWriter::new(io::BufWriter::new({
        if let Some(path) = config.log_path {
            if let Ok(file) = File::create(path) {
                Box::new(file) as Box<dyn Write>
            } else {
                eprintln!("failed to create battle log file");
                return ExitCode::from(1);
            }
        } else {
            Box::new(stdout()) as Box<dyn Write>
        }
    }));

    let mut player_initial_data = Vec::with_capacity(config.player_programs.len());
    let player_initial_placements = match map.get_spawn_locations(config.player_programs.len()) {
        Ok(x) => x,
        Err(_) => {
            eprintln!("failed to generate spawn locations for all playes on the given map");
            return ExitCode::from(1);
        }
    };
    for (player_program_file, (x, y, ori)) in
        config.player_programs.iter().zip(player_initial_placements)
    {
        let mut file = match std::fs::File::open(&player_program_file) {
            Ok(x) => x,
            Err(e) => {
                eprintln!(
                    "failed to open player program file: '{}': {}",
                    player_program_file.to_string_lossy(),
                    e
                );
                return ExitCode::from(1);
            }
        };
        let mut player_program = String::new();
        if let Err(e) = file.read_to_string(&mut player_program) {
            eprintln!(
                "failed to read player program from file '{}': {}",
                player_program_file.to_string_lossy(),
                e
            );
            return ExitCode::from(1);
        }

        let name: &str = player_program_file
            .file_stem()
            .map(|x| x.to_str().unwrap_or("player"))
            .unwrap_or("player");
        player_initial_data.push((new_player(x, y, ori, 5, 5, name), player_program));
    }

    let mut object_layer = SimpleBattleObjectLayer::new();
    {
        let mut rng = StdRng::seed_from_u64(1234567);
        let ammocrates_count =
            1.max(map.map_data().row_count() * map.map_data().row(0).len() / 100);
        for _ in 0..ammocrates_count {
            for _ in 0..100 {
                let y = rng.gen_range(0..map.map_data().row_count());
                let x = rng.gen_range(0..map.map_data().row(y).len());
                let x = x as i64;
                let y = y as i64;
                if !map_logic.passable(map.get_tile_at(x, y))
                    || object_layer.objects_at(x, y).len() > 0
                {
                    continue;
                }
                object_layer.add(SimpleObject::new(
                    x,
                    y,
                    GridOrientation::North,
                    ObjectCacheType::AmmoCrate(17),
                    false,
                    true,
                    false,
                ));
                break;
            }
        }
    }

    let game_logic = SimpleBattleLogic::new(
        map,
        map_logic,
        GridMapProber::new(),
        object_layer,
        CommandTimings {},
        1,
        30,
    );
    let mut battle = GridBattle::new(game_logic, player_initial_data, logger);
    let winners = battle.run_simulation_with_time_limit(config.time_limit);

    match winners {
        Some(winner_ids) if winner_ids.len() > 0 => {
            println!(
                "WINNERS:{}",
                winner_ids
                    .iter()
                    .map(|&x| { x.to_string() })
                    .collect::<Vec<String>>()
                    .join(",")
            );
        }
        _ => {
            println!("DRAW");
        }
    }

    ExitCode::SUCCESS
}

enum ArgsState {
    FlagOrMapPath,
    PlayerProgram,
    GameTimeLimit,
    PlayerProgramOrDone,
    BattleLogPath,
}

fn parse_args() -> Result<Config> {
    let mut state = ArgsState::FlagOrMapPath;
    let mut config = Config {
        map_path: PathBuf::new(),
        player_programs: Vec::new(),
        log_path: None,
        time_limit: None,
    };

    let args = args().skip(1);
    for arg in args {
        match state {
            ArgsState::FlagOrMapPath => match arg.as_str() {
                "-o" | "--output" => {
                    state = ArgsState::BattleLogPath;
                    continue;
                }
                "-l" | "--time-limit" => {
                    state = ArgsState::GameTimeLimit;
                    continue;
                }
                arg => {
                    config.map_path = PathBuf::from(arg);
                    state = ArgsState::PlayerProgram;
                }
            },
            ArgsState::PlayerProgram | ArgsState::PlayerProgramOrDone => {
                config.player_programs.push(PathBuf::from(arg));
                state = ArgsState::PlayerProgramOrDone;
            }
            ArgsState::BattleLogPath => {
                config.log_path = Some(PathBuf::from(arg));
                state = ArgsState::FlagOrMapPath;
            }
            ArgsState::GameTimeLimit => {
                config.time_limit = Some(if let Ok(x) = u64::from_str_radix(&arg, 10) {
                    x
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "invalid data for time limit",
                    ));
                });
                state = ArgsState::FlagOrMapPath;
            }
        }
    }

    if let ArgsState::PlayerProgramOrDone = state {
        Ok(config)
    } else {
        Err(Error::new(
            ErrorKind::InvalidData,
            "not all arguments provided",
        ))
    }
}
