use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Result, anyhow};
use common::config_str::{
    REDIS_INTERNAL_QUIC_SERVERS, REDIS_QUIC_SERVERS, REDIS_SPLIT, SYSTEM, USER_READ_MSG,
};
use common::models::chat_entity::add_read_chat_record::AddReadChatRecordDTO;
use common::models::chat_entity::chat_message_read::ChatMessageRecordRead;
use common::models::chat_entity::chat_message_record::ChatMessageRecord;
use common::models::group_entity::group_member::GroupMember;
use common::models::group_entity::group_message_record::GroupMessageRecord;
use common::state::CoreState;
use common::utils::internal_quic_client::send_internal_quic_msg;
use common::utils::internal_quic_msg::{InternalQuicRequest, RequestSource};
use common::utils::jwt_util::{Claims, verify_token};
use common::utils::mask::mask_addr;
use common::utils::message_types::MSG_TYPE_FORCE_LOGOUT;
use common::utils::text_msg::generate_text_msg;
use common::utils::time::get_now_time_stamp_as_millis;
use dashmap::DashMap;
use deadpool_redis::redis::{AsyncCommands, cmd};
use entity::models::chat_entity::chat_message_read::CHAT_TYPE_GROUP;
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use rbatis::dark_std::err;
use rbs::value;
use tokio::sync::{Mutex, watch};
use tokio::time::timeout;
use tracing::{error, info, warn};

use super::config::ChatNodeConfig;
use crate::models::first_quic_msg::FirstQuicMsg;
use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::msg_service::process_msg_service::process_rec_msg;

/// 启动并运行 QUIC 服务器，持续监听新连接
pub(crate) async fn run_server(
    endpoint: Arc<Endpoint>,
    connections: Arc<DashMap<String, QuicConnection>>,
    config: ChatNodeConfig,
    core: CoreState,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    info!("QUIC 服务器启动成功，地址: {}", config.bind_address);

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
                            error!("接受新连接失败：endpoint 已关闭");
                            return;
                        }
                    }
                }
            }
        };

        let conn = match incoming_conn.await {
            Ok(t) => t,
            Err(e) => {
                error!("建立连接失败 {}", e);
                continue;
            }
        };

        info!("[server] 已接受连接: address={}", mask_addr(&conn.remote_address().to_string()));
        let conns = connections.clone();
        let cfg = config.clone();
        let core_clone = core.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(conn, conns, cfg, core_clone).await {
                error!("打开双向流失败: {}", e);
            }
        });
    }
}

async fn handle_connection(
    quic_conn: Connection,
    connections: Arc<DashMap<String, QuicConnection>>,
    config: ChatNodeConfig,
    core: CoreState,
) -> Result<(), anyhow::Error> {
    info!("新连接来源: {}", mask_addr(&quic_conn.remote_address().to_string()));

    loop {
        match quic_conn.accept_bi().await {
            Ok((send_stream, recv_stream)) => {
                let address = quic_conn.remote_address().to_string().clone();
                let conns = connections.clone();
                let cfg = config.clone();
                let conn_handle = quic_conn.clone();
                let core_clone = core.clone();
                tokio::spawn(async move {
                    handle_conn(
                        send_stream,
                        recv_stream,
                        conn_handle,
                        address,
                        conns,
                        cfg,
                        core_clone,
                    )
                    .await
                    .unwrap_or_else(|x| error!("初始化连接失败 {}", x));
                });
            }
            Err(e) => {
                error!("连接错误: {:?}", e);
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
    address: &str,
) -> Result<FirstQuicMsg, anyhow::Error> {
    // 接收流元数据，确认消息类型和头部长度
    let mut _first_quic_msg = FirstQuicMsg::new();
    let mut first_buffer = vec![0u8; 1024 * 100]; //100k缓冲区
    match recv_stream.read(&mut first_buffer).await {
        Ok(Some(length)) => {
            let origin_str = String::from_utf8_lossy(&first_buffer[0..length]);
            info!("[server] 收到客户端初始化数据，长度: {}", length);
            match serde_json::from_str(&origin_str) {
                Ok(t) => {
                    _first_quic_msg = t;
                    info!(
                        "[server] 成功解析客户端初始化消息: uuid={}, msg_type={:?}",
                        _first_quic_msg.uuid, _first_quic_msg.msg_type
                    );
                }
                Err(e) => {
                    error!("序列化流元数据失败: {}", e);
                    send_stream.finish().await?;
                    return Err(anyhow!("[server] Client init message format error"));
                }
            };
        }
        Ok(None) => {
            error!(
                "[server] 接收客户端初始化消息失败: 客户端在发送初始化消息前已关闭连接，客户端地址: {}",
                mask_addr(address)
            );
            send_stream.finish().await?;
            return Err(anyhow!("[server] Client closed connection without sending init message"));
        }
        Err(e) => {
            error!("[server] 读取初始化元数据失败: {}, 客户端地址: {}", e, mask_addr(address));
            send_stream.finish().await?;
            return Err(anyhow!("[server] Error reading client init message"));
        }
    };
    Ok(_first_quic_msg)
}

/// 校验 token 有效性
async fn authenticate_connection(
    first_quic_msg: &FirstQuicMsg,
    send_stream: &mut SendStream,
) -> Result<Claims, anyhow::Error> {
    let claims =
        match verify_token(first_quic_msg.token.as_ref()).map_err(|_| "Failed to parse token") {
            Ok(t) => {
                if t.uuid != first_quic_msg.uuid {
                    error!("token 与账号不匹配!");
                    send_stream.finish().await?;
                    return Err(anyhow!("token does not match account!"));
                }
                t
            }
            Err(e) => {
                error!("解析 token 失败: {}", e);
                send_stream.finish().await?;
                return Err(anyhow!("Failed to parse token!"));
            }
        };
    Ok(claims)
}

/// 记录连接信息
#[allow(clippy::too_many_arguments)]
async fn set_conn_info(
    core: &CoreState,
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
        let mut conn = core.redis.get().await?;
        let index_str = server_index.to_string();
        conn.set_ex::<&str, &str, ()>(connection_key, &index_str, 7200).await?;
    }

    info!("当前在线客户端数: {}", connections.len());
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
    core: CoreState,
) -> Result<(), anyhow::Error> {
    info!("[server] 正在处理新连接，客户端地址: {}", mask_addr(&address));

    let first_quic_msg = process_first_msg(&mut send_stream, &mut recv_stream, &address).await?;
    let head_length = first_quic_msg.dyn_header_size;
    let claims = authenticate_connection(&first_quic_msg, &mut send_stream).await?;
    let platform = claims.sub;
    let uuid = claims.uuid;
    let online_lock_token = user_online(
        &core,
        &uuid,
        &platform,
        &connections,
        config.server_index,
        config.max_connections,
    )
    .await?;
    let current_uuid = uuid.clone();

    let _msg_type = first_quic_msg.msg_type.clone();

    let connection_key =
        format!("{}{}{}{}{}", platform, ":QUIC:SERVER:", uuid, ":", first_quic_msg.msg_type);
    let connection_key = connection_key.to_uppercase();
    info!("连接 key: {}", connection_key);

    let now = get_now_time_stamp_as_millis().unwrap_or(0);
    set_conn_info(
        &core,
        uuid,
        conn.clone(),
        &connection_key,
        address,
        now,
        &connections,
        config.server_index,
    )
    .await?;
    release_online_lock(&core, &platform, &current_uuid, &online_lock_token).await?;

    // 启动 uni stream 接收循环（客户端通过 open_uni 发送消息）
    let uni_shutdown = Arc::new(AtomicBool::new(false));
    let uni_shutdown_clone = uni_shutdown.clone();
    {
        let conn_for_uni = conn.clone();
        let conn_key = connection_key.clone();
        let platform_clone = platform.clone();
        let conns = connections.clone();
        let current_uid = current_uuid.clone();
        let core_clone = core.clone();
        tokio::spawn(async move {
            let uni_buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            loop {
                if uni_shutdown_clone.load(Ordering::Relaxed) {
                    info!("[server] uni 流收到关闭信号，退出循环");
                    break;
                }
                match conn_for_uni.accept_uni().await {
                    Ok(mut recv) => {
                        // 大消息会跨多个QUIC包到达，必须读取完整流后统一处理，
                        // 否则未读余量会触发对端 STOP_SENDING，导致"发送被对端终止"
                        let mut msg_data: Vec<u8> = Vec::new();
                        let mut chunk = vec![0u8; 1024 * 10];
                        loop {
                            match recv.read(&mut chunk).await {
                                Ok(Some(n)) => {
                                    msg_data.extend_from_slice(&chunk[..n]);
                                }
                                Ok(None) => break,
                                Err(e) => {
                                    warn!("[server] uni 流读取错误: {}", e);
                                    break;
                                }
                            }
                        }
                        if !msg_data.is_empty() {
                            let msg_len = msg_data.len();
                            let _ = process_rec_msg(
                                &core_clone,
                                &mut msg_data,
                                current_uid.clone(),
                                msg_len,
                                &conn_key,
                                &platform_clone,
                                uni_buffer_msg.clone(),
                                head_length,
                                conns.clone(),
                                config.server_index,
                            )
                            .await;
                        }
                    }
                    Err(e) => {
                        warn!("[server] uni 流接受错误: {}, 继续等待", e);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });
    }

    // 定期续期用户路由 key 的 TTL,避免长连接存活但 Redis 路由 key(7200s)过期失效
    {
        let refresh_core = core.clone();
        let refresh_key = connection_key.clone();
        let refresh_index = config.server_index.to_string();
        let shutdown_flag = uni_shutdown.clone();
        tokio::spawn(async move {
            let mut refresh_interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                refresh_interval.tick().await;
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }
                let mut conn = match refresh_core.redis.get().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        warn!("用户路由 key 续期失败(获取连接): key={} err={}", refresh_key, e);
                        continue;
                    }
                };
                if let Err(e) =
                    conn.set_ex::<&str, &str, ()>(&refresh_key, &refresh_index, 7200).await
                {
                    warn!("用户路由 key 续期失败: key={} err={}", refresh_key, e);
                }
            }
        });
    }

    // 维持原有 bidi 接收循环（处理初始化 + 保持兼容）
    let buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));

    loop {
        // 循环处理流中的数据
        let mut buffer = vec![0u8; 1024 * 10]; // 将缓冲区设置为 10KB
        let buffer_len = buffer_msg.lock().await.len();
        if buffer_len > config.max_buffer_length {
            error!("分包长度超过限制: {}", buffer.len());
            // TODO: 发送限速消息给客户端进行纠正
            break;
        }
        let change_buffer = &mut buffer;
        match recv_stream.read(change_buffer).await {
            Ok(Some(length)) => {
                match process_rec_msg(
                    &core,
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
                        info!("消息处理完成")
                    }
                    Err(error) => {
                        error!("消息处理失败! {:#}", error.backtrace());
                    }
                }
            }
            Ok(None) => {
                info!("[server] 流已关闭");
                break;
            }
            Err(e) => {
                warn!("[server] 读取错误: {}, 退出流 {}", e, recv_stream.id());
                break;
            }
        }
    }

    uni_shutdown.store(true, Ordering::Relaxed);

    end_server(&core, &connection_key, &connection_key, now, conn.stable_id(), &connections)
        .await?;
    Ok(())
}

/// 用户离线
async fn end_server(
    core: &CoreState,
    close_key: &str,
    connection_key: &str,
    close_now: i64,
    connection_id: usize,
    connections: &Arc<DashMap<String, QuicConnection>>,
) -> Result<(), anyhow::Error> {
    let mut uuid = "".to_string();
    {
        if let Some(book) = connections.get_mut(close_key) {
            let now = book.update_time;
            if now == close_now as u64 && book.conn.stable_id() == connection_id {
                info!("用户已断开连接: {}", close_key);
                uuid = book.uuid.clone();
                drop(book);
                connections.remove(close_key);
                let mut conn = core.redis.get().await?;
                if let Err(e) = conn.del::<&str, ()>(connection_key).await {
                    error!("删除连接信息失败: {}", e);
                }
            }
        }
    }

    info!("[server] 连接 {} 处理完成，当前在线连接数: {}", close_key, connections.len());

    if !uuid.is_empty() {
        user_offline(core, uuid).await?;
    }

    Ok(())
}

/// 用户离线
async fn user_offline(core: &CoreState, uuid: String) -> std::result::Result<(), anyhow::Error> {
    sync_read_messages(core, &uuid).await
}

/// 将 Redis 中缓存的已读消息同步到数据库。
async fn sync_read_messages(
    core: &CoreState,
    uuid: &str,
) -> std::result::Result<(), anyhow::Error> {
    // TODO
    let mut redis = core.redis.get().await?;
    let rb = &core.db;
    // 1. 设置 Redis 分布式锁，防止用户频繁上下线切换
    // 2. 将 Redis 缓存同步到数据库，记录用户操作
    // 将已读消息从 Redis 持久化到数据库
    let read_key = format!("{}{}", USER_READ_MSG, uuid).to_uppercase();
    let read_record: Option<String> = redis.get(&read_key).await?;
    let Some(read_record) = read_record else {
        info!("用户没有待同步的已读消息: {}", uuid);
        return Ok(());
    };
    drop(redis);
    info!("读取已读消息，来源: {}", read_record);
    let last_chat_message_read: Vec<AddReadChatRecordDTO> = serde_json::from_str(&read_record)?;
    info!("读取已读消息，转换后: {:?}", last_chat_message_read);
    // TODO: 校验已读消息的有效性

    for item in last_chat_message_read.into_iter() {
        // 群聊已读消息：校验群消息与群成员，更新群成员已读游标
        if item.chat_type == Some(CHAT_TYPE_GROUP) {
            let group_msg = match GroupMessageRecord::select_by_nano_id(
                rb,
                item.nano_id.as_deref().unwrap_or(""),
            )
            .await
            {
                Ok(Some(msg)) => msg,
                _ => {
                    err!("群已读消息无效 {:?}", item);
                    continue;
                }
            };
            let group_uuid = match group_msg.group_uuid {
                Some(u) => u,
                None => {
                    err!("群已读消息缺少群UUID {:?}", item);
                    continue;
                }
            };
            // 读者必须是群成员，且已读游标只推进不回退
            let mut member =
                match GroupMember::select_by_group_and_user(rb, &group_uuid, &item.recv_user)
                    .await?
                {
                    Some(m) => m,
                    None => {
                        err!("群已读消息无效，用户不在群中 {:?}", item);
                        continue;
                    }
                };
            let msg_id = group_msg.id.unwrap_or(0);
            if member.last_read_msg_id.unwrap_or(0) < msg_id {
                member.last_read_msg_id = Some(msg_id);
                GroupMember::update_by_group_and_user(rb, &member, &group_uuid, &item.recv_user)
                    .await?;
                info!("群已读消息更新成功 {:?}", item);
            }
            continue;
        }

        // 单聊已读消息：校验后写入 chat_message_record_read 表
        let record = ChatMessageRecordRead {
            id: None,
            nano_id: item.nano_id.clone(),
            timestamp: item.timestamp,
            send_user: item.send_user,
            recv_user: item.recv_user,
        };

        let is_exist =
            ChatMessageRecord::select_by_map(rb, value! {"nano_id": &record.nano_id}).await?;
        if is_exist.is_empty() || is_exist.len() > 1 {
            continue;
        }
        let exit_item = match is_exist.first() {
            Some(item) => item,
            None => {
                error!("已读消息列表异常: is_exist 为空");
                continue;
            }
        };
        if exit_item.recv_user.to_string() != record.recv_user.to_string()
            && exit_item.send_user.to_string() != record.recv_user.to_string()
        {
            err!("已读消息无效 {:?}", record);
            continue;
        }

        let insert_item = async |e| match ChatMessageRecordRead::insert(rb, &record).await {
            Ok(_) => {}
            Err(x) => {
                err!("更新已读消息失败 {} {}", e, x);
            }
        };
        match ChatMessageRecordRead::update_by_map(
            rb,
            &record,
            value! {"send_user": &record.send_user, "recv_user": &record.recv_user},
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

    // 3. 清理 Redis 缓存和锁
    Ok(())
}

/// 用户上线
async fn user_online(
    core: &CoreState,
    uuid: &str,
    platform: &str,
    connections: &Arc<DashMap<String, QuicConnection>>,
    server_index: u32,
    max_connections: usize,
) -> std::result::Result<String, anyhow::Error> {
    info!("用户上线: {}", uuid);

    let lock_key = format!("QUIC:ONLINE:LOCK:{}:{}", platform, uuid).to_uppercase();
    let lock_token = format!("{}:{}", server_index, get_now_time_stamp_as_millis().unwrap_or(0));
    let mut redis = core.redis.get().await?;
    let acquired: Option<String> = cmd("SET")
        .arg(&lock_key)
        .arg(&lock_token)
        .arg("NX")
        .arg("EX")
        .arg(30)
        .query_async(&mut redis)
        .await?;
    if acquired.is_none() {
        return Err(anyhow!("用户正在处理上线: {}", uuid));
    }
    drop(redis);

    // 在上线锁保护期间先持久化上一次连接产生的已读状态。
    timeout(std::time::Duration::from_secs(25), sync_read_messages(core, uuid))
        .await
        .map_err(|_| anyhow!("同步用户已读消息超时: {}", uuid))??;

    let connection_key = format!(
        "{}:{}{}{}{}",
        platform,
        REDIS_QUIC_SERVERS,
        uuid,
        REDIS_SPLIT,
        ConnectionType::Text
    )
    .to_uppercase();
    let payload = generate_text_msg(
        MSG_TYPE_FORCE_LOGOUT,
        "您的账号已在其他设备登录".as_bytes().to_vec(),
        uuid.to_string(),
        SYSTEM.to_string(),
    )?;

    // Redis 中保存旧连接所在节点。没有旧记录时仍检查本机，避免 Redis 短暂丢失造成重复连接。
    let old_index = {
        let mut redis = core.redis.get().await?;
        let index: Option<String> = redis.get(&connection_key).await?;
        index.and_then(|value| value.parse::<u32>().ok())
    };

    let has_old_connection = connections.contains_key(&connection_key) || old_index.is_some();
    if connections.len() >= max_connections && !has_old_connection {
        return Err(anyhow!("Maximum connections reached: {}", connections.len()));
    }

    if let Some(old) = connections.get(&connection_key) {
        let old_conn = old.conn.clone();
        drop(old);
        kick_local_connection(core, connections, &connection_key, old_conn, payload.clone())
            .await?;
    } else if let Some(old_index) = old_index.filter(|index| *index != server_index) {
        let mut redis = core.redis.get().await?;
        let node_key = format!("{}{}", REDIS_INTERNAL_QUIC_SERVERS, old_index);
        let node_addr: Option<String> = redis.get(&node_key).await?;
        drop(redis);

        let node_addr =
            node_addr.ok_or_else(|| anyhow!("旧连接所在节点不可用: server_index={}", old_index))?;
        let response = send_internal_quic_msg(
            node_addr.parse()?,
            InternalQuicRequest {
                msg_type: MSG_TYPE_FORCE_LOGOUT,
                payload,
                target_user: uuid.to_string(),
                preferred_index: old_index,
                platform: platform.to_string(),
                source: RequestSource::QuicExternal,
                ttl: 3,
                close_after_delivery: true,
            },
        )
        .await?;
        if response.delivered != Some(true) {
            return Err(anyhow!(
                "远程旧连接踢下线失败: {}",
                response.message.unwrap_or_else(|| "unknown error".to_string())
            ));
        }
    }

    Ok(lock_token)
}

async fn release_online_lock(
    core: &CoreState,
    platform: &str,
    uuid: &str,
    lock_token: &str,
) -> Result<()> {
    let lock_key = format!("QUIC:ONLINE:LOCK:{}:{}", platform, uuid).to_uppercase();
    let mut redis = core.redis.get().await?;
    let _: i32 = cmd("EVAL")
        .arg("if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('del', KEYS[1]) else return 0 end")
        .arg(1)
        .arg(&lock_key)
        .arg(lock_token)
        .query_async(&mut redis)
        .await?;
    Ok(())
}

async fn kick_local_connection(
    core: &CoreState,
    connections: &Arc<DashMap<String, QuicConnection>>,
    connection_key: &str,
    old_conn: Connection,
    payload: Vec<u8>,
) -> Result<()> {
    if let Ok(mut send) = old_conn.open_uni().await {
        if let Err(error) = send.write_all(&payload).await {
            warn!("发送强制退出消息失败: {}", error);
        } else if let Err(error) = send.finish().await {
            warn!("完成强制退出消息失败: {}", error);
        }
    } else {
        warn!("旧连接已无法打开单向流，直接关闭连接");
    }
    old_conn.close(0u32.into(), b"replaced by another login");

    if connections
        .get(connection_key)
        .map(|entry| entry.conn.stable_id() == old_conn.stable_id())
        .unwrap_or(false)
    {
        connections.remove(connection_key);
        let mut redis = core.redis.get().await?;
        let _: () = redis.del(connection_key).await?;
    }

    Ok(())
}
