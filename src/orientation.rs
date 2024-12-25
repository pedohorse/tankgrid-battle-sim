pub trait SimpleOrientation: Sized {
    fn same_as(&self, other: &Self) -> bool;
    fn opposite_of(&self, other: &Self) -> bool;
    /// opposite direction (if such exists) should count as both left_of and right_of
    /// while same direction counts as neither
    fn left_of(&self, other: &Self) -> bool;
    fn right_of(&self, other: &Self) -> bool;

    // orthogonal directions (if such exist) should return false in both cases
    fn codirected_with(&self, other: &Self) -> bool;
    fn counterdirected_with(&self, other: &Self) -> bool;
    
    fn turn_cw(&self) -> Self;
    fn turn_ccw(&self) -> Self;
    fn opposite(&self) -> Self;
    fn from_relative_to_global(&self, relative_to: &Self) -> Self;
    fn global_to_relative_to(&self, relative_to: &Self) -> Self;

    /// return orientation closest to given direction.
    /// if orientation is exact - second tuple element is None
    /// otherwise second element is second closest orientation
    fn direction_to_closest_orientations(from: (i64, i64), to: (i64, i64)) -> (Self, Option<Self>);
}

macro_rules! main_logic_tests {
    ($name:ident, $($values:expr),+) => {
        #[test]
        fn $name() {
            let vals = [$($values),+];
            for val1 in vals.iter() {
                for val2 in vals.iter() {
                    if val1.same_as(val2) {
                        assert!(val2.same_as(val1));
                        assert!(!val1.right_of(val2));
                        assert!(!val1.left_of(val2));
                        assert!(!val2.right_of(val1));
                        assert!(!val2.left_of(val1));
                    }
                    if val1.opposite_of(val2) {
                        assert!(val2.opposite_of(val1));
                        assert!(val1.right_of(val2));
                        assert!(val1.left_of(val2));
                        assert!(val2.right_of(val1));
                        assert!(val2.left_of(val1));
                    }
                }
            }
        }
    };
}
pub(crate) use main_logic_tests;