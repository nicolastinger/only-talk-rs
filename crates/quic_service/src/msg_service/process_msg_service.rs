use std::sync::Arc;

use anyhow::anyhow;
use dashmap::DashMap;
use common::RBATIS_DATABASE;
use common::config_str::{MOBILE_PLATFORM, PC_PLATFORM, PONG, REDIS_QUIC_SERVERS, REDIS_SPLIT, SYSTEM};
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::utils::message_types;
use common::utils::time::get_now_time_stamp_as_millis;
use tracing::{error, info, warn};
use nanoid::nanoid;
use quinn::Connection;
use rbatis::rbdc::{Bytes, Uuid};
use tokio::sync::Mutex;
use common::utils::server_count_sync::compute_preferred_index;
use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::models::text_msg::TextQuicMsg;
use crate::msg_service::text_msg_service::{
    generate_text_msg, generate_text_msg_with_time, get_text_msg,
};

pub async fn process_rec_msg(
    buffer: &mut Vec<u8>,
    uuid: String,
    length: usize,
    connection_key: &str,
    platform: &str,
    buffer_msg: Arc<Mutex<Vec<u8>>>,
    head_length: usize,
    connections: Arc<DashMap<String, QuicConnection>>,
) -> anyhow::Result<()> {
    let text_vec = get_text_msg(buffer, length, buffer_msg, head_length).await?;
    info!("接收到客户端信息 {:?}", text_vec);
    process_text_msg(text_vec, uuid, platform, connection_key, &connections).await?;

    Ok(())
}

async fn process_text_msg(
    text_quic_msg: Vec<TextQuicMsg>,
    uuid: String,
    platform: &str,
    connection_key: &str,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> anyhow::Result<()> {
    for mut text_msg in text_quic_msg.into_iter() {
        if uuid != text_msg.send_user {
            error!("错误的发送人 {},{}", uuid, text_msg.send_user);
            continue;
        }
        // 心跳消息
        if text_msg.text_type == message_types::MSG_TYPE_PING {
            send_ping(connection_key, text_msg.send_user, connections).await?;
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
            add_user_chat_record(text_msg_clone).await.expect("插入用户消息失败");
            // 发送ack消息
            send_msg_record_success(ack_nano_id, &conn_key, current_user, ack_raw_id, now, &conns)
                .await
                .expect("发送ack消息失败");
        });
        send_msg_to_user(text_msg, platform, connections).await?;
    }

    info!("处理完成");
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

    // 计算首选节点序号
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

    // 给自己另一设备同步
    let own_preferred = compute_preferred_index(&send_user);
    if platform == PC_PLATFORM {
        send_msg_to_user_by_platform(
            &res,
            MOBILE_PLATFORM,
            &send_user,
            connections,
            own_preferred,
        )
        .await?;
    } else {
        send_msg_to_user_by_platform(
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

    // 尝试本机投递
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
        // 本机未找到 → 转发给内网 QUIC 走两阶段路由
        use common::utils::internal_quic_msg::{InternalQuicRequest, RequestSource};
        use common::utils::internal_quic_client::send_internal_quic_msg;
        use std::net::SocketAddr;

        warn!("当前用户不在本机: {}，转发到内网 QUIC", user_key);

        let payload = String::from_utf8_lossy(res).to_string();
        let request = InternalQuicRequest {
            msg_type: message_types::MSG_TYPE_TEXT,
            payload,
            target_user: target_user.to_string(),
            preferred_index,
            platform: platform.to_string(),
            source: RequestSource::QuicExternal,
            ttl: 3,
        };

        // 发给本机内网 QUIC
        let internal_addr: SocketAddr = "127.0.0.1:4434".parse()?;
        send_internal_quic_msg(internal_addr, request).await?;
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

/// 发送ack消息
async fn send_msg_record_success(
    nano_id: String,
    connection_key: &str,
    current_user: String,
    nanoid: String,
    timestamp: i64,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> anyhow::Result<()> {
    let res = generate_text_msg_with_time(
        nano_id,
        message_types::MSG_TYPE_RECALL_SUCCESS,
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

/// 用户新增聊天记录
pub async fn add_user_chat_record(text_quic_msg: TextQuicMsg) -> Result<(), anyhow::Error> {
    // TODO kafka转发消息ck批量写入
    let rb = RBATIS_DATABASE.read().await;
    let rb = rb.as_ref().ok_or(anyhow!("获取连接失败"))?;
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
