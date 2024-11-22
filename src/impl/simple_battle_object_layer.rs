use crate::{map_object::MapObject, object_layer::ObjectLayer};

pub struct SimpleBattleObjectLayer<MObj> {
    cache: Vec<MObj>,
}

impl<R, MObj> ObjectLayer<R, MObj> for SimpleBattleObjectLayer<MObj>
where
    MObj: MapObject<R>,
{
    fn new() -> Self {
        SimpleBattleObjectLayer { cache: Vec::new() }
    }

    fn add(&mut self, obj: MObj) -> u64 {
        let mut uid = obj.unique_id();
        // ensure uid uniqueness
        let obj = if self.cache.len() > 0 {
            let mut max_existing_uid = 0;
            let mut need_change = false;
            for obj in self.cache.iter() {
                let cur_uid = obj.unique_id();
                if cur_uid > max_existing_uid {
                    max_existing_uid = cur_uid;
                }
                if cur_uid == uid {
                    need_change = true;
                }
            }
            if need_change {
                uid = max_existing_uid + 1;
                MObj::clone_with_uid(&obj, uid)
            } else { obj }
        } else { obj };
        
        self.cache.push(obj);
        uid
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
