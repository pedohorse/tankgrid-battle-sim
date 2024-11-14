use super::map::MapReadAccess;
use super::map_object::MapObject;
use super::maptile_logic::MaptileLogic;
use super::object_layer::ObjectLayer;

// pub enum TileOrObject<T, MObj> {
//     Tile(T),
//     Object(MObj),
// }

pub trait MapProber<T, R, M, L, MObj, OL>
where
    R: Sized,
    MObj: MapObject<R>,
    OL: ObjectLayer<R, MObj>,
{
    fn raycast(
        &self,
        from: (i64, i64),
        map: &M,
        tile_logic: &L,
        object_layer: &OL,
        orientation: R,
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
