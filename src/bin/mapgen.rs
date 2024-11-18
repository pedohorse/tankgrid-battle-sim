use battle_sim::r#impl::grid_battle::GridBattle;

use std::env::args;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::str::FromStr;
use std::process::{ExitCode};

struct Config {
    width: usize,
    height: usize,
    out_path: Option<PathBuf>,
}

pub fn main() -> ExitCode {
    let config = match parse_args() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Error parsing arguments: {e}");
            return ExitCode::from(2);
        }
    };


    
    ExitCode::SUCCESS
}

enum ArgsState {
    FlagOrOut,
    MapWidth,
    MapHeight,
    Nothing,
}

fn parse_args() -> Result<Config, Error>{
    let mut config = Config {
        width: 16,
        height: 16,
        out_path: None,
    };

    let args = args();

    let args = args.skip(1);

    let mut state = ArgsState::FlagOrOut;
    for arg in args {
        match state {
            ArgsState::FlagOrOut => match arg.as_str() {
                "-s"|"--size" => {
                    state = ArgsState::MapWidth;
                    continue;
                }
                s => {
                    config.out_path = Some(PathBuf::from_str(s).unwrap());
                    state = ArgsState::Nothing;
                }
            }
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
            ArgsState::Nothing => {
                return Err(Error::new(ErrorKind::InvalidData, "not expecting any more arguments"));
            }
        }
    }
    
    Ok(config)
}