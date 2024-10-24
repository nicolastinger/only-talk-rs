use std::sync::Arc;
use quinn::{RecvStream, SendStream, Connection};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize)]
pub enum ConnectionType {
    Text,
    Img,
    Video,
    File,
    Other,
}

//quic服务器
#[derive(Debug)]
pub struct QuicConnection {
    pub is_online: bool,
    pub connection: Connection,
    pub user_id: String,
    pub connection_type: ConnectionType,
    pub send_stream: Arc<RwLock<SendStream>>,
    pub create_time: i64,
    pub update_time: i64,
    pub ipv4addr: String,
    pub ipv6addr: String,
    pub first_quic_msg: FirstQuicMsg
}


// quic初始包
#[derive(Debug, Serialize, Deserialize)]
pub struct FirstQuicMsg {
    pub token: String,  //用户token
    pub user_id: String,  //用户id
    pub msg_type: String,  //流数据类型,文字，图文，视频，其他实现。
    pub text_serde_struct: String,  //文字类型序列化的struct
    pub dyn_buffer_size: usize,  //缓冲区大小
    pub file_size: usize,  //图文大小
    pub file_name: String,  //图文名称
    pub raw_head: String  //视频帧头部
}