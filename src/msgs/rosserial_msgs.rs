use crate::msgs::Message;

pub const TOPICINFO_ID_PUBLISHER: u16 = 0;
pub const TOPICINFO_ID_SUBSCRIBER: u16 = 1;
pub const TOPICINFO_ID_SERVICE_SERVER: u16 = 2;
pub const TOPICINFO_ID_SERVICE_CLIENT: u16 = 4;
pub const TOPICINFO_ID_PARAMETER_REQUEST: u16 = 6;
pub const TOPICINFO_ID_LOG: u16 = 7;
pub const TOPICINFO_ID_TIME: u16 = 10;
pub const TOPICINFO_ID_TX_STOP: u16 = 11;


#[derive(Default)]
pub struct TopicInfo {
    pub id: u16,
    pub name: &'static str,
    pub message_type: &'static str,
    pub md5: &'static str,
    pub buffer_size: u32,
}

impl Message for TopicInfo {
    fn serialize(&self, buf: &mut [u8]) -> u16 {
        let mut offset = 0;

        buf[offset] = (self.id & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((self.id >> 8) & 0xFF) as u8;
        offset += 1;

        let length_name: u32 = self.name.len() as u32;
        buf[offset] = (length_name & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((length_name >> 8) & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((length_name >> 16) & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((length_name >> 24) & 0xFF) as u8;
        offset += 1;

        for b in self.name[..].as_bytes().iter() {
            buf[offset] = *b;
            offset += 1;
        }

        let length_message_type = self.message_type.len();
        buf[offset] = (length_message_type & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((length_message_type >> 8) & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((length_message_type >> 16) & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((length_message_type >> 24) & 0xFF) as u8;
        offset += 1;

        for b in self.message_type[..].as_bytes().iter() {
            buf[offset] = *b;
            offset += 1;
        }

        let length_md5 = self.md5.len();
        buf[offset] = (length_md5 & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((length_md5 >> 8) & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((length_md5 >> 16) & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((length_md5 >> 24) & 0xFF) as u8;
        offset += 1;

        for b in self.md5[..].as_bytes().iter() {
            buf[offset] = *b;
            offset += 1;
        }

        buf[offset] = (self.buffer_size & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((self.buffer_size >> 8) & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((self.buffer_size >> 16) & 0xFF) as u8;
        offset += 1;
        buf[offset] = ((self.buffer_size >> 24) & 0xFF) as u8;
        offset += 1;

        offset as u16
    }

    fn deserialize(&mut self, buf: &[u8]) -> u16 {
        0
    }

    fn name() -> &'static str {
        "rosserial_msgs/TopicInfo"
    }

    fn md5() -> &'static str {
        "0ad51f88fc44892f8c10684077646005"
    }
}