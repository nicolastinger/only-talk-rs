use serde::{Deserialize, Serialize};
use crate::common::quic_network_service::models::quic_connection::ConnectionType;

// quic初始包
#[derive(Debug, Serialize, Deserialize)]
pub struct FirstQuicMsg {
    pub token: String,  //用户token
    pub account: String,  //用户账号
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
            account: "".to_string(),
            msg_type: ConnectionType::Text,
            text_serde_struct: "".to_string(),
            dyn_buffer_size: 0,
            dyn_header_size: 0,
            split_symbol: "".to_string(),
        }
    }
}