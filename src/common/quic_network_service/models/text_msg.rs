use bincode::Options;
use serde::{Deserialize, Serialize};

pub trait TextMsg {
   fn get_bytes(&self) -> anyhow::Result<Vec<u8>>;
}

#[repr(u8)]
pub enum MessageType {
    Text = 1,
    Image = 2,
    File = 3,


    Ping = 99,
}

//头部消息
#[derive(Debug, Serialize, Deserialize)]
pub struct HeadMsg {
    pub version: u8,
    pub crc: u16,
    pub body_len: u32,       // 消息体长度
    pub message_type: u8,    // 消息类型, 1-好友单聊
}

//文本信息消息体
#[derive(Debug, Serialize, Deserialize)]
pub struct TextQuicMsg {
    pub text_type: u8,
    pub raw: String,
    pub recv_user: String,  //接收用户
    pub send_user: String,   //发送用户
    pub timestamp: i64
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
