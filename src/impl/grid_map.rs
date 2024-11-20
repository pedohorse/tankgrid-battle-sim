use crate::map::{MapReadAccess, MapWriteAccess};
use map_lib::MapData;
use std::{path::Path, vec::Vec};

use super::grid_orientation::GridOrientation;

pub struct GridBattleMap<T> {
    width: usize,
    height: usize,
    map_data: MapData<T>,
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
    fn is_within_bounds(&self, x: i64, y: i64) -> bool {
        return x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height;
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
        let map_data =
            MapData::new_from_constant_rows(default_tile_type, &row_sizes, out_of_bounds_cell_type);
        GridBattleMap {
            width,
            height,
            map_data,
        }
    }

    /// if map data can not represent grid map - error is returned
    pub fn new_from_data(map_data: MapData<T>) -> Result<GridBattleMap<T>, ()> {
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

    pub fn map_data(&self) -> &MapData<T> {
        &self.map_data
    }

    // will fail if map cannot have "count" of player spawn places
    pub fn get_spawn_locations(
        &self,
        count: usize,
    ) -> Result<Vec<(i64, i64, GridOrientation)>, ()> {
        match count {
            0 => Err(()),
            1 => Ok(vec![(0, 0, GridOrientation::East)]),
            2 => Ok(vec![
                (0, 0, GridOrientation::East),
                (
                    self.width as i64 - 1,
                    self.height as i64 - 1,
                    GridOrientation::West,
                ),
            ]),
            3 => Ok(vec![
                (0, 0, GridOrientation::East),
                (0, self.height as i64 - 1, GridOrientation::North),
                (
                    self.width as i64 - 1,
                    self.height as i64 - 1,
                    GridOrientation::West,
                ),
            ]),
            4 => Ok(vec![
                (0, 0, GridOrientation::East),
                (0, self.height as i64 - 1, GridOrientation::North),
                (
                    self.width as i64 - 1,
                    self.height as i64 - 1,
                    GridOrientation::West,
                ),
                (self.width as i64 - 1, 0, GridOrientation::South),
            ]),
            _ => Err(()), // FOR NOW we don't support more
        }
    }
}
