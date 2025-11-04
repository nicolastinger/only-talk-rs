use std::collections::HashMap;
use quinn::{ClientConfig, Endpoint, ServerConfig, TransportConfig};
use rustls::{Certificate, PrivateKey, RootCertStore};
use rustls_pemfile::{certs, ec_private_keys, rsa_private_keys};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use crc::Crc;
use lazy_static::lazy_static;
use tokio::sync::RwLock;
use crate::models::quic_connection::QuicConnection;

pub mod init_server;
pub mod msg_service;
pub(crate) mod quic_client;
pub(crate) mod quic_server;
pub mod models;
mod set_server;

// 创建CRC-16/X25计算器
const X25: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

lazy_static! {
    pub static ref GLOBAL_QUIC_SERVER_LIST: Arc<RwLock<HashMap<String, QuicConnection>>> =
        Arc::new(RwLock::new(HashMap::new()));
    }


