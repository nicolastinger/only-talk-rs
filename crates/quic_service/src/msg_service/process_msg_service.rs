use std::sync::Arc;

use anyhow::anyhow;
use dashmap::DashMap;
use entity::RBATIS_DATABASE;
use entity::config_str::{MOBILE_PLATFORM, PC_PLATFORM, PONG, REDIS_QUIC_SERVERS, REDIS_SPLIT, SYSTEM};
use entity::models::chat_entity::chat_message_record::ChatMessageRecord;
use entity::utils::message_types;
use entity::utils::time::get_now_time_stamp_as_millis;
use tracing::{error, info, warn};
use nanoid::nanoid;
use quinn::SendStream;
use rbatis::rbdc::{Bytes, Uuid};
use tokio::sync::{Mutex, RwLock};

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
    let my_send_stream = {
        let send = connections.get(connection_key).ok_or(anyhow!("连接不可用"))?;
        send.send_stream.clone()
    };

    let text_vec = get_text_msg(buffer, length, buffer_msg, head_length).await?;
    info!("接收到客户端信息 {:?}", text_vec);
    process_text_msg(my_send_stream, text_vec, uuid, platform, &connections).await?;

    Ok(())
}

async fn process_text_msg(
    send_stream: Arc<RwLock<SendStream>>,
    text_quic_msg: Vec<TextQuicMsg>,
    uuid: String,
    platform: &str,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> anyhow::Result<()> {
    for mut text_msg in text_quic_msg.into_iter() {
        if uuid != text_msg.send_user {
            error!("错误的发送人 {},{}", uuid, text_msg.send_user);
            continue;
        }
        // 心跳消息
        if text_msg.text_type == message_types::MSG_TYPE_PING {
            // 发送ping
            send_ping(send_stream.clone(), text_msg.send_user).await?;
            continue;
        }

        let nano_id = nanoid!();
        let ack_raw_id = text_msg.nano_id.clone();
        let ack_nano_id = nano_id.clone();
        text_msg.nano_id = nano_id;
        let now = get_now_time_stamp_as_millis()?;
        text_msg.timestamp = now;


        let text_msg_clone = text_msg.clone();
        let send_stream_clone = send_stream.clone();
        tokio::spawn(async move {
            let current_user = text_msg_clone.send_user.clone();
            add_user_chat_record(text_msg_clone).await.expect("插入用户消息失败");
            // 发送ack消息
            send_msg_record_success(ack_nano_id, send_stream_clone, current_user, ack_raw_id, now)
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
    send_msg_to_user_by_platform(&res, PC_PLATFORM, &recv_user, connections).await?;
    send_msg_to_user_by_platform(&res, MOBILE_PLATFORM, &recv_user, connections).await?;
    if platform == PC_PLATFORM {
        send_msg_to_user_by_platform(&res, MOBILE_PLATFORM, &send_user, connections).await?;
    } else {
        send_msg_to_user_by_platform(&res, PC_PLATFORM, &send_user, connections).await?;
    }
    Ok(())
}

async fn send_msg_to_user_by_platform(
    res: &Vec<u8>,
    platform: &str,
    target_user: &str,
    connections: &Arc<DashMap<String, QuicConnection>>,
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

    // 目标用户的发送流
    let target_send_stream: Option<Arc<RwLock<SendStream>>> = {
        match connections.get(&user_key) {
            Some(s) => Some(s.send_stream.clone()),
            None => {
                warn!("当前用户不在线: {}", user_key);
                None
            }
        }
    };

    if let Some(target_send_stream) = target_send_stream {
        // 发送消息
        target_send_stream.write().await.write_all(res).await.expect("发送消息失败");
    }
    Ok(())
}

/// 发送连接心跳
async fn send_ping(
    send_stream: Arc<RwLock<SendStream>>,
    current_user: String,
) -> anyhow::Result<()> {
    let ping_msg = generate_text_msg(
        message_types::MSG_TYPE_PING,
        Vec::from(PONG.as_bytes()),
        current_user,
        SYSTEM.to_string(),
    )?;
    send_stream.write().await.write_all(ping_msg.as_ref()).await?;
    Ok(())
}

/// 传递用户消息
async fn pass_text_msg(
    recv_send_stream: Arc<RwLock<SendStream>>,
    text_msg: TextQuicMsg,
) -> anyhow::Result<()> {
    
    
    Ok(())
}


/// 发送ack消息
async fn send_msg_record_success(
    nano_id: String,
    send_stream: Arc<RwLock<SendStream>>,
    current_user: String,
    nanoid: String,
    timestamp: i64,
) -> anyhow::Result<()> {
    let res = generate_text_msg_with_time(
        nano_id,
        message_types::MSG_TYPE_RECALL_SUCCESS,
        nanoid.as_bytes().to_vec(),
        current_user,
        SYSTEM.to_string(),
        timestamp,
    )?;
    send_stream.write().await.write_all(&res).await?;
    Ok(())
}

/// 记录失败消息
#[allow(dead_code)]
async fn send_msg_record_failure(
    send_stream: Arc<RwLock<SendStream>>,
    current_user: String,
    nanoid: String,
) -> anyhow::Result<()> {
    let res = generate_text_msg(
        message_types::MSG_TYPE_RECALL_FAILURE,
        nanoid.as_bytes().to_vec(),
        current_user,
        SYSTEM.to_string(),
    )?;
    send_stream.write().await.write_all(&res).await?;
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
