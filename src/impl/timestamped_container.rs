use std::{collections::HashMap, hash::Hash};

pub struct ExpiringContainer<K, TS, T> {
    elements: HashMap<K, (TS, Option<TS>, T)>, // timestamp added, timestamp expiration, data
}

pub struct ExpiringContainerIterator<'a, K, TS, T> {
    inner_iter: std::collections::hash_map::Values<'a, K, (TS, Option<TS>, T)>,
    slice_timestamp: TS,
}

///
/// this is trivial implementation
/// we don't need a smarter one at this point
///
impl<K, TS, T> ExpiringContainer<K, TS, T>
where
    K: Hash + Eq,
    TS: PartialOrd,
{
    pub fn new() -> ExpiringContainer<K, TS, T> {
        ExpiringContainer {
            elements: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, element: T, valid_from: TS, valid_to: Option<TS>) {
        self.elements.insert(key, (valid_from, valid_to, element));
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut (TS, Option<TS>, T)> {
        self.elements.get_mut(key)
    }

    pub fn iter_at_timestamp(&self, timestamp: TS) -> ExpiringContainerIterator<'_, K, TS, T> {
        ExpiringContainerIterator {
            inner_iter: self.elements.values(),
            slice_timestamp: timestamp,
        }
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.elements.values().map(|(_, _, val)| val)
    }

    pub fn prune_before_timestamp(&mut self, timestamp: TS) {
        self.elements.retain(|_, (_, end_maybe, _)| {
            if let Some(end) = end_maybe {
                *end > timestamp
            } else {
                true
            }
        });
    }
}

impl<'a, K, TS, T> Iterator for ExpiringContainerIterator<'a, K, TS, T>
where
    TS: PartialOrd,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next_elem = self.inner_iter.next();
            if let Some(elems) = next_elem {
                match elems {
                    (start, None, val) if *start <= self.slice_timestamp => {
                        return Some(val)
                    }
                    (start, Some(end), val) if *start <= self.slice_timestamp && self.slice_timestamp < *end => {
                        return Some(val)
                    }
                    _ => continue
                }
            } else {
                return None
            }
        }
    }
}


mod tests{
    use std::collections::HashSet;

    use super::ExpiringContainer;

    #[test]
    fn test_simple() {
        let mut cont = ExpiringContainer::new();

        assert_eq!(0, cont.iter_at_timestamp(0).collect::<Vec<_>>().len());

        cont.insert(123, 2345_i64, 0, Some(4));

        assert_eq!(vec![0_i64; 0], cont.iter_at_timestamp(-1).copied().collect::<Vec<_>>());
        assert_eq!(vec![2345_i64], cont.iter_at_timestamp(0).copied().collect::<Vec<_>>());
        assert_eq!(vec![2345_i64], cont.iter_at_timestamp(1).copied().collect::<Vec<_>>());
        assert_eq!(vec![2345_i64], cont.iter_at_timestamp(2).copied().collect::<Vec<_>>());
        assert_eq!(vec![2345_i64], cont.iter_at_timestamp(3).copied().collect::<Vec<_>>());
        assert_eq!(vec![0_i64; 0], cont.iter_at_timestamp(4).copied().collect::<Vec<_>>());

        cont.insert(234, 3456_i64, 2, Some(5));
        assert_eq!(HashSet::from([0_i64; 0]), cont.iter_at_timestamp(-1).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([2345_i64]), cont.iter_at_timestamp(0).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([2345_i64]), cont.iter_at_timestamp(1).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([2345_i64, 3456_i64]), cont.iter_at_timestamp(2).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([2345_i64, 3456_i64]), cont.iter_at_timestamp(3).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([3456_i64]), cont.iter_at_timestamp(4).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([0_i64; 0]), cont.iter_at_timestamp(5).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([0_i64; 0]), cont.iter_at_timestamp(6).copied().collect::<HashSet<_>>());

        cont.insert(345, 4567_i64, 4, None);
        assert_eq!(HashSet::from([0_i64; 0]), cont.iter_at_timestamp(-1).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([2345_i64]), cont.iter_at_timestamp(0).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([2345_i64]), cont.iter_at_timestamp(1).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([2345_i64, 3456_i64]), cont.iter_at_timestamp(2).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([2345_i64, 3456_i64]), cont.iter_at_timestamp(3).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([3456_i64, 4567_i64]), cont.iter_at_timestamp(4).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([4567_i64]), cont.iter_at_timestamp(5).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([4567_i64]), cont.iter_at_timestamp(6).copied().collect::<HashSet<_>>());
        assert_eq!(HashSet::from([4567_i64]), cont.iter_at_timestamp(666).copied().collect::<HashSet<_>>());
    }

    #[test]
    fn test_prune_simple() {
        let mut cont = ExpiringContainer::new();

        cont.insert(132, 1_234_i32, -10, Some(5));
        cont.insert(234, 2_345_i32, 0, Some(1));
        cont.insert(456, 3_456_i32, 3, Some(6));
        cont.insert(567, 4_567_i32, -20, None);

        assert_eq!(HashSet::from([1_234_i32, 2_345_i32, 3_456_i32, 4_567_i32]), cont.values().copied().collect::<HashSet<_>>());

        cont.prune_before_timestamp(5);

        assert_eq!(HashSet::from([3_456_i32, 4_567_i32]), cont.values().copied().collect::<HashSet<_>>());
    }
}