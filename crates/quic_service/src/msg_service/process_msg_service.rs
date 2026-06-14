use std::sync::Arc;

use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::models::text_msg::TextQuicMsg;
use crate::msg_service::group_msg_service::handle_group_msg_from_client;
use crate::msg_service::text_msg_service::{
    generate_text_msg, generate_text_msg_with_time, get_text_msg,
};
use anyhow::anyhow;
use common::RBATIS_DATABASE;
use common::REDIS_CLIENT;
use common::config_str::{
    MOBILE_PLATFORM, PC_PLATFORM, PONG, REDIS_INTERNAL_QUIC_SERVERS, REDIS_QUIC_SERVERS,
    REDIS_SPLIT, SYSTEM,
};
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::utils::group_msg::GroupQuicMsg;
use common::utils::internal_quic_client::send_internal_quic_msg;
use common::utils::internal_quic_msg::{InternalQuicRequest, RequestSource};
use common::utils::message_types;
use common::utils::server_count_sync::compute_preferred_index;
use common::utils::time::get_now_time_stamp_as_millis;
use dashmap::DashMap;
use deadpool_redis::redis::AsyncCommands;
use nanoid::nanoid;
use quinn::Connection;
use rbatis::rbdc::{Bytes, Uuid};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub async fn process_rec_msg(
    buffer: &mut Vec<u8>,
    uuid: String,
    length: usize,
    connection_key: &str,
    platform: &str,
    buffer_msg: Arc<Mutex<Vec<u8>>>,
    head_length: usize,
    connections: Arc<DashMap<String, QuicConnection>>,
    server_index: u32,
) -> anyhow::Result<()> {
    let text_vec = get_text_msg(buffer, length, buffer_msg, head_length).await?;
    info!("[single chat] received client message {:?}", text_vec);
    process_text_msg(text_vec, uuid, platform, connection_key, &connections, server_index).await?;

    Ok(())
}

async fn process_text_msg(
    text_quic_msg: Vec<TextQuicMsg>,
    uuid: String,
    platform: &str,
    connection_key: &str,
    connections: &Arc<DashMap<String, QuicConnection>>,
    server_index: u32,
) -> anyhow::Result<()> {
    for mut text_msg in text_quic_msg.into_iter() {
        if uuid != text_msg.send_user {
            error!("[single chat] mismatched sender {},{}", uuid, text_msg.send_user);
            continue;
        }
        // Heartbeat message
        if text_msg.text_type == message_types::MSG_TYPE_PING {
            send_ping(connection_key, text_msg.send_user, connections).await?;
            continue;
        }

        // Group chat message: route to group chat pipeline (save to DB -> Redis lookup members -> local delivery + internal broadcast)
        if matches!(
            text_msg.text_type,
            message_types::MSG_TYPE_GROUP_TEXT
                | message_types::MSG_TYPE_GROUP_IMAGE
                | message_types::MSG_TYPE_GROUP_FILE
                | message_types::MSG_TYPE_GROUP_NOTIFICATION
        ) {
            debug!(
                "[group chat] received group message type={} group={} sender={} raw_len={}",
                text_msg.text_type,
                text_msg.recv_user,
                text_msg.send_user,
                text_msg.raw.len()
            );
            let nano_id = nanoid!();
            let ack_raw_id = text_msg.nano_id.clone();
            let ack_nano_id = nano_id.clone();
            let now = get_now_time_stamp_as_millis()?;
            let group_msg = GroupQuicMsg {
                nano_id: nano_id.clone(),
                msg_type: text_msg.text_type,
                group_uuid: text_msg.recv_user.clone(),
                send_user: text_msg.send_user.clone(),
                raw: text_msg.raw.clone(),
                timestamp: now,
            };

            let conns = connections.clone();
            let conn_key = connection_key.to_string();
            let current_user = text_msg.send_user.clone();
            tokio::spawn(async move {
                debug!(
                    "[group chat] processing group message nano_id={} group={} sender={}",
                    nano_id, group_msg.group_uuid, group_msg.send_user
                );
                if let Err(e) = handle_group_msg_from_client(group_msg, server_index, &conns).await
                {
                    error!("[group chat] failed to process group message: {}", e);
                    return;
                }
                let now = get_now_time_stamp_as_millis().unwrap_or(0);
                if let Err(e) = send_msg_record_success(
                    ack_nano_id,
                    &conn_key,
                    current_user,
                    ack_raw_id,
                    now,
                    &conns,
                    message_types::MSG_TYPE_GROUP_ACK,
                )
                .await
                {
                    error!("[group chat] failed to send ACK: {}", e);
                }
            });
            continue;
        }

        let nano_id = nanoid!();
        let ack_raw_id = text_msg.nano_id.clone();
        let ack_nano_id = nano_id.clone();
        text_msg.nano_id = nano_id;
        let now = get_now_time_stamp_as_millis()?;
        text_msg.timestamp = now;

        let text_msg_clone = text_msg.clone();
        let conn_key = connection_key.to_string();
        let conns = connections.clone();
        tokio::spawn(async move {
            let current_user = text_msg_clone.send_user.clone();
            if let Err(e) = add_user_chat_record(text_msg_clone).await {
                error!("[single chat] failed to insert message: {}", e);
            }
            // Send ACK message
            if let Err(e) = send_msg_record_success(
                ack_nano_id,
                &conn_key,
                current_user,
                ack_raw_id,
                now,
                &conns,
                message_types::MSG_TYPE_RECALL_SUCCESS,
            )
            .await
            {
                error!("[single chat] failed to send ACK: {}", e);
            }
        });
        send_msg_to_user(text_msg, platform, connections).await?;
    }

    info!("[single chat] processing complete");
    Ok(())
}

async fn send_msg_to_user(
    text_msg: TextQuicMsg,
    platform: &str,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> anyhow::Result<()> {
    let recv_user = text_msg.recv_user.clone();
    let send_user = text_msg.send_user.clone();

    let res = generate_text_msg_with_time(
        text_msg.nano_id,
        text_msg.text_type,
        text_msg.raw,
        text_msg.recv_user,
        text_msg.send_user,
        text_msg.timestamp,
    )?;

    // Compute preferred node index
    let preferred_index = compute_preferred_index(&recv_user);

    for target_platform in [PC_PLATFORM, MOBILE_PLATFORM] {
        send_msg_to_user_by_platform(
            &res,
            target_platform,
            &recv_user,
            connections,
            preferred_index,
        )
        .await?;
    }

    // Sync to own other device
    let own_preferred = compute_preferred_index(&send_user);
    if platform == PC_PLATFORM {
        send_msg_to_user_by_platform(&res, MOBILE_PLATFORM, &send_user, connections, own_preferred)
            .await?;
    } else {
        send_msg_to_user_by_platform(&res, PC_PLATFORM, &send_user, connections, own_preferred)
            .await?;
    }
    Ok(())
}

async fn send_msg_to_user_by_platform(
    res: &Vec<u8>,
    platform: &str,
    target_user: &str,
    connections: &Arc<DashMap<String, QuicConnection>>,
    preferred_index: u32,
) -> anyhow::Result<()> {
    let user_key = format!(
        "{}:{}{}{}{}",
        platform,
        REDIS_QUIC_SERVERS,
        target_user,
        REDIS_SPLIT,
        ConnectionType::Text
    );
    let user_key = user_key.to_uppercase();

    // Try local delivery
    let conn: Option<Connection> = {
        match connections.get(&user_key) {
            Some(s) => Some(s.conn.clone()),
            None => None,
        }
    };

    if let Some(conn) = conn {
        let mut send = conn.open_uni().await?;
        send.write_all(res).await?;
        send.finish().await?;
    } else {
        warn!(
            "[single chat] user not on local machine: {}, preferred node index: {}",
            user_key, preferred_index
        );
        // Not found locally -> forward to internal QUIC for two-phase routing
        warn!("[single chat] forwarding to internal QUIC: {}", user_key);

        // res 已经是 bincode 序列化的 TextQuicMsg 二进制，直接透传
        let request = InternalQuicRequest {
            msg_type: message_types::MSG_TYPE_TEXT,
            payload: res.to_vec(),
            target_user: target_user.to_string(),
            preferred_index,
            platform: platform.to_string(),
            source: RequestSource::QuicExternal,
            ttl: 3,
        };

        // 根据 preferred_index 从 Redis 获取目标节点的内网 QUIC 地址
        let redis = REDIS_CLIENT.read().await;
        if let Some(redis) = redis.as_ref() {
            let mut conn = redis.get().await?;
            let key = format!("{}{}", REDIS_INTERNAL_QUIC_SERVERS, preferred_index);
            let addr_str: Option<String> = conn.get(&key).await?;
            if let Some(addr_str) = addr_str {
                let internal_addr: std::net::SocketAddr = addr_str.parse()?;
                info!("[single chat] sending internal QUIC message to: {}", internal_addr);
                send_internal_quic_msg(internal_addr, request).await?;
            } else {
                warn!("[single chat] internal QUIC address not found for node {}", preferred_index);
            }
        }
    }
    Ok(())
}

/// Send connection heartbeat
async fn send_ping(
    connection_key: &str,
    current_user: String,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> anyhow::Result<()> {
    let ping_msg = generate_text_msg(
        message_types::MSG_TYPE_PING,
        Vec::from(PONG.as_bytes()),
        current_user,
        SYSTEM.to_string(),
    )?;

    if let Some(entry) = connections.get(connection_key) {
        let mut send = entry.conn.open_uni().await?;
        send.write_all(ping_msg.as_ref()).await?;
        send.finish().await?;
    }
    Ok(())
}

/// Send ACK message
async fn send_msg_record_success(
    nano_id: String,
    connection_key: &str,
    current_user: String,
    nanoid: String,
    timestamp: i64,
    connections: &Arc<DashMap<String, QuicConnection>>,
    ack_type: u16,
) -> anyhow::Result<()> {
    let res = generate_text_msg_with_time(
        nano_id,
        ack_type,
        nanoid.as_bytes().to_vec(),
        current_user,
        SYSTEM.to_string(),
        timestamp,
    )?;

    if let Some(entry) = connections.get(connection_key) {
        let mut send = entry.conn.open_uni().await?;
        send.write_all(&res).await?;
        send.finish().await?;
    }
    Ok(())
}

/// Record failed message
#[allow(dead_code)]
async fn send_msg_record_failure(
    connection_key: &str,
    current_user: String,
    nanoid: String,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> anyhow::Result<()> {
    let res = generate_text_msg(
        message_types::MSG_TYPE_RECALL_FAILURE,
        nanoid.as_bytes().to_vec(),
        current_user,
        SYSTEM.to_string(),
    )?;

    if let Some(entry) = connections.get(connection_key) {
        let mut send = entry.conn.open_uni().await?;
        send.write_all(&res).await?;
        send.finish().await?;
    }
    Ok(())
}

/// Add user chat record
pub async fn add_user_chat_record(text_quic_msg: TextQuicMsg) -> Result<(), anyhow::Error> {
    // TODO kafka转发消息ck批量写入
    let rb = RBATIS_DATABASE.read().await;
    let rb = rb.as_ref().ok_or(anyhow!("Failed to get database connection"))?;
    let chat_msg = ChatMessageRecord {
        id: None,
        nano_id: Some(text_quic_msg.nano_id),
        timestamp: Some(text_quic_msg.timestamp),
        raw: Bytes::from(text_quic_msg.raw),
        text_type: Some(text_quic_msg.text_type as u32),
        send_user: text_quic_msg.send_user.parse::<Uuid>()?,
        recv_user: text_quic_msg.recv_user.parse::<Uuid>()?,
    };
    ChatMessageRecord::insert(rb, &chat_msg).await?;
    Ok(())
}
