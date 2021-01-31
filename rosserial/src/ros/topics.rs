use crate::msgs::rosserial_msgs::TopicInfo;
use crate::msgs::Message;

#[derive(Clone, Copy)]
pub struct Publisher {
    pub topic: &'static str,
    pub id: u16,
    pub message_type: &'static str,
    pub md5sum: &'static str,
}

impl Publisher {
    pub fn new(topic: &'static str, id: u16, message_type: &'static str, md5sum: &'static str) -> Self {
        Publisher {
            topic,
            id,
            message_type,
            md5sum,
        }
    }
}

impl Into<TopicInfo> for Publisher {
    fn into(self) -> TopicInfo {
        TopicInfo {
            id: self.id,
            name: self.topic,
            message_type: self.message_type,
            md5: self.md5sum,
            buffer_size: 256,
        }
    }
}

pub trait MessageHandler {
    fn handle_message(&mut self, data: &[u8]);
}

pub trait TopicBase {
    fn topic(&self) -> &'static str;
    fn message_type(&self) -> &'static str;
    fn md5sum(&self) -> &'static str;
}

pub struct Subscriber<Msg, F: FnMut(Msg)> {
    pub topic: &'static str,
    pub message_type: &'static str,
    pub md5sum: &'static str,

    callback: F,
    phantom_msg: core::marker::PhantomData<Msg>,
}

impl<Msg: Message, F: FnMut(Msg)> Subscriber<Msg, F> {
    pub fn new(topic: &'static str, callback: F) -> Self {
        Subscriber {
            topic,
            message_type: Msg::name(),
            md5sum: Msg::md5(),
            callback,
            phantom_msg: Default::default(),
        }
    }
}

impl<Msg: Message + Default, F: FnMut(Msg)> MessageHandler for Subscriber<Msg, F> {
    fn handle_message(&mut self, data: &[u8]) {
        let mut msg = Msg::default();
        msg.deserialize(data);

        (self.callback)(msg);
    }
}

impl<Msg: Message, F: FnMut(Msg)> TopicBase for Subscriber<Msg, F> {
    fn topic(&self) -> &'static str {
        self.topic
    }

    fn message_type(&self) -> &'static str {
        self.message_type
    }

    fn md5sum(&self) -> &'static str {
        self.md5sum
    }
}

// impl Into<TopicInfo> for Subscriber {
//     fn into(self) -> TopicInfo {
//         TopicInfo {
//             id: self.id,
//             name: self.topic,
//             message_type: self.message_type,
//             md5: self.md5sum,
//             buffer_size: 256,
//         }
//     }
// }
