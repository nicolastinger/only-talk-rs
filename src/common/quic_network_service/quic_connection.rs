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
    pub recv_stream: Arc<RwLock<RecvStream>>,
    pub create_time: i64,
    pub update_time: i64,
    pub ipv4addr: String,
    pub ipv6addr: String,
}

impl QuicConnection {

}

// quic初始包
#[derive(Debug, Serialize, Deserialize)]
pub struct FirstQuicMsg {
    pub ipv4addr: String,  //客户端的ipv4地址
    pub ipv6addr: String,  //客户端的ipv6地址
}