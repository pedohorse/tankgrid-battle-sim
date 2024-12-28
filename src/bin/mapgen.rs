use battle_sim::r#impl::grid_map::GridBattleMap;
use battle_sim::r#impl::tile_types::TileType;
use battle_sim::map::MapWriteAccess;
use battle_sim::serialization::ToFile;

use std::env::args;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;
use rand::prelude::*;

struct Config {
    width: usize,
    height: usize,
    is_opened: bool,
    seed: u64,
    out_path: Option<PathBuf>,
}

fn main() -> ExitCode {
    let config = match parse_args() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Error parsing arguments: {e}");
            return ExitCode::from(2);
        }
    };

    let mut map = GridBattleMap::new(
        config.width,
        config.height,
        TileType::Ground,
        if config.is_opened {
            TileType::Ground
        } else {
            TileType::Wall
        },
    );

    // some basic gen for now, nothing smart
    let mut rng = StdRng::seed_from_u64(config.seed);
    for _ in 0..(((config.height*config.width) as f64 * 0.07) as usize) {
        let x: i64 = rng.gen_range(0..config.width).try_into().unwrap();
        let y: i64 = rng.gen_range(0..config.height).try_into().unwrap();

        map.set_tile_at(x, y, TileType::Wall);
    }

    if let Some(path) = config.out_path {
        if let Err(e) = map.save_to_file(&path) {
            eprintln!("failed to save map to file: {}", e);
            return ExitCode::from(1);
        }
    } else {
        if let Err(e) = map.save_to_writer(std::io::stdout()) {
            eprintln!("failed to save map to stdout: {}", e);
            return ExitCode::from(1);
        }
    }
    

    ExitCode::SUCCESS
}

enum ArgsState {
    FlagOrOut,
    Seed,
    MapWidth,
    MapHeight,
    Nothing,
}

fn parse_args() -> Result<Config, Error> {
    let mut config = Config {
        width: 16,
        height: 16,
        is_opened: false,
        seed: 123456,
        out_path: None,
    };

    let args = args();

    let args = args.skip(1);

    let mut state = ArgsState::FlagOrOut;
    for arg in args {
        match state {
            ArgsState::FlagOrOut => match arg.as_str() {
                "-s" | "--size" => {
                    state = ArgsState::MapWidth;
                    continue;
                }
                "-r" | "--seed" => {
                    state = ArgsState::Seed;
                    continue;
                }
                "--opened" => {
                    config.is_opened = true;
                    continue;
                }
                s => {
                    config.out_path = Some(PathBuf::from_str(s).unwrap());
                    state = ArgsState::Nothing;
                }
            },
            ArgsState::MapWidth => {
                config.width = match usize::from_str_radix(&arg, 10) {
                    Ok(x) => x,
                    Err(e) => return Err(Error::new(ErrorKind::InvalidData, e)),
                };
                state = ArgsState::MapHeight;
            }
            ArgsState::MapHeight => {
                config.height = match usize::from_str_radix(&arg, 10) {
                    Ok(x) => x,
                    Err(e) => return Err(Error::new(ErrorKind::InvalidData, e)),
                };
                state = ArgsState::FlagOrOut;
            }
            ArgsState::Seed => {
                config.seed = match u64::from_str_radix(&arg, 10) {
                    Ok(x) => x,
                    Err(e) => return Err(Error::new(ErrorKind::InvalidData, e)),
                };
                state = ArgsState::FlagOrOut;
            }
            ArgsState::Nothing => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "not expecting any more arguments",
                ));
            }
        }
    }

    Ok(config)
}
