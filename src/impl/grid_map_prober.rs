use super::grid_map::GridBattleMap;
use super::grid_orientation::GridOrientation;
use crate::map::MapReadAccess;
use crate::map_object::MapObject;
use crate::map_prober::MapProber;
use crate::maptile_logic::MaptileLogic;
use crate::object_layer::ObjectLayer;

pub struct GridMapProber {}

const MAX_LOOK_DIST: usize = 32;

// TODO: this can be generalized, only specific behaviour is orientation
impl<T, M, L, MObj, OL> MapProber<T, GridOrientation, M, L, MObj, OL> for GridMapProber
where
    T: Copy + Clone,
    M: MapReadAccess<T>,
    L: MaptileLogic<T>,
    MObj: MapObject<GridOrientation>,
    OL: ObjectLayer<GridOrientation, MObj>,
{
    fn look<'a>(
        &self,
        from: (i64, i64),
        map: &M,
        tile_logic: &L,
        objects: &'a OL,
        orientation: GridOrientation,
    ) -> Vec<(T, Option<&'a MObj>)> {
        let mut ret = Vec::new();
        self.raymarch(
            from,
            map,
            tile_logic,
            objects,
            orientation,
            &mut |tile, tile_object| {
                ret.push((tile, tile_object));
            },
        );
        ret
    }

    fn raycast(
        &self,
        from: (i64, i64),
        map: &M,
        tile_logic: &L,
        objects: &OL,
        orientation: GridOrientation,
    ) -> Option<(i64, i64)> {
        self.raymarch(
            from,
            map,
            tile_logic,
            objects,
            orientation,
            &mut |_, _| {},
        )
    }
}

impl GridMapProber {
    fn raymarch<'a, T, M, L, MObj, OL, F>(
        &self,
        from: (i64, i64),
        map: &M,
        tile_logic: &L,
        objects: &'a OL,
        orientation: GridOrientation,
        do_each_step: &mut F,
    ) -> Option<(i64, i64)>
    where
        T: Copy + Clone,
        M: MapReadAccess<T>,
        L: MaptileLogic<T>,
        MObj: MapObject<GridOrientation> + 'a,
        OL: ObjectLayer<GridOrientation, MObj>,
        F: FnMut(T, Option<&'a MObj>),
    {
        let (mut x, mut y) = from;
        while map.is_within_bounds(x, y) {
            match orientation {
                GridOrientation::Up => {
                    y -= 1;
                }
                GridOrientation::Right => {
                    x += 1;
                }
                GridOrientation::Down => {
                    y += 1;
                }
                GridOrientation::Left => {
                    x -= 1;
                }
            };
            let tile = map.get_tile_at(x, y);
            // TODO: see same todo in look
            let mut tile_object = None;
            let mut object_blocks_view = false;
            for object in objects.objects_at(x, y) {
                tile_object = Some(object);
                object_blocks_view = !object.seethroughable();
                break;
            }
            do_each_step(tile, tile_object);
            if !tile_logic.seethroughable(tile) || object_blocks_view {
                break;
            }
            if !tile_logic.seethroughable(tile) {
                return Some((x, y));
            }
        }
        None
    }
}
