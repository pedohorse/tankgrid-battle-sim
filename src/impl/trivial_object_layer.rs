use crate::{map_object::MapObject, object_layer::ObjectLayer};

pub struct TrivialObjectLayer<MObj> {
    cache: Vec<MObj>,
}

impl<R, MObj> ObjectLayer<R, MObj> for TrivialObjectLayer<MObj>
where
    MObj: MapObject<R>,
{
    fn new() -> Self {
        TrivialObjectLayer { cache: Vec::new() }
    }

    fn add(&mut self, obj: MObj) {
        self.cache.push(obj);
    }

    fn clear(&mut self) {
        self.cache.clear();
    }

    fn clear_by<F>(&mut self, f: F)
    where
        F: Fn(&MObj) -> bool,
    {
        self.cache.retain(|x| !f(x));
    }

    fn remove_object(&mut self, uid: u64) -> bool {
        let prev_len = self.cache.len();
        self.cache.retain(|x| x.unique_id() != uid);
        self.cache.len() != prev_len
    }

    fn object_by_id(&self, uid: u64) -> Option<&MObj> {
        for obj in self.cache.iter() {
            if obj.unique_id() == uid {
                return Some(&obj);
            }
        }
        None
    }

    fn objects_at(&self, x: i64, y: i64) -> Vec<&MObj> {
        let mut ret = Vec::new();
        // trivial method: just check all objects
        for object in self.cache.iter() {
            if object.position() == (x, y) {
                ret.push(object);
            }
        }
        ret
    }

    fn objects(&self) -> &[MObj] {
        &self.cache
    }
}
