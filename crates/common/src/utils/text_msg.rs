use crc::Crc;
use serde::{Deserialize, Serialize};

// 创建 CRC-16/X25 计算器
pub const X25: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

pub trait TextMsg {
    fn get_bytes(&self) -> anyhow::Result<Vec<u8>>;
}

#[repr(u16)]
pub enum MessageType {
    Text = 1,  // 文本消息
    Image = 2, // 图片消息
    File = 3,  // 文件消息
    P2P = 4,
    P2PVideoCall = 5,
    P2pVideoData = 6,
    P2pVideoConfig = 7,

    Ping = 99,           // Ping 信息
    RecallSuccess = 201, // 已收到且解析成功
    RecallFailure = 202, // 已收到但解析失败

    P2pUserServer = 203, // 通知作为 P2P 服务端(NAT 发现后由服务端下发)
    P2pUserClient = 204, // 通知作为 P2P 客户端(NAT 发现后由服务端下发)
    System = 10001,      // 系统通知
}

// 头部消息
#[derive(Debug, Serialize, Deserialize)]
pub struct HeadMsg {
    pub version: u8,
    pub crc: u16,
    pub body_len: u32,     // 消息体长度
    pub message_type: u16, // 消息类型, 1 = 好友私聊
}

// 文本消息体
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextQuicMsg {
    pub nano_id: String,
    pub text_type: u16, // 消息类型
    pub raw: Vec<u8>,
    pub recv_user: String, // 接收用户
    pub send_user: String, // 发送用户
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

// 组装头部 + 消息体
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

// 生成文本消息
pub fn generate_text_msg(
    text_type: u16,
    raw: Vec<u8>,
    recv_user: String,
    send_user: String,
) -> anyhow::Result<Vec<u8>> {
    let now = crate::utils::time::get_now_time_stamp_as_millis().unwrap_or(-99999999999);
    let text_quic_msg = TextQuicMsg {
        nano_id: nanoid::nanoid!(),
        text_type,
        raw,
        recv_user,
        send_user,
        timestamp: now,
    };
    build_text(text_quic_msg)
}

// 生成文本消息(指定消息 ID)
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

// 生成文本消息(指定时间戳)
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
