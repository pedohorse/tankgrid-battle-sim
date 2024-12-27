use crate::log_data::LogRepresentable;
use crate::orientation::SimpleOrientation;
use crate::script_repr::{FromScriptRepr, ToScriptRepr};

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
            "front" | "forward" | "north" => Some(GridOrientation::North),
            "right" | "east" => Some(GridOrientation::East),
            "back" | "south" => Some(GridOrientation::South),
            "left" | "west" => Some(GridOrientation::West),
            _ => None,
        }
    }
}

impl ToScriptRepr for GridOrientation {
    fn to_script_repr(&self) -> String {
        match self {
            GridOrientation::North => "forward",  // TODO: note, we have here "local" representation
            GridOrientation::East => "right",  // TODO: but for log repr - "global" representation hardcoded
            GridOrientation::South => "back",
            GridOrientation::West => "left",
        }
        .to_owned()
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

impl From<u64> for GridOrientation {
    fn from(value: u64) -> Self {
        match value {
            0 => GridOrientation::North,
            1 => GridOrientation::East,
            2 => GridOrientation::South,
            3 => GridOrientation::West,
            _ => panic!("bad value for converting into GridOrientation: {}", value),
        }
    }
}

impl Into<u64> for GridOrientation {
    fn into(self) -> u64 {
        match self {
            GridOrientation::North => 0,
            GridOrientation::East => 1,
            GridOrientation::South => 2,
            GridOrientation::West => 3,
        }
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
            GridOrientation::North => {
                if let GridOrientation::East | GridOrientation::South = other {
                    true
                } else {
                    false
                }
            }
            GridOrientation::East => {
                if let GridOrientation::South | GridOrientation::West = other {
                    true
                } else {
                    false
                }
            }
            GridOrientation::South => {
                if let GridOrientation::West | GridOrientation::North = other {
                    true
                } else {
                    false
                }
            }
            GridOrientation::West => {
                if let GridOrientation::North | GridOrientation::East = other {
                    true
                } else {
                    false
                }
            }
        }
    }

    fn right_of(&self, other: &Self) -> bool {
        // self is to the right from other
        match self {
            GridOrientation::North => {
                if let GridOrientation::West | GridOrientation::South = other {
                    true
                } else {
                    false
                }
            }
            GridOrientation::East => {
                if let GridOrientation::North | GridOrientation::West = other {
                    true
                } else {
                    false
                }
            }
            GridOrientation::South => {
                if let GridOrientation::East | GridOrientation::North = other {
                    true
                } else {
                    false
                }
            }
            GridOrientation::West => {
                if let GridOrientation::South | GridOrientation::East = other {
                    true
                } else {
                    false
                }
            }
        }
    }

    fn dot(&self, other: &Self) -> f64 {
        if self.same_as(other) { return 1.0; }
        if self.opposite_of(other) { return -1.0; }
        match (self, other) {
            (GridOrientation::North | GridOrientation::South, GridOrientation::East | GridOrientation::West) => 0.0,
            (GridOrientation::East | GridOrientation::West, GridOrientation::North | GridOrientation::South) => 0.0,
            _ => unreachable!()
        }
    }

    fn turn_cw(&self) -> Self{
        match self {
            GridOrientation::North => GridOrientation::East,
            GridOrientation::East => GridOrientation::South,
            GridOrientation::South => GridOrientation::West,
            GridOrientation::West => GridOrientation::North,
        }
    }
    fn turn_ccw(&self) -> Self {
        match self {
            GridOrientation::North => GridOrientation::West,
            GridOrientation::East => GridOrientation::North,
            GridOrientation::South => GridOrientation::East,
            GridOrientation::West => GridOrientation::South,
        }
    }
    fn opposite(&self) -> Self {
        match self {
            GridOrientation::North => GridOrientation::South,
            GridOrientation::East => GridOrientation::West,
            GridOrientation::South => GridOrientation::North,
            GridOrientation::West => GridOrientation::East,
        }
    }

    /// we consider North to be local "forward"
    fn from_relative_to_global(&self, relative_to: &Self) -> Self {
        match relative_to {
            GridOrientation::North => *self,
            GridOrientation::East => self.turn_cw(),
            GridOrientation::South => self.turn_cw().turn_cw(),
            GridOrientation::West => self.turn_ccw(),
        }
    }
    fn global_to_relative_to(&self, relative_to: &Self) -> Self {
        match relative_to {
            GridOrientation::North => *self,
            GridOrientation::East => self.turn_ccw(),
            GridOrientation::South => self.turn_ccw().turn_ccw(),
            GridOrientation::West => self.turn_cw(),
        }
    }

    fn direction_to_closest_orientations(from: (i64, i64), to: (i64, i64)) -> (Self, Option<Self>) {
        let dir = (to.0 - from.0, to.1 - from.1);

        let xori = if dir.0 >= 0 {
            GridOrientation::East
        } else {
            GridOrientation::West
        };
        let yori = if dir.1 <= 0 { // <= cuz y grows down
            GridOrientation::North
        } else {
            GridOrientation::South
        };

        if dir.0 == 0 {
            (yori, None)
        } else if dir.1 == 0 {
            (xori, None)
        // special "fair" way of treating border values
        } else if dir.1 * dir.0 > 0 {
            if dir.1.abs() >= dir.0.abs() {
                (yori, Some(xori))
            } else {
                (xori, Some(yori))
            }
        } else {
            if dir.1.abs() > dir.0.abs() {
                (yori, Some(xori))
            } else {
                (xori, Some(yori))
            }
        }
    }
}


mod tests {
    use crate::orientation::main_logic_tests;
    use super::GridOrientation;
    use super::SimpleOrientation;

    main_logic_tests!(grid_tests, GridOrientation::North, GridOrientation::East, GridOrientation::South, GridOrientation::West);
}