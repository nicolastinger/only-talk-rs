use bincode::Options;
use serde::{Deserialize, Serialize};

pub trait TextMsg {
   fn get_bytes(&self) -> anyhow::Result<Vec<u8>>;
}

//头部消息
#[derive(Debug, Serialize, Deserialize)]
pub struct HeadMsg {
    pub(crate) body_len: u64,       // 消息体长度
    pub(crate) message_type: u8,    // 消息类型, 1-好友单聊
}

//文本信息消息体
#[derive(Debug, Serialize, Deserialize)]
pub struct TextQuicMsg {
    pub text_type: String,
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
