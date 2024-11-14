use super::map_object::MapObject;
use super::map::MapReadAccess;
use super::maptile_logic::MaptileLogic;
use super::object_layer::ObjectLayer;

pub trait PlayerControl<R, M, T, C, MObj, OL>
where
    M: MapReadAccess<T>,
    C: MaptileLogic<T>,
    MObj: MapObject<R>,
    OL: ObjectLayer<R, MObj>,
{
    fn move_forward(&mut self, map: &mut M, logic: &C, object_layer: &OL);
    fn turn_cw(&mut self, map: &mut M, logic: &C, object_layer: &OL);
    fn turn_ccw(&mut self, map: &mut M, logic: &C, object_layer: &OL);
    
    fn expend_resource(&mut self, res_id: usize, amount: usize);
    fn gain_resource(&mut self, res_id: usize, amount: usize);

    fn resource_value(&self, res_id: usize) -> usize;
}
