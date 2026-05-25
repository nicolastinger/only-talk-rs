use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Result, anyhow};
use dashmap::DashMap;
use deadpool_redis::redis::AsyncCommands;
use common::config_str::{USER_READ_MSG};
use common::models::chat_entity::chat_message_read::ChatMessageRecordRead;
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::utils::jwt_util::{decode_jwt, Claims};
use common::utils::redis_utils::get_redis_conn;
use common::utils::time::get_now_time_stamp_as_millis;
use common::{REDIS_CLIENT};
use tracing::{error, info, warn};
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use rbatis::dark_std::err;
use rbs::value;
use tokio::sync::{Mutex, watch};
use common::utils::sql_utils::get_sql_client;
use super::config::ChatNodeConfig;
use crate::models::first_quic_msg::FirstQuicMsg;
use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::msg_service::process_msg_service::process_rec_msg;

/// 启动并运行QUIC服务器，持续监听新连接
pub(crate) async fn run_server(
    endpoint: Arc<Endpoint>,
    connections: Arc<DashMap<String, QuicConnection>>,
    config: ChatNodeConfig,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    info!(
        "quic服务器启动成功,使用地址为: {}",
        config.bind_address
    );

    loop {
        let incoming_conn = {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    info!("收到关闭信号，停止接受新连接");
                    return;
                }
                result = endpoint.accept() => {
                    match result {
                        Some(conn) => conn,
                        None => {
                            error!("接收新连接请求失败: endpoint 已关闭");
                            return;
                        }
                    }
                }
            }
        };

        let conn = match incoming_conn.await {
            Ok(t) => t,
            Err(e) => {
                error!("建立链接失败 {}", e);
                continue;
            }
        };

        info!(
            "[服务端] 连接已接受: 地址={}",
            conn.remote_address()
        );
        let conns = connections.clone();
        let cfg = config.clone();
        tokio::spawn(async move {
            handle_connection(conn, conns, cfg).await.expect("打开双向流失败");
        });
    }
}

async fn handle_connection(
    quic_conn: Connection,
    connections: Arc<DashMap<String, QuicConnection>>,
    config: ChatNodeConfig,
) -> Result<(), anyhow::Error> {
    info!("新连接来源: {:?}", quic_conn.remote_address());

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
                        .unwrap_or_else(|x| error!("初始化连接失败 {}", x));
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

/// 处理元数据
async fn process_first_msg(
    send_stream: &mut SendStream,
    recv_stream: &mut RecvStream,
    address: &String,
) -> Result<FirstQuicMsg, anyhow::Error> {
    //接收流元数据，确认消息类型以及头部长度
    let mut _first_quic_msg = FirstQuicMsg::new();
    let mut first_buffer = vec![0u8; 1024 * 100]; //100k缓冲区
    match recv_stream.read(&mut first_buffer).await {
        Ok(Some(length)) => {
            let origin_str = String::from_utf8_lossy(&first_buffer[0..length]);
            info!("[服务端] 接收到客户端初始化数据，长度为 {}，内容: {}", length, origin_str);
            match serde_json::from_str(&origin_str) {
                Ok(t) => {
                    _first_quic_msg = t;
                    info!(
                        "[服务端] 成功解析客户端初始化消息: uuid={}, msg_type={:?}",
                        _first_quic_msg.uuid, _first_quic_msg.msg_type
                    );
                }
                Err(e) => {
                    error!("序列化流数据的元数据失败: {}，原始数据: {}", e, origin_str);
                    send_stream.finish().await?;
                    return Err(anyhow!("[服务端] 客户端初始化消息格式错误"));
                }
            };
        }
        Ok(None) => {
            error!(
                "[服务端] 接收客户端初始化消息失败: 客户端在发送初始化消息前关闭了连接，客户端地址: {}",
                address
            );
            send_stream.finish().await?;
            return Err(anyhow!("[服务端] 客户端未发送初始化消息就关闭了连接"));
        }
        Err(e) => {
            error!("[服务端] 初始化读取元数据错误: {}, 客户端地址: {}", e, address.as_str());
            send_stream.finish().await?;
            return Err(anyhow!("[服务端] 读取客户端初始化消息时发生错误"));
        }
    };
    Ok(_first_quic_msg)
}

/// 校验token有效性
async fn verify_token(
    first_quic_msg: &FirstQuicMsg,
    send_stream: &mut SendStream,
) -> Result<Claims, anyhow::Error> {
    let claims = match decode_jwt(first_quic_msg.token.as_ref()).map_err(|_| "解析token失败") {
        Ok(t) => {
            if t.uuid != first_quic_msg.uuid {
                error!("令牌跟账号不匹配！");
                send_stream.finish().await?;
                return Err(anyhow!("令牌跟账号不匹配！"));
            }
            t
        }
        Err(e) => {
            error!("解析令牌失败 {}", e);
            send_stream.finish().await?;
            return Err(anyhow!("解析令牌失败！"));
        }
    };
    Ok(claims)
}

/// 检测是否达到最大连接数
async fn verify_max_client(
    send_stream: &mut SendStream,
    connections: &Arc<DashMap<String, QuicConnection>>,
    max_connections: usize,
) -> Result<(), anyhow::Error> {
    let server_book_len = connections.len();
    if server_book_len > max_connections {
        error!("达到最大连接数 {}", server_book_len);
        send_stream.finish().await?;
    }
    Ok(())
}

/// 记录连接信息
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
        let redis = redis.as_ref().ok_or(anyhow!("获取连接失败"))?;

        let mut conn = redis.get().await?;
        let index_str = server_index.to_string();
        conn.set_ex::<&str, &str, ()>(connection_key, &index_str, 7200).await?;
    }

    info!("当前的在线客户端 {}", connections.len());
    Ok(())
}

/// 处理连接
async fn handle_conn(
    mut send_stream: SendStream,
    mut recv_stream: RecvStream,
    conn: Connection,
    address: String,
    connections: Arc<DashMap<String, QuicConnection>>,
    config: ChatNodeConfig,
) -> Result<(), anyhow::Error> {
    info!("[服务端] 开始处理新连接，客户端地址: {}", address);

    let first_quic_msg =
        process_first_msg(&mut send_stream, &mut recv_stream, &address.clone()).await?;
    let head_length = first_quic_msg.dyn_header_size;
    let claims = verify_token(&first_quic_msg, &mut send_stream).await?;
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
    set_conn_info(uuid, conn.clone(), &connection_key, address, now, &connections, config.server_index).await?;

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
                    info!("[服务端] uni流收到关闭信号，退出循环");
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
                                )
                                .await;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                warn!("[服务端] uni流读取错误: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[服务端] uni accept 错误: {}，继续等待", e);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });
    }

    // 维持原有 bidi 接收循环（处理初始化 + 保持兼容）
    let buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));

    loop {
        // 循环处理流中的数据
        let mut buffer = vec![0u8; 1024 * 10]; //设置缓冲区为10KB
        let buffer_len = buffer_msg.lock().await.len();
        if buffer_len > config.max_buffer_length {
            error!("半包长度超过限制 {}", buffer.len());
            // TODO发送限制消息给客户端纠错
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
                )
                .await
                {
                    Ok(_) => {
                        info!("处理消息完成")
                    }
                    Err(error) => {
                        error!("处理信息失败! {:#}", error.backtrace());
                    }
                }
            }
            Ok(None) => {
                info!("[服务端] 流关闭");
                break;
            }
            Err(e) => {
                warn!("[服务端] 读取错误: {},退出流{}", e, recv_stream.id());
                break;
            }
        }
    }

    uni_shutdown.store(true, Ordering::Relaxed);

    end_server(&connection_key, &connection_key, now, &connections).await?;
    Ok(())
}

/// 用户下线
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
                info!("用户下线 {}", close_key);
                uuid = book.uuid.clone();
                drop(book);
                connections.remove(close_key);
                let redis = REDIS_CLIENT.read().await;
                let redis = redis.as_ref().expect("获取redis连接失败");

                let mut conn = redis.get().await.expect("打开redis连接失败");
                conn.del::<&str, ()>(connection_key).await.expect("删除连接信息失败");
            }
        }
    }

    info!(
        "[服务器] 处理完成连接 {} 完成, 在线连接数为 {}",
        close_key,
        connections.len()
    );

    user_offline(uuid).await?;

    Ok(())
}

/// 用户下线
async fn user_offline(uuid: String) -> std::result::Result<(), anyhow::Error> {
    // TODO
    let mut redis = get_redis_conn().await?;
    let rb = get_sql_client().await?;
    // 1.设置redis分布式锁，防止用户下线的同时立马上线
    // 2.同步所有redis缓存到数据库，记录用户操作
    // 已读消息从redis中持久化到数据库
    let read_key = format!("{}{}", USER_READ_MSG, uuid);
    let read_record: String = redis.get(&read_key).await?;
    info!("已读消息, 源 {}", read_record);
    let last_chat_message_read: Vec<ChatMessageRecordRead> = serde_json::from_str(&read_record)?;
    info!("已读消息, 转换 {:?}", last_chat_message_read);
    // TODO已读消息有效校验

    for item in last_chat_message_read.into_iter() {
        let is_exist =
            ChatMessageRecord::select_by_map(&rb, value! {"nano_id": &item.nano_id}).await?;
        if is_exist.is_empty() || is_exist.len() > 1 {
            continue;
        }
        let exit_item = is_exist.first().expect("获取已读消息失败");
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

    // 3.清理redis缓存，清理redis锁
    Ok(())
}

/// 用户上线
async fn user_online(uuid: &str, _platform: &str) -> std::result::Result<(), anyhow::Error> {
    info!("用户上线 {}", uuid);
    // TODO
    // 1.设置redis分布式锁，防止用户上线的同时立马下线
    // 2.同步所有数据库到redis缓存
    // 3.清理redis锁
    Ok(())
}
