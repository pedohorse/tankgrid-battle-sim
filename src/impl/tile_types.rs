use crate::script_repr::ToScriptRepr;

#[derive(Clone, Copy)]
pub enum TileType {
    Ground,
    Mud,
    Wall,
}

impl ToScriptRepr for TileType {
    fn to_script_repr(&self) -> String {
        match self {
            TileType::Ground => "ground",
            TileType::Mud => "mud",
            TileType::Wall => "wall",
        }
        .to_string()
    }
}
