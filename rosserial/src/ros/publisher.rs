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
