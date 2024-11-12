use std::{fs, io};

use super::grid_map::{GridBattleMap, FromFile};

use map_lib::MapData;
use serde::de::DeserializeOwned;
use serde_json;

impl<T> FromFile<T> for GridBattleMap<T>
where
    T: Copy + Clone + DeserializeOwned,
{
    fn load_from_file(path: &std::path::Path) -> io::Result<GridBattleMap<T>> {
        let data: MapData<T> = {
            let file = fs::File::open(path)?;
            match serde_json::from_reader(io::BufReader::new(file)) {
                Ok(x) => x,
                Err(e) => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, e));
                }
            }
        };

        match GridBattleMap::new_from_data(data) {
            Ok(x) => Ok(x),
            Err(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "data does not represent a grid map",
            )),
        }
    }
}

#[test]
fn todo() {}
