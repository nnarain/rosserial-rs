pub trait HardwareInterface {
    fn read(&self) -> Option<u8>;
    fn write(&mut self, data: u8);
    fn time(&self) -> u32;
}
