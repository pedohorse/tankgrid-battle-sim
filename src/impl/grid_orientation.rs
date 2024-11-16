use crate::script_repr::FromScriptRepr;
use crate::log_data::LogRepresentable;

//Copy + Clone + Eq + Hash + Send + 'static + From<String>
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum GridOrientation {
    Up,
    Right,
    Down,
    Left,
}

impl FromScriptRepr for GridOrientation {
    fn from_script_repr(from: &str) -> Option<Self> {
        match from {
            "up"|"north" => Some(GridOrientation::Up),
            "right"|"east" => Some(GridOrientation::Right),
            "down"|"south" => Some(GridOrientation::Down),
            "left"|"west" => Some(GridOrientation::Left),
            _ => None
        }
    }
}

impl LogRepresentable for GridOrientation {
    fn log_repr(&self) -> String {
        match self {
            GridOrientation::Up => "up",
            GridOrientation::Right => "right",
            GridOrientation::Down => "down",
            GridOrientation::Left => "left",
        }.to_owned()
    }
}