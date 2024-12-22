use std::{fs, io};

use super::grid_map::GridBattleMap;
use crate::serialization::{FromFile, ToFile};
use crate::map_data::MapData;

use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use serde_json;

impl<T> FromFile for GridBattleMap<T>
where
    T: Copy + Clone + DeserializeOwned,
{
    fn load_from_reader<R>(r: R) -> std::io::Result<Self>
    where
        R: io::Read,
    {
        let data: MapData<T> = {
            match serde_json::from_reader(r) {
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

impl<T> ToFile for GridBattleMap<T>
where
    T: Copy + Clone + Serialize,
{
    fn save_to_writer<W>(&self, w: W) -> std::io::Result<()>
    where
        W: io::Write,
    {
        match serde_json::to_writer(w, self.map_data()) {
            Ok(x) => Ok(x),
            Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
        }
    }
}
