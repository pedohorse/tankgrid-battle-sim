use serde::de::{self, Deserialize, Visitor};
use serde::ser::Serialize;

use super::tile_types::TileType;

impl Serialize for TileType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let c = match self {
            TileType::Ground => 0,
            TileType::Wall => 1,
            TileType::Mud => 2,
        };
        serializer.serialize_i8(c)
    }
}

struct TileVisitor;

impl<'de> Visitor<'de> for TileVisitor {
    type Value = u64;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Tile should be represented by a u64")
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(u64::from(value))
    }
}

impl<'de> Deserialize<'de> for TileType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer.deserialize_i8(TileVisitor) {
            Ok(0) => Ok(TileType::Ground),
            Ok(1) => Ok(TileType::Wall),
            Ok(2) => Ok(TileType::Mud),
            Ok(_) => Err(de::Error::custom("there are no tiles with that val")),
            Err(e) => Err(e),
        }
    }
}
