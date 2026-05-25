use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;
use common::config_str::{REDIS_QUIC_SERVERS, REDIS_SPLIT, SYSTEM};
use common::utils::internal_quic_client::send_internal_quic_msg;
use common::utils::internal_quic_msg::{InternalQuicRequest, RequestSource};
use tracing::warn;
use common::utils::server_count_sync::compute_preferred_index;
use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::msg_service::text_msg_service::generate_text_msg;

/// 针对用户发送系统消息（通过内网 QUIC 路由）
pub async fn send_quic_system_msg(
    current_user: String,
    msg_type: u16,
    text: String,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> anyhow::Result<()> {
    // 1. 先尝试本机投递（对 PC / MOBILE 两个平台）
    let preferred_index = compute_preferred_index(&current_user);

    for platform in [common::config_str::PC_PLATFORM, common::config_str::MOBILE_PLATFORM] {
        let user_key = format!(
            "{}:{}{}{}{}",
            platform,
            REDIS_QUIC_SERVERS,
            current_user.as_str(),
            REDIS_SPLIT,
            ConnectionType::Text
        );
        let user_key = user_key.to_uppercase();

        match connections.get(&user_key) {
            Some(entry) => {
                let res = generate_text_msg(
                    msg_type,
                    text.as_bytes().to_vec(),
                    current_user.clone(),
                    SYSTEM.to_string(),
                )?;
                let mut send = entry.conn.open_uni().await?;
                send.write_all(&res).await?;
                send.finish().await?;
                return Ok(());
            }
            None => {
                warn!("当前用户不在本机: {} (platform={})", user_key, platform);
            }
        }
    }

    // 2. 本机未找到 → 转发给内网 QUIC
    let request = InternalQuicRequest {
        msg_type,
        payload: text,
        target_user: current_user,
        preferred_index,
        platform: common::config_str::PC_PLATFORM.to_string(),
        source: RequestSource::HttpApi,
        ttl: 3,
    };

    let internal_addr: SocketAddr = "127.0.0.1:4434".parse()?;
    send_internal_quic_msg(internal_addr, request).await?;
    Ok(())
}
