use super::map_object::MapObject;
use super::object_layer::ObjectLayer;

pub trait MapProber<T, R, M, L, MObj, OL>
where
    R: Sized,
    MObj: MapObject<R>,
    OL: ObjectLayer<R, MObj>,
{   
    fn step_in_direction(&self, pos: (i64, i64), ori: R) -> (i64, i64);

    fn raycast(
        &self,
        from: (i64, i64),
        map: &M,
        tile_logic: &L,
        object_layer: &OL,
        orientation: R,
        stop_at_impassable_objects: bool,
        stop_at_unseethroughable_objects: bool,
        stop_at_shootable_objects: bool,
    ) -> Option<(i64, i64)>;

    /// since we don't know what user script is going to do with the look result -
    /// there is no point not o alloc it from the very start - it will be moved into
    /// player's vm anyway
    ///
    /// result is a tuple of tile type and first object on it if any
    fn look<'a>(
        &self,
        from: (i64, i64),
        map: &M,
        tile_logic: &L,
        object_layer: &'a OL,
        orientation: R,
    ) -> Vec<(T, Option<&'a MObj>)>;
}
