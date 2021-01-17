pub mod std_msgs;
pub mod rosserial_msgs;

pub trait Message {
    fn serialize(&self, buf: &mut [u8]) -> u16;
    fn deserialize(&mut self, buf: &[u8]) -> u16;
    fn name() -> &'static str where Self: Sized;
    fn md5() -> &'static str where Self: Sized;
}
