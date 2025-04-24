use std::sync::Arc;
use quinn::{SendStream, Connection};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConnectionType {
    Text,
    Img,
    Video,
    File,
    Other
}

impl ConnectionType {
    pub fn to_string(&self) -> String {
        match self {
            ConnectionType::Text => "text".to_string(),
            ConnectionType::Img => "img".to_string(),
            ConnectionType::Video => "video".to_string(),
            ConnectionType::File => "file".to_string(),
            ConnectionType::Other => "other".to_string()
        }
    }
}

//quic服务器
#[derive(Debug)]
pub struct QuicConnection {
    pub is_online: bool,
    pub user_id: String,
    pub connection_type: ConnectionType,
    pub send_stream: Arc<RwLock<SendStream>>,
    pub create_time: u64,
    pub update_time: u64,
    pub ipv4addr: String,
    pub ipv6addr: String,
}

// quic初始包
#[derive(Debug, Serialize, Deserialize)]
pub struct FirstQuicMsg {
    pub token: String,  //用户token
    pub user_id: String,  //用户id
    pub msg_type: ConnectionType,  //流数据类型,文字，图文，视频，其他实现。
    pub text_serde_struct: String,  //文字类型序列化的struct
    pub dyn_buffer_size: usize,  //缓冲区大小
    pub dyn_header_size: usize,  //头部大小
    pub split_symbol: String,  //消息分隔符
}

impl FirstQuicMsg {
    pub(crate) fn new() ->FirstQuicMsg{
        FirstQuicMsg {
            token: "".to_string(),
            user_id: "".to_string(),
            msg_type: ConnectionType::Text,
            text_serde_struct: "".to_string(),
            dyn_buffer_size: 0,
            dyn_header_size: 0,
            split_symbol: "".to_string(),
        }
    }
}