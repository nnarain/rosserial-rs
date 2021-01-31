use super::{HardwareInterface, Publisher, MessageHandler, TopicBase};
use crate::msgs::{Message, rosserial_msgs, std_msgs};

// use alloc::boxed::Box;

pub type PublisherHandle = usize;

#[derive(Debug)]
pub enum NodeHandleError {
    MaxPublishersReached,
}

enum State {
    Sync,
    ProtocolVersion,
    SizeLsb,
    SizeMsb,
    SizeChecksum,
    TopicIdLsb,
    TopicIdMsb,
    Message,
    MessageChecksum
}

// const PROTOCOL_VER1: u8 = 0xFF;
const PROTOCOL_VER2: u8 = 0xFE;

const MESSAGE_BUFFER_SIZE: usize = 1024;
const MAX_PUB_SUBS: usize = 256;


pub struct NodeHandle<'a> {
    state: State,
    message_in: [u8; MESSAGE_BUFFER_SIZE],
    index: usize,
    bytes: u16,
    topic: u16,
    checksum: u16,
    configured: bool,

    publishers: [Option<Publisher>; MAX_PUB_SUBS],
    subscribers: [Option<&'a mut dyn MessageHandler>; MAX_PUB_SUBS],
    subscriber_info: [Option<rosserial_msgs::TopicInfo>; MAX_PUB_SUBS],
}

impl<'a> Default for NodeHandle<'a> {
    fn default() -> Self {
        NodeHandle {
            state: State::Sync,
            message_in: [0; 1024],
            index: 0,
            bytes: 0,
            topic: 0,
            checksum: 0,
            configured: false,

            publishers: [None; MAX_PUB_SUBS],
            subscribers: [None; MAX_PUB_SUBS],
            subscriber_info: [None; MAX_PUB_SUBS],
        }
    }
}

impl<'a> NodeHandle<'a> {
    pub fn advertise<Msg: Message>(&mut self, topic: &'static str) -> Result<PublisherHandle, NodeHandleError> {
        // Find the next available slot
        let slot = self.publishers.iter_mut().filter(|item| item.is_none()).enumerate().next();

        if let Some((i, slot)) = slot {
            let handle = i;
            *slot = Some(Publisher::new(topic, (i + 100) as u16, Msg::name(), Msg::md5()));
            Ok(handle)
        }
        else {
            Err(NodeHandleError::MaxPublishersReached)
        }
    }

    pub fn register_subscriber<Sub: MessageHandler + TopicBase, Msg: Message>(&mut self, sub: &'a mut Sub) {
        let slot = self.subscribers.iter_mut().filter(|item| item.is_none()).enumerate().next();
        if let Some((i, slot)) = slot {
            // Info for this topic
            let mut ti = rosserial_msgs::TopicInfo::default();
            ti.id = (i as u16) + 100;
            ti.name = sub.topic();
            ti.message_type = sub.message_type();
            ti.md5 = sub.md5sum();
            ti.buffer_size = 256;

            self.subscriber_info[i] = Some(ti);

            *slot = Some(sub);

        }
    }

    pub fn publish(&self, handle: usize, msg: &dyn Message, hardware: &mut dyn HardwareInterface) {
        if let Some(ref p) = self.publishers[handle] {
            self.send_message(p.id, msg, hardware);
        }
    }

    pub fn spin_once(&mut self, hardware: &mut dyn HardwareInterface) {
        // let current_time = hardware.time();

        let data = hardware.read();

        if let Some(data) = data {
            match self.state {
                State::Sync => {
                    if data == 0xFF {
                        self.state = State::ProtocolVersion;
                    }
                },
                State::ProtocolVersion => {
                    self.state = if data == PROTOCOL_VER2 {
                        State::SizeLsb
                    }
                    else {
                        State::Sync
                    };
                },
                State::SizeLsb => {
                    self.bytes = data as u16;
                    self.index = 0;
                    self.checksum = data as u16; // first byte to calculate checksum
                    self.state = State::SizeMsb;
                },
                State::SizeMsb => {
                    self.bytes |= (data as u16) << 8;
                    self.state = State::SizeChecksum;
                },
                State::SizeChecksum => {
                    // Message Length Checksum = 255 - ((Message Length High Byte + Message Length Low Byte) % 256 )
                    // TODO(nnarain): This doesn't work?
                    // state = if checksum % 256 == 255 {
                    //     State::TopicIdLsb
                    // }
                    // else {
                    //     State::Sync
                    // };
                    self.state = State::TopicIdLsb;
                },
                State::TopicIdLsb => {
                    self.topic = data as u16;
                    self.checksum = data as u16;
                    self.state = State::TopicIdMsb;
                },
                State::TopicIdMsb => {
                    self.topic |= (data as u16) << 8;
                    self.state = if self.bytes == 0 { State::MessageChecksum } else { State::Message };
                },
                State::Message => {
                    self.message_in[self.index] = data;
                    self.index += 1;
                    self.bytes -= 1;

                    if self.bytes == 0 {
                        self.state = State::MessageChecksum;
                    }
                },
                State::MessageChecksum => {
                    // TODO: checksum

                    if self.topic == rosserial_msgs::TOPICINFO_ID_PUBLISHER {
                        self.request_sync_time(hardware);
                        self.negotiate_topics(hardware);

                        self.configured = true;
                    }
                    else if self.topic == rosserial_msgs::TOPICINFO_ID_TIME {
                        // TODO: sync time
                    }
                    else if self.topic == rosserial_msgs::TOPICINFO_ID_TX_STOP {
                        self.configured = false;
                    }
                    else {
                        let idx = self.topic - 100;
                        if (idx as usize) < self.subscribers.len() {
                            if let Some(ref mut sub) = self.subscribers[idx as usize] {
                                sub.handle_message(&self.message_in[..]);
                            }
                        }
                    }

                    self.state = State::Sync;
                },
            }
        }
    }

    pub fn request_sync_time(&self, hardware: &mut dyn HardwareInterface) {
        let time = std_msgs::Time::default();
        self.send_message(rosserial_msgs::TOPICINFO_ID_TIME, &time, hardware)
    }

    fn negotiate_topics(&self, hardware: &mut dyn HardwareInterface) {
        for p in self.publishers.iter() {
            if let Some(ref p) = p {
                let ti: rosserial_msgs::TopicInfo = (*p).into();
                self.send_message(rosserial_msgs::TOPICINFO_ID_PUBLISHER, &ti, hardware);
            }
        }

        for ti in self.subscriber_info.iter() {
            if let Some(ref ti) = ti {
                self.send_message(rosserial_msgs::TOPICINFO_ID_SUBSCRIBER, ti, hardware);
            }
        }
    }

    fn send_message(&self, topic_id: u16, msg: &dyn Message, hardware: &mut dyn HardwareInterface) {
        let mut message_out: [u8; 256] = [0; 256];

        let len = msg.serialize(&mut message_out[7..]);

        message_out[0] = 0xFF;
        message_out[1] = PROTOCOL_VER2;
        message_out[2] = (len & 0xFF) as u8;
        message_out[3] = ((len >> 8) & 0xFF) as u8;
        message_out[4] = 255 - ((message_out[2] as u16 + message_out[3] as u16) % 256) as u8;
        message_out[5] = (topic_id & 0xFF) as u8;
        message_out[6] = ((topic_id >> 8) & 0xFF) as u8;

        // TODO: Want to use iterator sum here..
        let last = 7 + len as usize;
        let mut checksum: u32 = 0;
        for b in &message_out[5..last] {
            checksum += *b as u32;
        }
        message_out[last] = (255 - (checksum as u16 % 256)) as u8;

        for data in &message_out[..last+1] {
            hardware.write(*data);
        }
    }
}
