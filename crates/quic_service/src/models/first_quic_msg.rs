use serde::{Deserialize, Serialize};

use crate::models::quic_connection::ConnectionType;

// QUIC 初始数据包
#[derive(Debug, Serialize, Deserialize)]
pub struct FirstQuicMsg {
    pub token: String,             // 用户 token
    pub uuid: String,              // 用户账号
    pub msg_type: ConnectionType,  // 流数据类型：文本、图片、视频、其他
    pub text_serde_struct: String, // 文本类型序列化结构体
    pub dyn_buffer_size: usize,    // 缓冲区大小
    pub dyn_header_size: usize,    // 头部大小
}

impl FirstQuicMsg {
    pub(crate) fn new() -> FirstQuicMsg {
        FirstQuicMsg {
            token: "".to_string(),
            uuid: "".to_string(),
            msg_type: ConnectionType::Text,
            text_serde_struct: "".to_string(),
            dyn_buffer_size: 0,
            dyn_header_size: 0,
        }
    }
}
