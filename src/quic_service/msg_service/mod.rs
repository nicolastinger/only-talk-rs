use crate::GLOBAL_QUIC_SERVER_LIST;
use anyhow::anyhow;
use quinn::SendStream;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod process_msg_service;
pub mod text_msg_service;

pub async fn get_send_stream_by_uuid(
    uuid: &String,
    connection_type: &String,
) -> Result<Arc<RwLock<SendStream>>, anyhow::Error> {
    let my_send_stream = {
        let connection_key = format!("{}{}{}{}", "QUIC:SERVER:", uuid, ":", connection_type);
        let connection_key = connection_key.to_uppercase();
        let bind = GLOBAL_QUIC_SERVER_LIST.read().await;
        let send = bind.get(&connection_key).ok_or(anyhow!("连接不可用"))?;
        send.send_stream.clone()
    };
    Ok(my_send_stream)
}
