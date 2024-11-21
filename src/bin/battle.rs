use battle_sim::object_layer::ObjectLayer;
use battle_sim::r#impl::buf_battle_logger::BufferLogWriter;
use battle_sim::r#impl::grid_battle::{GridBattle, GridPlayerState};
use battle_sim::r#impl::grid_map::GridBattleMap;
use battle_sim::r#impl::grid_map_prober::GridMapProber;
use battle_sim::r#impl::grid_orientation::GridOrientation;
use battle_sim::r#impl::simple_battle_logic::PlayerCommand;
use battle_sim::r#impl::simple_battle_logic::{CommandTimer, SimpleBattleLogic};
use battle_sim::r#impl::tile_types_logic::TileTypeLogic;
use battle_sim::r#impl::trivial_object_layer::TrivialObjectLayer;
use battle_sim::serialization::FromFile;

use std::collections::HashMap;
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
            PlayerCommand::MoveFwd => 10,
            PlayerCommand::TurnCW => 15,
            PlayerCommand::TurnCCW => 15,
            PlayerCommand::Shoot => 5,
            PlayerCommand::Look(_) => 4,
            PlayerCommand::Wait => 5,
            PlayerCommand::AddAmmo(_) => 2,
            PlayerCommand::AddHealth(_) => 2,
        }
    }
}

struct Config {
    map_path: PathBuf,
    player_programs: Vec<PathBuf>,
    log_path: Option<PathBuf>,
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

        player_initial_data.push((
            GridPlayerState::new(x, y, ori, 20, 1, "player"),
            player_program,
        ));
    }

    let game_logic = SimpleBattleLogic::new(
        map,
        TileTypeLogic::new(),
        GridMapProber::new(),
        TrivialObjectLayer::new(),
        CommandTimings {},
        1,
    );
    let mut battle = GridBattle::new(game_logic, player_initial_data, logger);
    battle.run_simulation();

    ExitCode::SUCCESS
}

enum ArgsState {
    FlagOrMapPath,
    PlayerProgram,
    PlayerProgramOrDone,
    BattleLogPath,
}

fn parse_args() -> Result<Config> {
    let mut state = ArgsState::FlagOrMapPath;
    let mut config = Config {
        map_path: PathBuf::new(),
        player_programs: Vec::new(),
        log_path: None,
    };

    let args = args().skip(1);
    for arg in args {
        match state {
            ArgsState::FlagOrMapPath => match arg.as_str() {
                "-o" | "--output" => {
                    state = ArgsState::BattleLogPath;
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
