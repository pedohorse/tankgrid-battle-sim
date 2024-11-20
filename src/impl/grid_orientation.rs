use crate::log_data::LogRepresentable;
use crate::orientation::SimpleOrientation;
use crate::script_repr::FromScriptRepr;

//Copy + Clone + Eq + Hash + Send + 'static + From<String>
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum GridOrientation {
    North,
    East,
    South,
    West,
}

impl FromScriptRepr for GridOrientation {
    fn from_script_repr(from: &str) -> Option<Self> {
        match from {
            "up" | "north" => Some(GridOrientation::North),
            "right" | "east" => Some(GridOrientation::East),
            "down" | "south" => Some(GridOrientation::South),
            "left" | "west" => Some(GridOrientation::West),
            _ => None,
        }
    }
}

impl LogRepresentable for GridOrientation {
    fn log_repr(&self) -> String {
        match self {
            GridOrientation::North => "north",
            GridOrientation::East => "east",
            GridOrientation::South => "south",
            GridOrientation::West => "west",
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
            (GridOrientation::North, GridOrientation::South) => true,
            (GridOrientation::East, GridOrientation::West) => true,
            (GridOrientation::South, GridOrientation::North) => true,
            (GridOrientation::West, GridOrientation::East) => true,
            _ => false,
        }
    }

    fn left_of(&self, other: &Self) -> bool {
        // self is to the left from other
        match self {
            GridOrientation::North => if let GridOrientation::East|GridOrientation::South = other { true } else { false },
            GridOrientation::East => if let GridOrientation::South|GridOrientation::West = other { true } else { false },
            GridOrientation::South => if let GridOrientation::West|GridOrientation::North = other { true } else { false },
            GridOrientation::West => if let GridOrientation::North|GridOrientation::East = other { true } else { false },
        }
    }

    fn right_of(&self, other: &Self) -> bool {
        // self is to the right from other
        match self {
            GridOrientation::North => if let GridOrientation::West|GridOrientation::South = other { true } else { false },
            GridOrientation::East => if let GridOrientation::North|GridOrientation::West = other { true } else { false },
            GridOrientation::South => if let GridOrientation::East|GridOrientation::North = other { true } else { false },
            GridOrientation::West => if let GridOrientation::South|GridOrientation::East = other { true } else { false },
        }
    }
}