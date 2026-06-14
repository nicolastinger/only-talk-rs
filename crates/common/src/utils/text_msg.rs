use crc::Crc;
use serde::{Deserialize, Serialize};

// Create CRC-16/X25 calculator
pub const X25: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

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

// Assemble header + message body
pub fn build_text_msg<H: TextMsg, G: TextMsg>(
    text_head: &H,
    text_msg: &G,
) -> anyhow::Result<Vec<u8>> {
    let mut head_byte = text_head.get_bytes()?;
    let mut msg_byte = text_msg.get_bytes()?;
    head_byte.append(&mut msg_byte);
    Ok(head_byte)
}

fn build_text(text_quic_msg: TextQuicMsg) -> anyhow::Result<Vec<u8>> {
    let meta_data = text_quic_msg.get_bytes()?;
    let crc = X25.checksum(&meta_data);
    let head_msg = HeadMsg {
        version: 1,
        crc,
        body_len: meta_data.len() as u32,
        message_type: crate::utils::message_types::MSG_TYPE_TEXT,
    };

    build_text_msg(&head_msg, &text_quic_msg)
}

// Generate text message
pub fn generate_text_msg(
    text_type: u16,
    raw: Vec<u8>,
    recv_user: String,
    send_user: String,
) -> anyhow::Result<Vec<u8>> {
    let now = crate::utils::time::get_now_time_stamp_as_millis().unwrap_or(-99999999999);
    let text_quic_msg =
        TextQuicMsg { nano_id: nanoid::nanoid!(), text_type, raw, recv_user, send_user, timestamp: now };
    build_text(text_quic_msg)
}

// Generate text message
pub fn generate_text_msg_with_id(
    nano_id: String,
    text_type: u16,
    raw: Vec<u8>,
    recv_user: String,
    send_user: String,
) -> anyhow::Result<Vec<u8>> {
    let now = crate::utils::time::get_now_time_stamp_as_millis().unwrap_or(-99999999999);
    let text_quic_msg =
        TextQuicMsg { nano_id, text_type, raw, recv_user, send_user, timestamp: now };
    build_text(text_quic_msg)
}

// Generate text message
pub fn generate_text_msg_with_time(
    nano_id: String,
    text_type: u16,
    raw: Vec<u8>,
    recv_user: String,
    send_user: String,
    timestamp: i64,
) -> anyhow::Result<Vec<u8>> {
    let text_quic_msg = TextQuicMsg { nano_id, text_type, raw, recv_user, send_user, timestamp };
    build_text(text_quic_msg)
}
