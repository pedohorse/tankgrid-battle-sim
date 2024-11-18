use crate::log_data::LogRepresentable;
use crate::orientation::SimpleOrientation;
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
            "up" | "north" => Some(GridOrientation::Up),
            "right" | "east" => Some(GridOrientation::Right),
            "down" | "south" => Some(GridOrientation::Down),
            "left" | "west" => Some(GridOrientation::Left),
            _ => None,
        }
    }
}

impl LogRepresentable for GridOrientation {
    fn log_repr(&self) -> String {
        match self {
            GridOrientation::Up => "north",
            GridOrientation::Right => "east",
            GridOrientation::Down => "south",
            GridOrientation::Left => "west",
        }
        .to_owned()
    }
}


impl SimpleOrientation for GridOrientation {
    fn same_as(&self, other: &Self) -> bool {
        self == other
    }

    fn opposite_of(&self, other: &Self) -> bool {
        match (self, other) {
            (GridOrientation::Up, GridOrientation::Down) => true,
            (GridOrientation::Right, GridOrientation::Left) => true,
            (GridOrientation::Down, GridOrientation::Up) => true,
            (GridOrientation::Left, GridOrientation::Right) => true,
            _ => false,
        }
    }

    fn left_of(&self, other: &Self) -> bool {
        // self is to the left from other
        match self {
            GridOrientation::Up => if let GridOrientation::Right|GridOrientation::Down = other { true } else { false },
            GridOrientation::Right => if let GridOrientation::Down|GridOrientation::Left = other { true } else { false },
            GridOrientation::Down => if let GridOrientation::Left|GridOrientation::Up = other { true } else { false },
            GridOrientation::Left => if let GridOrientation::Up|GridOrientation::Right = other { true } else { false },
        }
    }

    fn right_of(&self, other: &Self) -> bool {
        // self is to the right from other
        match self {
            GridOrientation::Up => if let GridOrientation::Left|GridOrientation::Down = other { true } else { false },
            GridOrientation::Right => if let GridOrientation::Up|GridOrientation::Left = other { true } else { false },
            GridOrientation::Down => if let GridOrientation::Right|GridOrientation::Up = other { true } else { false },
            GridOrientation::Left => if let GridOrientation::Down|GridOrientation::Right = other { true } else { false },
        }
    }
}