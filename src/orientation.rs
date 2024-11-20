pub trait SimpleOrientation {
    fn same_as(&self, other: &Self) -> bool;
    fn opposite_of(&self, other: &Self) -> bool;
    fn left_of(&self, other: &Self) -> bool;
    fn right_of(&self, other: &Self) -> bool;
    
    fn turn_cw(&self) -> Self;
    fn turn_ccw(&self) -> Self;
    fn from_relative_to_global(&self, relative_to: &Self) -> Self;
}