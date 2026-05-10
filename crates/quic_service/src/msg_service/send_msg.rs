use std::sync::Arc;

use dashmap::DashMap;
use entity::config_str::{REDIS_QUIC_SERVERS, REDIS_SPLIT, SYSTEM};
use tracing::error;

use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::msg_service::text_msg_service::generate_text_msg;

/// 针对用户发送系统消息
pub async fn send_quic_system_msg(
    current_user: String,
    msg_type: u16,
    text: String,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> anyhow::Result<()> {
    // 目标用户的连接
    let user_key = format!(
        "{}{}{}{}",
        REDIS_QUIC_SERVERS,
        current_user.as_str(),
        REDIS_SPLIT,
        ConnectionType::Text
    );
    let user_key = user_key.to_uppercase();
    let conn = {
        match connections.get(&user_key) {
            Some(s) => Some(s.conn.clone()),
            None => {
                error!("当前用户不在线: {}", user_key);
                return Ok(());
            }
        }
    };
    let res =
        generate_text_msg(msg_type, text.as_bytes().to_vec(), current_user, SYSTEM.to_string())?;
    if let Some(conn) = conn {
        let mut send = conn.open_uni().await?;
        send.write_all(&res).await?;
        send.finish().await?;
    }
    Ok(())
}
