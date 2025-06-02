use bincode::Options;
use serde::{Deserialize, Serialize};

pub trait TextMsg {
   fn get_bytes(&self) -> anyhow::Result<Vec<u8>>;
}

#[repr(u8)]
pub enum MessageType {
    Text = 1,  //文本信息
    Image = 2,  //图片信息
    File = 3,   //文件信息


    Ping = 99,  //ping信息
    RecallSuccess = 201, //接受解析成功
    RecallFailure = 202,  //接收解析失败

    P2pUserServer = 203,  //作为p2p服务端发起
    P2pUserClient = 204,  //作为p2p客户端
}

//头部消息
#[derive(Debug, Serialize, Deserialize)]
pub struct HeadMsg {
    pub version: u8,
    pub crc: u16,
    pub body_len: u32,       // 消息体长度
    pub message_type: u16,    // 消息类型, 1-好友单聊
}

//文本信息消息体
#[derive(Debug, Serialize, Deserialize)]
pub struct TextQuicMsg {
    pub id: String,
    pub text_type: u16,  //消息类型
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
