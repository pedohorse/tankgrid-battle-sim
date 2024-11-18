use std::io::{BufReader, BufWriter, Empty};
use std::path::PathBuf;
use std::str::FromStr;

use battle_sim::r#impl::grid_map::GridBattleMap;
use battle_sim::map::MapWriteAccess;
use battle_sim::serialization::{FromFile, ToFile};
use serde::de::{self, Visitor, DeserializeOwned};
use serde::{Deserialize, Serialize};


#[derive(Clone, Copy, PartialEq, Debug)]
enum SimpleTile {
    Empty,
    Wall,
    Mud,
    Fire,
}

impl Serialize for SimpleTile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let c = match self {
            SimpleTile::Empty => 0,
            SimpleTile::Wall => 1,
            SimpleTile:: Mud => 2,
            SimpleTile::Fire => 3,
        };
        serializer.serialize_i8(c)
    }
}

struct TileVisitor;

impl<'de> Visitor<'de> for TileVisitor {
    type Value = i8;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Tile should be represented by a i8")
    }


    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(i8::from(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if let Ok(x) = i8::try_from(value) {
            Ok(x)
        } else {
            Err(de::Error::custom("bad tile value"))
        }
    }
}

impl<'de> Deserialize<'de> for SimpleTile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        match deserializer.deserialize_i8(TileVisitor) {
            Ok(0) => Ok(SimpleTile::Empty),
            Ok(1) => Ok(SimpleTile::Wall),
            Ok(2) => Ok(SimpleTile::Mud),
            Ok(3) => Ok(SimpleTile::Fire),
            Ok(_) => Err(de::Error::custom("there are no tiles with that val")),
            Err(e) => Err(e),
        }
    }
}

#[test]
fn test_simple_de() {
    let mut map = GridBattleMap::new(10, 10, SimpleTile::Empty, SimpleTile::Fire);

    map.set_tile_at(1, 2, SimpleTile::Fire);
    map.set_tile_at(2, 2, SimpleTile::Mud);
    map.set_tile_at(3, 6, SimpleTile::Wall);
    map.set_tile_at(3, 8, SimpleTile::Wall);
    map.set_tile_at(4, 9, SimpleTile::Fire);
    map.set_tile_at(5, 1, SimpleTile::Mud);
    map.set_tile_at(5, 4, SimpleTile::Wall);
    map.set_tile_at(6, 0, SimpleTile::Mud);
    map.set_tile_at(6, 9, SimpleTile::Wall);
    map.set_tile_at(7, 3, SimpleTile::Fire);
    map.set_tile_at(8, 7, SimpleTile::Wall);
    map.set_tile_at(9, 9, SimpleTile::Fire);

    let mut buf = Vec::new();
    map.save_to_writer(BufWriter::new(&mut buf)).unwrap();
    
    let map2: GridBattleMap<SimpleTile> = GridBattleMap::load_from_reader(BufReader::new(buf.as_slice())).unwrap();

    for row_i in 0..map.map_data().row_count() {
        for (i, val) in map.map_data().row(row_i).iter().enumerate() {
            assert_eq!(map2.map_data().row(row_i)[i], *val);
        }
    }
}
