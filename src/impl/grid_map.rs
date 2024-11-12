use map_lib::MapData;
use std::{path::Path, vec::Vec};
use crate::map::{MapReadAccess, MapWriteAccess};

pub struct GridBattleMap<T> {
    width: usize,
    height: usize,
    map_data: MapData<T>,
}


pub trait FromFile<T> {
    fn load_from_file(path: &Path) -> std::io::Result<GridBattleMap<T>>;
}

impl<T> MapReadAccess<T> for GridBattleMap<T>
where
    T: Copy + Clone,
{
    fn get_tile_at(&self, x: i64, y: i64) -> T {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return self.map_data.outer_value();
        }
        let x = x as usize;
        let y = y as usize;
        self.map_data.row(y)[x]
    }
}

impl<T> MapWriteAccess<T> for GridBattleMap<T>
where
    T: Copy + Clone,
{
    fn set_tile_at(&mut self, x: i64, y: i64, val: T) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        let x = x as usize;
        let y = y as usize;
        self.map_data.row_mut(y)[x] = val;
    }
}

impl<T> GridBattleMap<T>
where
    T: Copy + Clone,
{
    pub fn new(
        width: usize,
        height: usize,
        default_tile_type: T,
        out_of_bounds_cell_type: T,
    ) -> GridBattleMap<T> {
        let mut row_sizes = Vec::with_capacity(height);
        row_sizes.resize(height, width);
        let map_data = MapData::new_from_constant_rows(default_tile_type, &row_sizes, out_of_bounds_cell_type);
        GridBattleMap {
            width,
            height,
            map_data,
        }
    }

    /// if map data can not represent grid map - error is returned
    pub fn new_from_data(
        map_data: MapData<T>,
    ) -> Result<GridBattleMap<T>, ()> {
        let height = map_data.row_count();
        if height == 0 {
            return Err(());
        };
        let width = map_data.row(0).len();
        if width == 0 {
            return Err(());
        };
        for i in 0..height {
            if width != map_data.row(i).len() {
                return Err(());
            }
        }
        Ok(GridBattleMap {
            width,
            height,
            map_data,
        })
    }
}
