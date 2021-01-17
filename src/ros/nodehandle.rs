use super::HardwareInterface;

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

pub struct NodeHandle<Hardware: HardwareInterface> {
    hardware: Hardware,
    state: State,
}

impl<Hardware: HardwareInterface + Default> Default for NodeHandle<Hardware> {
    fn default() -> Self {
        NodeHandle {
            hardware: Hardware::default(),
            state: State::Sync,
        }
    }
}

impl<Hardware: HardwareInterface> NodeHandle<Hardware> {
    pub fn spinOnce(&self) {
        let current_time = self.hardware.time();

        match self.state {
            State::Sync => {},
            State::ProtocolVersion => {},
            State::SizeLsb => {},
            State::SizeMsb => {},
            State::SizeChecksum => {},
            State::TopicIdLsb => {},
            State::TopicIdMsb => {},
            State::Message => {},
            State::MessageChecksum => {},
        }
    }
}
