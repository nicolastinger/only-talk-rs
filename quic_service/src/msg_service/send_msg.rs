use entity::config_str::{REDIS_QUIC_SERVERS, REDIS_SPLIT, SYSTEM};
use tracing::error;

use crate::GLOBAL_QUIC_SERVER_LIST;
use crate::models::quic_connection::ConnectionType;
use crate::msg_service::text_msg_service::generate_text_msg;

/// 针对用户发送系统消息
pub async fn send_quic_system_msg(
    current_user: String,
    msg_type: u16,
    text: String,
) -> anyhow::Result<()> {
    // 目标用户的发送流
    let user_key = format!(
        "{}{}{}{}",
        REDIS_QUIC_SERVERS,
        current_user.as_str(),
        REDIS_SPLIT,
        ConnectionType::Text
    );
    let user_key = user_key.to_uppercase();
    let send_stream = {
        let bind = GLOBAL_QUIC_SERVER_LIST.read().await;
        match bind.get(&user_key) {
            Some(s) => Some(s.send_stream.clone()),
            None => {
                error!("当前用户不在线: {}", user_key);
                return Ok(());
            }
        }
    };
    let res =
        generate_text_msg(msg_type, text.as_bytes().to_vec(), current_user, SYSTEM.to_string())?;
    if let Some(target_send_stream) = send_stream {
        // 处理在线消息
        target_send_stream.write().await.write_all(&res).await?;
    }
    Ok(())
}
