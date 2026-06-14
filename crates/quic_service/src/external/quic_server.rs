use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::config::ChatNodeConfig;
use crate::models::first_quic_msg::FirstQuicMsg;
use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::msg_service::process_msg_service::process_rec_msg;
use anyhow::{Result, anyhow};
use common::REDIS_CLIENT;
use common::config_str::USER_READ_MSG;
use common::models::chat_entity::chat_message_read::ChatMessageRecordRead;
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::utils::jwt_util::{Claims, verify_token};
use common::utils::redis_utils::get_redis_conn;
use common::utils::sql_utils::get_sql_client;
use common::utils::time::get_now_time_stamp_as_millis;
use dashmap::DashMap;
use deadpool_redis::redis::AsyncCommands;
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use rbatis::dark_std::err;
use rbs::value;
use tokio::sync::{Mutex, watch};
use tracing::{error, info, warn};

/// Start and run the QUIC server, continuously listening for new connections
pub(crate) async fn run_server(
    endpoint: Arc<Endpoint>,
    connections: Arc<DashMap<String, QuicConnection>>,
    config: ChatNodeConfig,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    info!("QUIC server started successfully, address: {}", config.bind_address);

    loop {
        let incoming_conn = {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    info!("received shutdown signal, stopping new connections");
                    return;
                }
                result = endpoint.accept() => {
                    match result {
                        Some(conn) => conn,
                        None => {
                            error!("failed to accept new connection: endpoint closed");
                            return;
                        }
                    }
                }
            }
        };

        let conn = match incoming_conn.await {
            Ok(t) => t,
            Err(e) => {
                error!("failed to establish connection {}", e);
                continue;
            }
        };

        info!("[server] Connection accepted: address={}", conn.remote_address());
        let conns = connections.clone();
        let cfg = config.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(conn, conns, cfg).await {
                error!("failed to open bi-directional stream: {}", e);
            }
        });
    }
}

async fn handle_connection(
    quic_conn: Connection,
    connections: Arc<DashMap<String, QuicConnection>>,
    config: ChatNodeConfig,
) -> Result<(), anyhow::Error> {
    info!("new connection source: {:?}", quic_conn.remote_address());

    loop {
        match quic_conn.accept_bi().await {
            Ok((send_stream, recv_stream)) => {
                let address = quic_conn.remote_address().to_string().clone();
                let conns = connections.clone();
                let cfg = config.clone();
                let conn_handle = quic_conn.clone();
                tokio::spawn(async move {
                    handle_conn(send_stream, recv_stream, conn_handle, address, conns, cfg)
                        .await
                        .unwrap_or_else(|x| error!("failed to initialize connection {}", x));
                });
            }
            Err(e) => {
                error!("Connection error: {:?}", e);
                break;
            }
        }
    }
    Ok(())
}

/// Process metadata
async fn process_first_msg(
    send_stream: &mut SendStream,
    recv_stream: &mut RecvStream,
    address: &String,
) -> Result<FirstQuicMsg, anyhow::Error> {
    // Receive stream metadata, confirm message type and header length
    let mut _first_quic_msg = FirstQuicMsg::new();
    let mut first_buffer = vec![0u8; 1024 * 100]; //100k缓冲区
    match recv_stream.read(&mut first_buffer).await {
        Ok(Some(length)) => {
            let origin_str = String::from_utf8_lossy(&first_buffer[0..length]);
            info!(
                "[server] received client init data, length: {}, content: {}",
                length, origin_str
            );
            match serde_json::from_str(&origin_str) {
                Ok(t) => {
                    _first_quic_msg = t;
                    info!(
                        "[server] Successfully parsed client init message: uuid={}, msg_type={:?}",
                        _first_quic_msg.uuid, _first_quic_msg.msg_type
                    );
                }
                Err(e) => {
                    error!("failed to serialize stream metadata: {}, raw data: {}", e, origin_str);
                    send_stream.finish().await?;
                    return Err(anyhow!("[server] Client init message format error"));
                }
            };
        }
        Ok(None) => {
            error!(
                "[server] Failed to receive client init message: client closed connection before sending init, client address: {}",
                address
            );
            send_stream.finish().await?;
            return Err(anyhow!("[server] Client closed connection without sending init message"));
        }
        Err(e) => {
            error!(
                "[server] failed to read init metadata: {}, client address: {}",
                e,
                address.as_str()
            );
            send_stream.finish().await?;
            return Err(anyhow!("[server] Error reading client init message"));
        }
    };
    Ok(_first_quic_msg)
}

/// Validate token effectiveness
async fn authenticate_connection(
    first_quic_msg: &FirstQuicMsg,
    send_stream: &mut SendStream,
) -> Result<Claims, anyhow::Error> {
    let claims =
        match verify_token(first_quic_msg.token.as_ref()).map_err(|_| "Failed to parse token") {
            Ok(t) => {
                if t.uuid != first_quic_msg.uuid {
                    error!("token does not match account!");
                    send_stream.finish().await?;
                    return Err(anyhow!("token does not match account!"));
                }
                t
            }
            Err(e) => {
                error!("failed to parse token: {}", e);
                send_stream.finish().await?;
                return Err(anyhow!("Failed to parse token!"));
            }
        };
    Ok(claims)
}

/// Check if maximum connections reached
async fn verify_max_client(
    send_stream: &mut SendStream,
    connections: &Arc<DashMap<String, QuicConnection>>,
    max_connections: usize,
) -> Result<(), anyhow::Error> {
    let server_book_len = connections.len();
    if server_book_len > max_connections {
        error!("max connections reached: {}", server_book_len);
        send_stream.finish().await?;
        return Err(anyhow!("Maximum connections reached: {}", server_book_len));
    }
    Ok(())
}

/// Record connection info
async fn set_conn_info(
    uuid: String,
    conn: Connection,
    connection_key: &str,
    address: String,
    now: i64,
    connections: &Arc<DashMap<String, QuicConnection>>,
    server_index: u32,
) -> Result<(), anyhow::Error> {
    let new_connection = QuicConnection {
        is_online: true,
        uuid,
        connection_type: ConnectionType::Text,
        conn,
        create_time: now as u64,
        update_time: now as u64,
        ipv4addr: address,
        ipv6addr: "".to_string(),
    };

    {
        connections.insert(connection_key.to_owned(), new_connection);
    }
    {
        let redis = REDIS_CLIENT.read().await;
        let redis = redis.as_ref().ok_or(anyhow!("Failed to acquire connection"))?;

        let mut conn = redis.get().await?;
        let index_str = server_index.to_string();
        conn.set_ex::<&str, &str, ()>(connection_key, &index_str, 7200).await?;
    }

    info!("current online clients: {}", connections.len());
    Ok(())
}

/// Handle connection
async fn handle_conn(
    mut send_stream: SendStream,
    mut recv_stream: RecvStream,
    conn: Connection,
    address: String,
    connections: Arc<DashMap<String, QuicConnection>>,
    config: ChatNodeConfig,
) -> Result<(), anyhow::Error> {
    info!("[server] processing new connection, client address: {}", address);

    let first_quic_msg =
        process_first_msg(&mut send_stream, &mut recv_stream, &address.clone()).await?;
    let head_length = first_quic_msg.dyn_header_size;
    let claims = authenticate_connection(&first_quic_msg, &mut send_stream).await?;
    let platform = claims.sub;
    let uuid = claims.uuid;
    verify_max_client(&mut send_stream, &connections, config.max_connections).await?;
    user_online(&uuid, &platform).await?;
    let current_uuid = uuid.clone();

    let _msg_type = first_quic_msg.msg_type.clone();

    let connection_key =
        format!("{}{}{}{}{}", platform, ":QUIC:SERVER:", uuid, ":", first_quic_msg.msg_type);
    let connection_key = connection_key.to_uppercase();
    info!("connection key: {}", connection_key);

    let now = get_now_time_stamp_as_millis().unwrap_or(0);
    set_conn_info(
        uuid,
        conn.clone(),
        &connection_key,
        address,
        now,
        &connections,
        config.server_index,
    )
    .await?;

    // 启动 uni stream 接收循环（客户端通过 open_uni 发送消息）
    let uni_shutdown = Arc::new(AtomicBool::new(false));
    let uni_shutdown_clone = uni_shutdown.clone();
    {
        let conn_for_uni = conn.clone();
        let conn_key = connection_key.clone();
        let platform_clone = platform.clone();
        let conns = connections.clone();
        let current_uid = current_uuid.clone();
        tokio::spawn(async move {
            let uni_buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            loop {
                if uni_shutdown_clone.load(Ordering::Relaxed) {
                    info!("[server] uni stream received close signal, exiting loop");
                    break;
                }
                match conn_for_uni.accept_uni().await {
                    Ok(mut recv) => {
                        let mut buf = vec![0u8; 1024 * 10];
                        match recv.read(&mut buf).await {
                            Ok(Some(length)) => {
                                let _ = process_rec_msg(
                                    &mut buf,
                                    current_uid.clone(),
                                    length,
                                    &conn_key,
                                    &platform_clone,
                                    uni_buffer_msg.clone(),
                                    head_length,
                                    conns.clone(),
                                    config.server_index,
                                )
                                .await;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                warn!("[server] uni stream read error: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[server] uni accept error: {}, continuing to wait", e);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });
    }

    // 维持原有 bidi 接收循环（处理初始化 + 保持兼容）
    let buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));

    loop {
        // Loop to process data in the stream
        let mut buffer = vec![0u8; 1024 * 10]; // Set buffer to 10KB
        let buffer_len = buffer_msg.lock().await.len();
        if buffer_len > config.max_buffer_length {
            error!("partial packet length exceeds limit: {}", buffer.len());
            // TODO: Send rate limit message to client for correction
            break;
        }
        let change_buffer = &mut buffer;
        match recv_stream.read(change_buffer).await {
            Ok(Some(length)) => {
                match process_rec_msg(
                    change_buffer,
                    current_uuid.clone(),
                    length,
                    &connection_key,
                    &platform,
                    buffer_msg.clone(),
                    head_length,
                    connections.clone(),
                    config.server_index,
                )
                .await
                {
                    Ok(_) => {
                        info!("message processing complete")
                    }
                    Err(error) => {
                        error!("failed to process message! {:#}", error.backtrace());
                    }
                }
            }
            Ok(None) => {
                info!("[server] stream closed");
                break;
            }
            Err(e) => {
                warn!("[server] read error: {}, exiting stream {}", e, recv_stream.id());
                break;
            }
        }
    }

    uni_shutdown.store(true, Ordering::Relaxed);

    end_server(&connection_key, &connection_key, now, &connections).await?;
    Ok(())
}

/// User offline
async fn end_server(
    close_key: &str,
    connection_key: &str,
    close_now: i64,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> Result<(), anyhow::Error> {
    let mut uuid = "".to_string();
    {
        if let Some(book) = connections.get_mut(close_key) {
            let now = book.update_time;
            if now == close_now as u64 {
                info!("user disconnected: {}", close_key);
                uuid = book.uuid.clone();
                drop(book);
                connections.remove(close_key);
                let redis = REDIS_CLIENT.read().await;
                let redis = match redis.as_ref() {
                    Some(r) => r,
                    None => {
                        error!("FATAL: failed to get redis connection");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        panic!("failed to get redis connection");
                    }
                };

                let mut conn = match redis.get().await {
                    Ok(c) => c,
                    Err(e) => {
                        error!("FATAL: failed to open redis connection: {}", e);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        panic!("failed to open redis connection: {}", e);
                    }
                };
                if let Err(e) = conn.del::<&str, ()>(connection_key).await {
                    error!("failed to delete connection info: {}", e);
                }
            }
        }
    }

    info!(
        "[server] Connection handled for {} complete, online connections: {}",
        close_key,
        connections.len()
    );

    user_offline(uuid).await?;

    Ok(())
}

/// User offline
async fn user_offline(uuid: String) -> std::result::Result<(), anyhow::Error> {
    // TODO
    let mut redis = get_redis_conn().await?;
    let rb = get_sql_client().await?;
    // 1. Set Redis distributed lock to prevent rapid offline/online transitions
    // 2. Sync all Redis cache to database, record user operations
    // Persist read messages from Redis to database
    let read_key = format!("{}{}", USER_READ_MSG, uuid);
    let read_record: String = redis.get(&read_key).await?;
    info!("read messages, source: {}", read_record);
    let last_chat_message_read: Vec<ChatMessageRecordRead> = serde_json::from_str(&read_record)?;
    info!("read messages, converted: {:?}", last_chat_message_read);
    // TODO: Validate read message effectiveness

    for item in last_chat_message_read.into_iter() {
        let is_exist =
            ChatMessageRecord::select_by_map(&rb, value! {"nano_id": &item.nano_id}).await?;
        if is_exist.is_empty() || is_exist.len() > 1 {
            continue;
        }
        let exit_item = match is_exist.first() {
            Some(item) => item,
            None => {
                error!("read message list exception: is_exist is empty");
                continue;
            }
        };
        if exit_item.recv_user.to_string() != item.recv_user.to_string()
            && exit_item.send_user.to_string() != item.recv_user.to_string()
        {
            err!("已读消息无效 {:?}", item);
            continue;
        }

        let insert_item = async |e| match ChatMessageRecordRead::insert(&rb, &item).await {
            Ok(_) => {}
            Err(x) => {
                err!("更新已读消息失败 {} {}", e, x);
            }
        };
        match ChatMessageRecordRead::update_by_map(
            &rb,
            &item,
            value! {"send_user": &item.send_user, "recv_user": &item.recv_user},
        )
        .await
        {
            Ok(d) => {
                if d.rows_affected < 1u64 {
                    insert_item(d.to_string()).await;
                }
            }
            Err(e) => {
                insert_item(e.to_string()).await;
            }
        };
    }

    // 3. Clean up Redis cache and lock
    Ok(())
}

/// User online
async fn user_online(uuid: &str, _platform: &str) -> std::result::Result<(), anyhow::Error> {
    info!("user online: {}", uuid);
    // TODO
    // 1. Set Redis distributed lock to prevent rapid online/offline transitions
    // 2. Sync all database to Redis cache
    // 3. Clean up Redis lock
    Ok(())
}
