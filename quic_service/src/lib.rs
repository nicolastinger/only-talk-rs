use std::collections::HashMap;
use std::sync::Arc;

use crc::Crc;
use lazy_static::lazy_static;
use tokio::sync::RwLock;

use crate::models::quic_connection::QuicConnection;

pub mod init_server;
pub mod models;
pub mod msg_service;
pub mod p2p_service;
pub(crate) mod quic_client;
pub(crate) mod quic_server;
mod set_server;

// 创建CRC-16/X25计算器
const X25: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

lazy_static! {
    pub static ref GLOBAL_QUIC_SERVER_LIST: Arc<RwLock<HashMap<String, QuicConnection>>> =
        Arc::new(RwLock::new(HashMap::new()));
}
