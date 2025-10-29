use crate::common::service::user_service::{user_offline, user_online};
use crate::quic_service::make_server_endpoint;
use crate::quic_service::models::first_quic_msg::FirstQuicMsg;
use crate::quic_service::models::quic_connection::{ConnectionType, QuicConnection};
use crate::quic_service::msg_service::process_msg_service::process_rec_msg;
use crate::utils::global_static_str::{
    MAX_QUIC_BUFFER_LEN, MAX_QUIC_SERVERS, SERVER_NAME, USER_READ_MSG,
};
use crate::utils::jwt_util::decode_jwt;
use crate::utils::time::get_now_time_stamp_as_millis;
use crate::{GLOBAL_QUIC_SERVER_LIST, REDIS_CLIENT};
use anyhow::{anyhow, Result};
use deadpool_redis::redis::AsyncCommands;
use log::{error, info};
use quinn::{Connection, RecvStream, SendStream};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub(crate) fn init_server(addr: SocketAddr) {
    tokio::spawn(run_server(addr));
}

/// 启动并运行QUIC服务器，持续监听新连接
async fn run_server(addr: SocketAddr) {
    // 创建服务器端点和证书
    let (endpoint, _server_cert) = make_server_endpoint(addr).unwrap();
    info!("quic服务器启动成功,使用地址为: {}", addr);

    // 持续监听新连接请求
    loop {
        let incoming_conn = endpoint.accept().await.unwrap(); // 接收新连接请求
        let conn = match incoming_conn.await {
            Ok(t) => t,
            Err(e) => {
                error!("建立链接失败 {}", e.to_string());
                continue;
            }
        }; // 确认连接建立

        info!(
            "[服务端] 连接已接受: 地址={}",
            conn.remote_address() // 打印客户端地址
        );
        tokio::spawn(async move {
            // 异步处理每个连接
            handle_connection(conn).await.expect("打开双向流失败");
        });
    }
}

// 单个连接的多流处理函数
async fn handle_connection(conn: Connection) -> Result<(), anyhow::Error> {
    info!("新连接来源: {:?}", conn.remote_address());

    // 4. 循环接受该连接的双向流
    loop {
        match conn.accept_bi().await {
            Ok((send_stream, recv_stream)) => {
                // 5. 为每个流生成独立异步任务
                let address = conn.remote_address().to_string().clone();
                tokio::spawn(async move {
                    handle_conn(send_stream, recv_stream, address)
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
    let mut first_quic_msg = FirstQuicMsg::new();
    let mut first_buffer = vec![0u8; 1024 * 10]; //10k缓冲区
    match recv_stream.read(&mut first_buffer).await {
        Ok(Some(length)) => {
            let origin_str = String::from_utf8_lossy(&first_buffer[0..length]);
            info!("[服务端] 长度为 {} ", length);
            match serde_json::from_str(&*origin_str) {
                Ok(t) => {
                    first_quic_msg = t;
                }
                _ => {
                    error!("序列化流数据的元数据失败 {}", origin_str);
                    send_stream.finish().await?;
                }
            };
        }
        Ok(None) => {
            error!("[服务端] 发送流初始化元数据失败: {}", address);
            return Err(anyhow!("[服务端] 发送流初始化元数据为空"));
        }
        Err(e) => {
            error!(
                "[服务端] 初始化读取元数据错误: {},退出流{}",
                e,
                address.as_str()
            );
            return Err(anyhow!("[服务端] 发送流初始化元数据为空"));
        }
    };
    Ok(first_quic_msg)
}

/// 校验token有效性
async fn verify_token(
    first_quic_msg: &FirstQuicMsg,
    send_stream: &mut SendStream,
) -> Result<String, anyhow::Error> {
    let uuid = match decode_jwt(first_quic_msg.token.as_ref()).map_err(|_| "解析token失败") {
        Ok(t) => {
            if t != first_quic_msg.uuid {
                error!("令牌跟账号不匹配！");
                send_stream.finish().await?;
                return Err(anyhow!("令牌跟账号不匹配！"));
            }
            t
        }
        Err(e) => {
            error!("解析令牌失败 {}", e.to_string());
            send_stream.finish().await?;
            return Err(anyhow!("解析令牌失败！"));
        }
    };
    Ok(uuid)
}

/// 检测是否达到最大连接数
async fn verify_max_client(send_stream: &mut SendStream) -> Result<(), anyhow::Error> {
    let server_book = GLOBAL_QUIC_SERVER_LIST.read().await;
    let server_book_len = server_book.len();
    if server_book_len > MAX_QUIC_SERVERS {
        error!("达到最大连接数 {}", server_book_len);
        send_stream.finish().await?;
    }
    Ok(())
}

/// 记录连接信息
async fn set_conn_info(
    uuid: String,
    send_stream: Arc<RwLock<SendStream>>,
    connection_key: &String,
    address: String,
    now: i64,
) -> Result<(), anyhow::Error> {
    let new_connection = QuicConnection {
        is_online: true,
        uuid,
        connection_type: ConnectionType::Text,
        send_stream: send_stream.clone(),
        create_time: now as u64,
        update_time: now as u64,
        ipv4addr: address,
        ipv6addr: "".to_string(),
    };

    {
        let mut server_book = GLOBAL_QUIC_SERVER_LIST.write().await;
        server_book.insert(connection_key.clone(), new_connection);
    }
    {
        let redis = REDIS_CLIENT.read().await;
        let redis = redis.as_ref().ok_or(anyhow!("获取连接失败"))?;

        let mut conn = redis.get().await?;
        conn.set_ex::<&str, &str, ()>(&connection_key, SERVER_NAME, 7200)
            .await?;
    }

    info!(
        "当前的在线客户端 {}",
        GLOBAL_QUIC_SERVER_LIST.read().await.len()
    );
    Ok(())
}

/// 处理连接
async fn handle_conn(
    mut send_stream: SendStream,
    mut recv_stream: RecvStream,
    address: String,
) -> Result<(), anyhow::Error> {
    let first_quic_msg =
        process_first_msg(&mut send_stream, &mut recv_stream, &address.clone()).await?;
    let head_length = first_quic_msg.dyn_header_size;
    let uuid = verify_token(&first_quic_msg, &mut send_stream).await?;
    verify_max_client(&mut send_stream).await?;
    user_online(uuid.clone()).await?;
    let current_uuid = uuid.clone();

    let msg_type = first_quic_msg.msg_type.clone();

    let connection_key = format!(
        "{}{}{}{}",
        "QUIC:SERVER:",
        uuid,
        ":",
        first_quic_msg.msg_type.to_string()
    );
    let connection_key = connection_key.to_uppercase();
    info!("connection key: {}", connection_key);
    let close_key = connection_key.clone();

    //通过原子计数和异步锁共享变量
    let send_stream = Arc::new(RwLock::new(send_stream));
    let now = get_now_time_stamp_as_millis().unwrap_or_else(|_| 0);
    set_conn_info(uuid, send_stream, &connection_key, address, now).await?;

    let buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));

    loop {
        // 循环处理流中的数据
        let mut buffer = vec![0u8; 1024 * 10]; //设置缓冲区为10KB
        let buffer_len = buffer_msg.lock().await.len();
        if buffer_len > MAX_QUIC_BUFFER_LEN {
            error!("半包长度超过限制 {}", buffer.len());
            // TODO发送限制消息给客户端纠错
            break;
        }
        let change_buffer = &mut buffer;
        match recv_stream.read(change_buffer).await {
            Ok(Some(length)) => {
                let new_close_key = close_key.clone();

                match process_rec_msg(
                    change_buffer,
                    current_uuid.clone(),
                    length,
                    new_close_key,
                    &msg_type,
                    buffer_msg.clone(),
                    head_length,
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
                error!("[服务端] 读取错误: {},退出流{}", e, recv_stream.id());
                break;
            }
        }
    }

    end_server(&close_key, &connection_key, now).await?;
    Ok(())
}

/// 用户下线
async fn end_server(
    close_key: &String,
    connection_key: &String,
    close_now: i64,
) -> Result<(), anyhow::Error> {
    let mut uuid = "".to_string();
    {
        let mut server_book = GLOBAL_QUIC_SERVER_LIST.write().await;
        if let Some(book) = server_book.get_mut(close_key) {
            let now = book.update_time;
            if now == close_now as u64 {
                info!("用户下线 {}", close_key);
                uuid = book.uuid.clone();
                server_book.remove(close_key);
                let redis = REDIS_CLIENT.read().await;
                let redis = redis.as_ref().expect("获取redis连接失败");

                let mut conn = redis.get().await.expect("打开redis连接失败");
                conn.del::<&str, ()>(&connection_key)
                    .await
                    .expect("删除连接信息失败");
            }
        }
    }

    info!(
        "[服务器] 处理完成连接 {} 完成, 在线连接数为 {}",
        close_key,
        GLOBAL_QUIC_SERVER_LIST.read().await.len()
    );

    user_offline(uuid).await?;

    Ok(())
}
