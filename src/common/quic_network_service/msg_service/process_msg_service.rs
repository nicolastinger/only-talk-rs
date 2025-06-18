use crate::utils::global_static_str::{PONG, REDIS_QUIC_SERVERS, REDIS_SPLIT, SYSTEM};
use crate::common::quic_network_service::models::quic_connection::ConnectionType;
use crate::common::quic_network_service::models::text_msg::{MessageType, TextQuicMsg};
use crate::common::quic_network_service::msg_service::text_msg_service::{generate_text_msg, get_text_msg};
use crate::GLOBAL_QUIC_SERVER_LIST;
use anyhow::{anyhow};
use log::{error, info};
use quinn::{SendStream};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use crate::common::service::user_service::add_user_chat_record;

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
    uuid: String
) -> anyhow::Result<()> {
    for text_msg in text_quic_msg.into_iter() {
        if uuid != text_msg.send_user {
            error!("错误的发送人 {},{}", uuid, text_msg.send_user);
            continue;
        }
        // 心跳消息
        if text_msg.text_type == MessageType::Ping as u16 {
            // 发送ping
            send_ping(send_stream.clone(), text_msg.send_user).await?;
            continue;
        }

        let user_key = format!(
            "{}{}{}{}",
            REDIS_QUIC_SERVERS,
            text_msg.recv_user.as_str(),
            REDIS_SPLIT,
            ConnectionType::Text.to_string()
        );
        let user_key = user_key.to_uppercase();

        let text_msg_clone = text_msg.clone();
        tokio::spawn(async move {
            add_user_chat_record(text_msg_clone).await.expect("插入用户消息失败");
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
            // 判断用户权限发送消息
            pass_text_msg(target_send_stream.clone(), send_stream.clone(), text_msg).await?;
        } else {
            // 处理 my_send_stream 为 None 的情况
            info!("用户不在线，无法发送消息: {}", user_key);
            // TODO这里可以添加其他处理逻辑
        }
    }

    Ok(())
}

/// 发送连接心跳
async fn send_ping(
    send_stream: Arc<RwLock<SendStream>>,
    current_user: String,
) -> anyhow::Result<()> {
    let ping_msg = generate_text_msg(
        MessageType::Ping as u16,
        Vec::from(PONG.as_bytes()),
        current_user,
        SYSTEM.to_string()
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
    current_send_stream: Arc<RwLock<SendStream>>,
    text_msg: TextQuicMsg,
) -> anyhow::Result<()> {
    let nanoid = text_msg.id;
    let current_user = text_msg.send_user.clone();
    let res = generate_text_msg(
        text_msg.text_type,
        text_msg.raw,
        text_msg.recv_user,
        text_msg.send_user,
    )?;
    tokio::spawn(async move {
        {
            send_msg_permissions().await.expect("鉴权失败");

            let current_send_stream = current_send_stream.clone();
            let res= recv_send_stream
                .write()
                .await
                .write_all(&res)
                .await;
            match res {
                Ok(_) => {
                    send_msg_record_success(current_send_stream, current_user, nanoid).await.expect("记录消息失败");
                }
                Err(e) => {
                    error!("发送消息失败 {}", e.to_string());
                    send_msg_record_failure(current_send_stream, current_user, nanoid).await.expect("记录消息失败");
                }
            }
        }
    });
    Ok(())
}

/// 判断是否有权限发送消息
async fn send_msg_permissions() -> anyhow::Result<bool> {
    // TODO
    Ok(true)
}

/// 记录发送消息
async fn send_msg_record_success(send_stream: Arc<RwLock<SendStream>>, current_user: String, nanoid: String) -> anyhow::Result<()> {
    let res = generate_text_msg(MessageType::RecallSuccess as u16, nanoid.as_bytes().to_vec(), current_user, SYSTEM.to_string())?;
    send_stream.write().await.write_all(&res).await?;
    Ok(())
}

/// 记录失败消息
async fn send_msg_record_failure (send_stream: Arc<RwLock<SendStream>>, current_user: String, nanoid: String) -> anyhow::Result<()> {
    let res = generate_text_msg(MessageType::RecallFailure as u16, nanoid.as_bytes().to_vec(), current_user, SYSTEM.to_string())?;
    send_stream.write().await.write_all(&res).await?;
    Ok(())
}
