use crate::common::quic_network_service::make_server_endpoint;
use crate::common::quic_network_service::models::quic_connection::{
    ConnectionType, QuicConnection
};
use crate::utils::jwt_util::decode_jwt;
use crate::{GLOBAL_QUIC_SERVER_LIST};
use anyhow::{ Context, Result};
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Pool;
use log::{error, info};
use quinn::{Connection, RecvStream, SendStream};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use crate::common::quic_network_service::models::first_quic_msg::FirstQuicMsg;
use crate::common::quic_network_service::msg_service::process_msg_service::process_rec_msg;
use crate::utils::time::get_now_time_stamp_as_millis;

pub(crate) fn init_server(redis: Pool, addr: SocketAddr) {
    tokio::spawn(run_server(addr, redis));
}

/// 启动并运行QUIC服务器，持续监听新连接
async fn run_server(addr: SocketAddr, redis: Pool) {
    // 创建服务器端点和证书
    let (endpoint, _server_cert) = make_server_endpoint(addr).unwrap();
    info!("quic服务器启动成功,使用地址为: {}", addr);

    // 持续监听新的连接请求
    loop {
        let incoming_conn = endpoint.accept().await.unwrap(); // 接收新的连接请求
        let conn = match incoming_conn.await{
            Ok(t) => t,
            Err(e) => {
                error!("建立链接失败 {}", e.to_string());
                continue;
            }
        }; // 确认连接建立

        let new_pool = redis.clone();
        info!(
            "[服务端] 连接已接受: 地址={}",
            conn.remote_address() // 打印客户端地址
        );
        tokio::spawn(async move {
            // 异步处理每个连接
            handle_connection(conn, new_pool).await.expect("打开双向流失败");
        });
    }
}

// 单个连接的多流处理函数
async fn handle_connection(mut conn: Connection, redis: Pool) -> Result<(), anyhow::Error> {
        println!("New connection from: {:?}", conn.remote_address());

        // 4. 循环接受该连接的双向流
        loop {
            match conn.accept_bi().await {
                Ok((send_stream, recv_stream)) => {
                    // 5. 为每个流生成独立异步任务
                    let redis = redis.clone();
                    let address = conn.remote_address().to_string().clone();
                    tokio::spawn(async move {
                        handle_conn(send_stream, recv_stream, redis, address).await;
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

async fn handle_conn(mut send_stream: SendStream, mut recv_stream: RecvStream, redis: Pool, address: String) {
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
                    send_stream.finish().await.expect("发送终止信号失败");
                    return;
                }
            };
        }
        Ok(None) => {
            error!("[服务端] 发送流初始化元数据失败: {}", address);
            return;
        }
        Err(e) => {
            error!(
                "[服务端] 初始化读取元数据错误: {},退出流{}",
                e,
                address.as_str()
            );
        }
    }

    let head_length = first_quic_msg.dyn_header_size;
    match decode_jwt(first_quic_msg.token.as_ref()).map_err(|_| "解析token失败") {
        Ok(t) => {
            if t != first_quic_msg.account {
                error!("令牌跟账号不匹配！");
                send_stream.finish().await.expect("发送终止信号失败");
                return;
            }
        }
        Err(_) => {
            error!("解析令牌失败");
            send_stream.finish().await.expect("发送终止信号失败");
            return;
        }
    }



    let msg_type = first_quic_msg.msg_type.clone();

    let connection_key = format!("{}{}{}{}", "QUIC:SERVER:",first_quic_msg.account,":",first_quic_msg.msg_type.to_string());
    let connection_key = connection_key.to_uppercase();
    info!("connection key: {}", connection_key);
    let close_key = connection_key.clone();

    //通过原子计数和异步锁共享变量
    let send_stream = Arc::new(RwLock::new(send_stream));

    let now = get_now_time_stamp_as_millis().unwrap_or_else(|_| 0);
    let close_now = now.clone();
    let new_connection = QuicConnection {
        is_online: true,
        account: first_quic_msg.account,
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
        let mut conn = redis.get().await.expect("打开redis连接失败");
        conn
            .set::<&str, &str, ()>(&connection_key, "SERVER_1")
            .await
            .unwrap_or_else(|x| error!("插入redis失败! {}",x.to_string()))
    }

    info!(
        "当前的在线客户端 {}",
        GLOBAL_QUIC_SERVER_LIST.read().await.len()
    );
    let buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));

    loop {
        // 循环处理流中的数据
        let mut buffer = vec![0u8; 1024 * 10]; //设置缓冲区为10KB
        let change_buffer = &mut buffer;
        match recv_stream.read(change_buffer).await {
            Ok(Some(length)) => {
                let new_close_key = close_key.clone();

                match process_rec_msg(
                    change_buffer,
                    length,
                    new_close_key,
                    &msg_type,
                    buffer_msg.clone(),
                    head_length
                )
                .await
                {
                    Ok(_) => {
                        info!("处理消息完成")
                    }
                    Err(error) => {
                        error!("处理信息失败! {:#}", error);
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

    {
        let mut server_book = GLOBAL_QUIC_SERVER_LIST.write().await;
        if let Some(book) = server_book.get_mut(&close_key) {
            let now = book.update_time;
            if now == close_now as u64 {
                info!("用户下线 {}", close_key);
                server_book.remove(&close_key);
            }
        }
    }

    info!(
        "[服务器] 处理完成连接 {} 完成, 在线连接数为 {}",
        close_key,
        GLOBAL_QUIC_SERVER_LIST.read().await.len()
    );
}



