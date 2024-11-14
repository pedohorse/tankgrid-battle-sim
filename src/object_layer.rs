use super::map_object::MapObject;

pub trait ObjectLayer<R, MObj>
where
    MObj: MapObject<R>,
{
    fn new() -> Self;
    fn objects_at(&self, x: i64, y: i64) -> Vec<&MObj>;
    fn objects(&self) -> &[MObj];
    //fn objects_mut(&mut self) -> &[MObj];
    fn clear(&mut self);
    fn add(&mut self, obj: MObj);

    /// all objects at point are passable
    fn objects_at_are_passable(&self, x: i64, y: i64) -> bool {
        for object in self.objects_at(x, y) {
            if !object.passable() {
                return false;
            }
        }
        true
    }

    /// all objects at point are seethoughable
    fn objects_at_are_seethroughable(&self, x: i64, y: i64) -> bool {
        for object in self.objects_at(x, y) {
            if !object.seethroughable() {
                return false;
            }
        }
        true
    }
}
