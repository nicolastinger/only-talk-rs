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
    pub account: String,
    pub connection_type: ConnectionType,
    pub send_stream: Arc<RwLock<SendStream>>,
    pub create_time: u64,
    pub update_time: u64,
    pub ipv4addr: String,
    pub ipv6addr: String,
}