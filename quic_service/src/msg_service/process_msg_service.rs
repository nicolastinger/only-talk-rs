use crate::GLOBAL_QUIC_SERVER_LIST;
use anyhow::anyhow;
use log::{error, info};
use nanoid::nanoid;
use quinn::SendStream;
use std::sync::Arc;
use rbatis::rbdc::{Bytes, Uuid};
use tokio::sync::{Mutex, RwLock};
use entity::models::chat_entity::chat_message_record::ChatMessageRecord;
use entity::RBATIS_DATABASE;
use entity::utils::global_static_str::{PONG, REDIS_QUIC_SERVERS, REDIS_SPLIT, SYSTEM};
use entity::utils::message_types;
use entity::utils::time::get_now_time_stamp_as_millis;
use crate::models::quic_connection::ConnectionType;
use crate::models::text_msg::TextQuicMsg;
use crate::msg_service::text_msg_service::{generate_text_msg, generate_text_msg_with_time, get_text_msg};

pub async fn process_rec_msg(
    buffer: &mut Vec<u8>,
    uuid: String,
    length: usize,
    close_key: String,
    msg_type: &ConnectionType,
    buffer_msg: Arc<Mutex<Vec<u8>>>,
    head_length: usize,
) -> anyhow::Result<()> {
    let my_send_stream = {
        let bind = GLOBAL_QUIC_SERVER_LIST.read().await;
        let send = bind.get(&close_key).ok_or(anyhow!("连接不可用"))?;
        send.send_stream.clone()
    };

    match msg_type {
        // 文本消息，用户请求消息，比如发起视频请求，p2p请求
        ConnectionType::Text => {
            let text_vec = get_text_msg(buffer, length, buffer_msg, head_length).await?;
            info!("接收到客户端信息 {:?}", text_vec);
            process_text_msg(my_send_stream, text_vec, uuid).await?;
        }
        // 图片消息
        ConnectionType::Img => {}
        ConnectionType::Video => {}
        ConnectionType::File => {}
        ConnectionType::Other => {}
    }
    Ok(())
}

async fn process_text_msg(
    send_stream: Arc<RwLock<SendStream>>,
    text_quic_msg: Vec<TextQuicMsg>,
    uuid: String,
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

        let user_key = format!(
            "{}{}{}{}",
            REDIS_QUIC_SERVERS,
            text_msg.recv_user.as_str(),
            REDIS_SPLIT,
            ConnectionType::Text.to_string()
        );
        let user_key = user_key.to_uppercase();

        let text_msg_clone = text_msg.clone();
        let send_stream_clone = send_stream.clone();
        tokio::spawn(async move {
            let current_user = text_msg_clone.send_user.clone();
            add_user_chat_record(text_msg_clone)
                .await
                .expect("插入用户消息失败");
            // 发送ack消息
            send_msg_record_success(
                ack_nano_id,
                send_stream_clone,
                current_user,
                ack_raw_id,
                now,
            )
            .await
            .expect("发送ack消息失败");
        });

        // 目标用户的发送流
        let target_send_stream: Option<Arc<RwLock<SendStream>>> = {
            let bind = GLOBAL_QUIC_SERVER_LIST.read().await;
            match bind.get(&user_key) {
                Some(s) => Some(s.send_stream.clone()),
                None => {
                    error!("当前用户不在线: {}", user_key);
                    None
                }
            }
        };

        if let Some(target_send_stream) = target_send_stream {
            // 处理在线消息
            pass_text_msg(target_send_stream.clone(), text_msg).await?;
        } else {
            // 处理 my_send_stream 为 None 的情况
            info!("用户不在线，无法发送消息: {}", user_key);
            // TODO这里可以添加其他处理逻辑
        }
    }

    info!("处理完成");
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
    send_stream
        .write()
        .await
        .write_all(ping_msg.as_ref())
        .await?;
    Ok(())
}

/// 传递用户消息
async fn pass_text_msg(
    recv_send_stream: Arc<RwLock<SendStream>>,
    text_msg: TextQuicMsg,
) -> anyhow::Result<()> {
    let res = generate_text_msg_with_time(
        text_msg.nano_id,
        text_msg.text_type,
        text_msg.raw,
        text_msg.recv_user,
        text_msg.send_user,
        text_msg.timestamp,
    )?;
    send_msg_permissions().await.expect("鉴权失败");
    {
        recv_send_stream
            .write()
            .await
            .write_all(&res)
            .await
            .expect("发送消息失败");
    };
    Ok(())
}

/// 判断是否有权限发送消息
async fn send_msg_permissions() -> anyhow::Result<bool> {
    // TODO
    Ok(true)
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
