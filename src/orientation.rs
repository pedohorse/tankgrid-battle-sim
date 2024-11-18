pub trait SimpleOrientation {
    fn same_as(&self, other: &Self) -> bool;
    fn opposite_of(&self, other: &Self) -> bool;
    fn left_of(&self, other: &Self) -> bool;
    fn right_of(&self, other: &Self) -> bool;
}