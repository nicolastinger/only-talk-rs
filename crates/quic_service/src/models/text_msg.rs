use serde::{Deserialize, Serialize};

pub trait TextMsg {
    fn get_bytes(&self) -> anyhow::Result<Vec<u8>>;
}

#[repr(u16)]
pub enum MessageType {
    Text = 1,  // Text message
    Image = 2, // Image message
    File = 3,  // File message
    P2P = 4,
    P2PVideoCall = 5,
    P2pVideoData = 6,
    P2pVideoConfig = 7,

    Ping = 99,           // Ping info
    RecallSuccess = 201, // Received and parsed successfully
    RecallFailure = 202, // Received parse failed

    P2pUserServer = 203, // Notify as P2P server (issued by server after NAT discovery)
    P2pUserClient = 204, // Notify as P2P client (issued by server after NAT discovery)
    System = 10001,      // System notification
}

// Header message
#[derive(Debug, Serialize, Deserialize)]
pub struct HeadMsg {
    pub version: u8,
    pub crc: u16,
    pub body_len: u32,     // Message body length
    pub message_type: u16, // Message type, 1 = friend DM
}

// Text message body
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextQuicMsg {
    pub nano_id: String,
    pub text_type: u16, // Message type
    pub raw: Vec<u8>,
    pub recv_user: String, // Receiving user
    pub send_user: String, // Sending user
    pub timestamp: i64,
}

impl TextMsg for HeadMsg {
    fn get_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }
}

impl TextMsg for TextQuicMsg {
    fn get_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }
}
