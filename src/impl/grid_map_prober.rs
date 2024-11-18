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
            false,
            true,
            false,
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
        stop_at_impassable_objects: bool,
        stop_at_unseethroughable_objects: bool,
        stop_at_shootable_objects: bool,
    ) -> Option<(i64, i64)> {
        self.raymarch(
            from,
            map,
            tile_logic,
            objects,
            orientation,
            stop_at_impassable_objects,
            stop_at_unseethroughable_objects,
            stop_at_shootable_objects,
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
        stop_at_impassable_objects: bool,
        stop_at_unseethroughable_objects: bool,
        stop_at_shootable_objects: bool,
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
        for _ in 0..MAX_LOOK_DIST {
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

            let mut tile_object = None;
            let mut object_blocks_ray = false;
            for object in objects.objects_at(x, y) {
                tile_object = Some(object);
                object_blocks_ray = stop_at_unseethroughable_objects && !object.seethroughable()
                    || stop_at_impassable_objects && !object.passable()
                    || stop_at_shootable_objects && object.shootable();
                break;
            }
            do_each_step(tile, tile_object);
            if !tile_logic.seethroughable(tile) || object_blocks_ray {
                return Some((x, y));
            }
            // if this tile is outside bounds - while loop will end after this
            // if out-of-bounds tile is blocking ray - we'll get it in check above
            // so all good
        }
        None
    }
}
