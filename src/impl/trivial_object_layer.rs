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
