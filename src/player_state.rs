
pub trait PlayerControl
{
    fn move_to(&mut self, pos: (i64, i64));
    fn turn_cw(&mut self);
    fn turn_ccw(&mut self);
    
    fn expend_resource(&mut self, res_id: usize, amount: usize);
    fn gain_resource(&mut self, res_id: usize, amount: usize);

    fn resource_value(&self, res_id: usize) -> usize;
}
