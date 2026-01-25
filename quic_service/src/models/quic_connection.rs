use std::fmt;
use std::sync::Arc;

use quinn::SendStream;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConnectionType {
    Text,
    Img,
    Video,
    File,
    Other,
}

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConnectionType::Text => write!(f, "text"),
            ConnectionType::Img => write!(f, "img"),
            ConnectionType::Video => write!(f, "video"),
            ConnectionType::File => write!(f, "file"),
            ConnectionType::Other => write!(f, "other"),
        }
    }
}

//quic服务器
#[derive(Debug)]
pub struct QuicConnection {
    pub is_online: bool,
    pub uuid: String,
    pub connection_type: ConnectionType,
    pub send_stream: Arc<RwLock<SendStream>>,
    pub create_time: u64,
    pub update_time: u64,
    pub ipv4addr: String,
    pub ipv6addr: String,
}
