use super::Message;
use crate::ros;

#[derive(Default)]
pub struct Bool {
    pub data: bool,
}

impl Message for Bool {
    fn serialize(&self, buf: &mut [u8]) -> u16 {
        buf[0] = self.data as u8;
        1
    }

    fn deserialize(&mut self, buf: &[u8]) -> u16 {
        self.data = buf[0] != 0;
        1
    }

    fn name() -> &'static str {
        "std_msgs/Bool"
    }

    fn md5() -> &'static str {
        "8b94c1b53db61fb6aed406028ad6332a"
    }
}

#[derive(Default)]
pub struct Time {
    data: ros::Time,
}

impl Message for Time {
    fn serialize(&self, buf: &mut [u8]) -> u16 {
        buf[0] = (self.data.sec & 0xFF) as u8;
        buf[1] = ((self.data.sec >> 8) & 0xFF) as u8;
        buf[2] = ((self.data.sec >> 16) & 0xFF) as u8;
        buf[3] = ((self.data.sec >> 24) & 0xFF) as u8;

        buf[4] = (self.data.nsec & 0xFF) as u8;
        buf[5] = ((self.data.nsec >> 8) & 0xFF) as u8;
        buf[6] = ((self.data.nsec >> 16) & 0xFF) as u8;
        buf[7] = ((self.data.nsec >> 24) & 0xFF) as u8;

        8
    }

    fn deserialize(&mut self, buf: &[u8]) -> u16 {
        let sec = (buf[0] as u32)
                  | ((buf[1] as u32) << 8)
                  | ((buf[2] as u32) << 16)
                  | ((buf[3] as u32) << 24);
        let nsec = (buf[4] as u32)
                   | ((buf[5] as u32) << 8)
                   | ((buf[6] as u32) << 16)
                   | ((buf[7] as u32) << 24);

        self.data.sec = sec;
        self.data.nsec = nsec;

        8
    }

    fn name() -> &'static str
    where Self: Sized {
        "std_msgs/Time"
    }

    fn md5() -> &'static str
    where Self: Sized {
        "cd7166c74c552c311fbcc2fe5a7bc289"
    }
}
