use crate::script_repr::FromScriptRepr;

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