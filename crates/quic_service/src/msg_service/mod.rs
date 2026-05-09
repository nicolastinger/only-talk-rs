use std::sync::Arc;

use anyhow::anyhow;
use dashmap::DashMap;
use quinn::SendStream;
use tokio::sync::RwLock;
use entity::config_str::PC_PLATFORM;

use crate::models::quic_connection::QuicConnection;

pub mod process_msg_service;
pub mod send_msg;
pub mod text_msg_service;

pub async fn get_send_stream_by_uuid(
    uuid: &String,
    connection_type: &String,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> Result<Arc<RwLock<SendStream>>, anyhow::Error> {
    let my_send_stream = {
        let connection_key = format!("{}{}{}{}{}",PC_PLATFORM.to_string(), ":QUIC:SERVER:", uuid, ":", connection_type);
        let connection_key = connection_key.to_uppercase();
        let send = connections.get(&connection_key).ok_or(anyhow!("连接不可用"))?;
        send.send_stream.clone()
    };
    Ok(my_send_stream)
}
