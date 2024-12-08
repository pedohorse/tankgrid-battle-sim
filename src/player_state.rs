
pub trait PlayerControl
{
    fn move_to(&mut self, pos: (i64, i64));
    fn turn_cw(&mut self);
    fn turn_ccw(&mut self);
    
    fn expend_resource(&mut self, res_id: usize, amount: u64);
    fn gain_resource(&mut self, res_id: usize, amount: u64);
    fn set_resource(&mut self, res_id: usize, amount: u64);

    fn resource_value(&self, res_id: usize) -> u64;
}
