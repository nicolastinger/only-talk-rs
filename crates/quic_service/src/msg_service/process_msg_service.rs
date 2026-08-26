use std::sync::Arc;

use common::config_str::{
    MOBILE_PLATFORM, PC_PLATFORM, PONG, REDIS_INTERNAL_QUIC_SERVERS, REDIS_QUIC_SERVERS,
    REDIS_SPLIT, SYSTEM,
};
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::state::CoreState;
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

use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::models::text_msg::TextQuicMsg;
use crate::msg_service::group_msg_service::handle_group_msg_from_client;
use crate::msg_service::text_msg_service::{
    generate_text_msg, generate_text_msg_with_time, get_text_msg,
};

#[allow(clippy::too_many_arguments)]
pub async fn process_rec_msg(
    core: &CoreState,
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
    info!("[单聊] 收到客户端消息 {:?}", text_vec);
    process_text_msg(core, text_vec, uuid, platform, connection_key, &connections, server_index)
        .await?;

    Ok(())
}

async fn process_text_msg(
    core: &CoreState,
    text_quic_msg: Vec<TextQuicMsg>,
    uuid: String,
    platform: &str,
    connection_key: &str,
    connections: &Arc<DashMap<String, QuicConnection>>,
    server_index: u32,
) -> anyhow::Result<()> {
    for mut text_msg in text_quic_msg.into_iter() {
        if uuid != text_msg.send_user {
            error!("[单聊] 发送者不匹配 {},{}", uuid, text_msg.send_user);
            continue;
        }
        // 心跳消息
        if text_msg.text_type == message_types::MSG_TYPE_PING {
            send_ping(connection_key, text_msg.send_user, connections).await?;
            continue;
        }

        // 群聊消息:路由到群聊处理流程(保存到数据库 -> Redis 查询成员 -> 本地投递 + 内部广播)
        if matches!(
            text_msg.text_type,
            message_types::MSG_TYPE_GROUP_TEXT
                | message_types::MSG_TYPE_GROUP_IMAGE
                | message_types::MSG_TYPE_GROUP_FILE
                | message_types::MSG_TYPE_GROUP_NOTIFICATION
        ) {
            debug!(
                "[群聊] 收到群聊消息 type={} group={} sender={} raw_len={}",
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
            let core_clone = core.clone();
            tokio::spawn(async move {
                debug!(
                    "[群聊] 处理群聊消息 nano_id={} group={} sender={}",
                    nano_id, group_msg.group_uuid, group_msg.send_user
                );
                if let Err(e) =
                    handle_group_msg_from_client(&core_clone, group_msg, server_index, &conns).await
                {
                    error!("[群聊] 处理群聊消息失败: {}", e);
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
                    error!("[群聊] 发送 ACK 失败: {}", e);
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
        let core_clone = core.clone();
        tokio::spawn(async move {
            let current_user = text_msg_clone.send_user.clone();
            // WebRTC 信令(100)与视频通话控制消息(12-15)是呼叫建立期的瞬态消息：
            // 服务端仅转发、不持久化；信令(100)不回 ACK（前端独立发送命令不依赖发送/回执表），
            // 控制消息(12-15)仍回 ACK，因为它们仍走 send_text_msg 发送流程。
            let should_ack = text_msg_clone.text_type != message_types::MSG_TYPE_WEBRTC_SIGNAL;
            match text_msg_clone.text_type {
                message_types::MSG_TYPE_WEBRTC_SIGNAL
                | message_types::MSG_TYPE_P2P_VIDEO_CALL_INVITE
                | message_types::MSG_TYPE_P2P_VIDEO_CALL_ACCEPT
                | message_types::MSG_TYPE_P2P_VIDEO_CALL_REJECT
                | message_types::MSG_TYPE_P2P_VIDEO_CALL_END => {}
                _ => {
                    if let Err(e) = add_user_chat_record(&core_clone, text_msg_clone).await {
                        error!("[单聊] 插入消息失败: {}", e);
                    }
                }
            }
            // 发送 ACK 消息（信令不回执，其余类型仍回执）
            if should_ack
                && let Err(e) = send_msg_record_success(
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
                error!("[单聊] 发送 ACK 失败: {}", e);
            }
        });
        send_msg_to_user(core, text_msg, platform, connections).await?;
    }

    info!("[单聊] 处理完成");
    Ok(())
}

async fn send_msg_to_user(
    core: &CoreState,
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

    // 计算首选节点索引
    let preferred_index = compute_preferred_index(&recv_user);

    for target_platform in [PC_PLATFORM, MOBILE_PLATFORM] {
        send_msg_to_user_by_platform(
            core,
            &res,
            target_platform,
            &recv_user,
            connections,
            preferred_index,
        )
        .await?;
    }

    // 同步到自己的另一个设备
    let own_preferred = compute_preferred_index(&send_user);
    if platform == PC_PLATFORM {
        send_msg_to_user_by_platform(
            core,
            &res,
            MOBILE_PLATFORM,
            &send_user,
            connections,
            own_preferred,
        )
        .await?;
    } else {
        send_msg_to_user_by_platform(
            core,
            &res,
            PC_PLATFORM,
            &send_user,
            connections,
            own_preferred,
        )
        .await?;
    }
    Ok(())
}

async fn send_msg_to_user_by_platform(
    core: &CoreState,
    res: &[u8],
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

    // 尝试本地投递
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
        warn!("[单聊] 用户不在本机: {},首选节点索引: {}", user_key, preferred_index);
        // 本地未找到 -> 转发到内部 QUIC 进行两阶段路由
        warn!("[单聊] 转发到内部 QUIC: {}", user_key);

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
        let mut conn = core.redis.get().await?;
        let key = format!("{}{}", REDIS_INTERNAL_QUIC_SERVERS, preferred_index);
        let addr_str: Option<String> = conn.get(&key).await?;
        if let Some(addr_str) = addr_str {
            let internal_addr: std::net::SocketAddr = addr_str.parse()?;
            info!("[单聊] 发送内部 QUIC 消息到: {}", internal_addr);
            send_internal_quic_msg(internal_addr, request).await?;
        } else {
            warn!("[单聊] 未找到节点 {} 的内部 QUIC 地址", preferred_index);
        }
    }
    Ok(())
}

/// 发送连接心跳
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

/// 发送 ACK 消息
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

/// 记录失败消息
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

/// 添加用户聊天记录
pub async fn add_user_chat_record(
    core: &CoreState,
    text_quic_msg: TextQuicMsg,
) -> Result<(), anyhow::Error> {
    // TODO kafka转发消息ck批量写入
    let rb = &core.db;
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
