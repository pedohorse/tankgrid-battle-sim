use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct MapData<T> {
    rows: Vec<Vec<T>>,
    outer_value: T,
}

impl<T> MapData<T>
where
    T: Copy + Clone,
{
    pub fn new_from_rows(rows: &[&[T]], outer_value: T) -> MapData<T> {
        let mut row_vecs = Vec::with_capacity(rows.len());
        for row in rows {
            row_vecs.push(row.to_vec());
        }
        MapData {
            rows: row_vecs,
            outer_value,
        }
    }

    pub fn new_from_constant_rows(value: T, row_sizes: &[usize], outer_value: T) -> MapData<T> {
        let mut row_vecs = Vec::with_capacity(row_sizes.len());
        for &row_size in row_sizes {
            let mut row = Vec::with_capacity(row_size);
            row.resize(row_size, value);
            row_vecs.push(row);
        }
        MapData {
            rows: row_vecs,
            outer_value,
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn row(&self, row_num: usize) -> &[T] {
        &self.rows[row_num]
    }

    pub fn row_mut(&mut self, row_num: usize) -> &mut [T] {
        &mut self.rows[row_num]
    }

    pub fn outer_value(&self) -> T {
        self.outer_value
    }

    pub fn set_outer_value(&mut self, value: T) {
        self.outer_value = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn simple_ser() {
        let data = MapData::<usize> {
            rows: vec![vec![1, 2], vec![3, 4, 5], vec![]],
            outer_value: 123,
        };

        let ser = if let Ok(x) = serde_json::to_string(&data) {
            x
        } else {
            assert!(false, "failed to to_string");
            unreachable!();
        };

        assert_eq!(r#"{"rows":[[1,2],[3,4,5],[]],"outer_value":123}"#, ser)
    }

    #[test]
    fn simple_de() {
        let data: MapData<usize> = if let Ok(x) = serde_json::from_str(
            r#"{
        "outer_value": 123,
        "rows": [[1, 2], [3, 4, 5], []]
        }"#,
        ) {
            x
        } else {
            assert!(false, "failed to from_str");
            unreachable!();
        };

        assert_eq!(vec![vec![1, 2], vec![3, 4, 5], vec![]], data.rows);
    }
}
