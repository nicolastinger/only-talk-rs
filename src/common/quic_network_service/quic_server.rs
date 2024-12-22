use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use anyhow::{Context, Result};
use backtrace::Backtrace;
use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;
use log::{error, info};
use quinn::SendStream;
use redis::Msg;
use tokio::sync::{Mutex, RwLock};
use crate::common::quic_network_service::make_server_endpoint;
use crate::common::quic_network_service::quic_connection::{ConnectionType, FirstQuicMsg, QuicConnection, TextQuicMsg};
use crate::{GLOBAL_QUIC_SERVER_LIST, QUIC_MSG_SPLIT};
use crate::utils::jwt_util::decode_jwt;

pub(crate) fn init_server(redis: Pool, addr: SocketAddr) {
    tokio::spawn(run_server(addr, redis));
}

/// 启动并运行QUIC服务器，持续监听新连接
async fn run_server(addr: SocketAddr,redis: Pool) {
    // 创建服务器端点和证书
    let (endpoint, _server_cert) = make_server_endpoint(addr).unwrap();
    info!("quic服务器启动成功,使用地址为: {}", addr);


    // 持续监听新的连接请求
    loop {
        let incoming_conn = endpoint.accept().await.unwrap(); // 接收新的连接请求
        let conn = incoming_conn.await.unwrap(); // 确认连接建立

        let new_pool = redis.clone();
        info!(
            "[服务端] 连接已接受: 地址={}",
            conn.remote_address() // 打印客户端地址
        );
        tokio::spawn(async move {
            // 异步处理每个连接
            handle_conn(conn,new_pool).await;
        });
    }
}

async fn handle_conn(conn: quinn::Connection, redis: Pool) {
    // 打开双向流
    let (mut send_stream, mut recv_stream) = match conn.accept_bi().await {
        Ok((send, recv)) => (send, recv),
        Err(e) => {
            error!("打开双向流失败: {}", e);
            return;
        }
    };

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
            error!("[服务端] 发送流初始化元数据失败: {}",conn.remote_address());
            //return;
        }
        Err(e) => {
            error!("[服务端] 初始化读取元数据错误: {},退出流{}", e,conn.remote_address());
        }
    }

    match decode_jwt(first_quic_msg.token.as_ref()).map_err(|_| "解析token失败") {
        Ok(t) => {
            if t != first_quic_msg.user_id {
                error!("令牌跟账号不匹配！");
                send_stream.finish().await.expect("发送终止信号失败");
            }
        }
        Err(_) => {
            error!("解析令牌失败");
            send_stream.finish().await.expect("发送终止信号失败");
        }
    }


    let msg_type = first_quic_msg.msg_type.clone();

    let connection_key = "QUIC:SERVER:".to_string() + &*first_quic_msg.user_id.clone() + ":" + &*first_quic_msg.msg_type.clone().to_string();


    let connection_key = connection_key.to_uppercase();
    info!("connection_key {}",connection_key);
    let close_key = connection_key.clone();

    //通过原子计数和异步锁共享变量
    let send_stream = Arc::new(RwLock::new(send_stream));

    // 获取当前系统时间
    let start = SystemTime::now();

    let now = start.duration_since(UNIX_EPOCH).unwrap().as_millis();
    let new_connection = QuicConnection {
        is_online: true,
        connection: conn,
        user_id: "".to_string(),
        connection_type: ConnectionType::Text,
        send_stream: send_stream.clone(),
        create_time: now as u64,
        update_time: now as u64,
        ipv4addr: "".to_string(),
        ipv6addr: "".to_string(),
        first_quic_msg,
    };

    info!("插入写锁");
    {
        let mut server_book = GLOBAL_QUIC_SERVER_LIST.write().await;
        server_book.insert(connection_key.clone(), new_connection);
    }
    info!("释放写锁");
    {
        let mut conn = redis.get().await.expect("打开redis连接失败");
        match conn.set::<&str, &str, ()>(&connection_key,"SERVER_1").await{
            Ok(_)=>info!("添加成功"),
            Err(_)=>error!("添加失败")
        }
    }
    
    info!("当前的客户端列表 {}",GLOBAL_QUIC_SERVER_LIST.read().await.len());
    info!("[server] 流已接受: ID={}", recv_stream.id()); // 打印流ID*/

    loop {
        // 异步处理流中的数据
        let mut buffer = vec![0u8; 1024 * 10];  //设置缓冲区为10KB
        match recv_stream.read(&mut buffer).await {
            Ok(Some(length)) => {
                info!("[服务端] 长度为 {} 流数据: {:?}", length, String::from_utf8_lossy(&buffer[0..length]));
                let msg_type = msg_type.clone();
                let new_close_key = close_key.clone();

                match process_rec_msg(&buffer, length, new_close_key, msg_type).await {
                    Ok(_) => {}
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
                error!("[服务端] 读取错误: {},退出流{}", e,recv_stream.id());
                break;
            }
        }
    }

    {
        let mut server_book = GLOBAL_QUIC_SERVER_LIST.write().await;
        server_book.remove(&close_key);
    }

    info!("[服务器] 处理完成 {}",GLOBAL_QUIC_SERVER_LIST.read().await.len());
}

async fn process_rec_msg(buffer: &Vec<u8> ,length : usize,close_key: String, msg_type: ConnectionType) -> Result<()> {
    let mut my_send_stream =
        {
            let bind = GLOBAL_QUIC_SERVER_LIST.read().await;
            let send = bind.get(&close_key).unwrap();
            send.send_stream.clone()
        };

    match msg_type {
        ConnectionType::Text => {
            let msg = String::from_utf8_lossy(&buffer[0..length]).to_string();
            info!("收到消息为 {}",msg);
            match msg.as_str() {
                "ping" => {
                    my_send_stream.write().await.write_all("pong".as_bytes()).await?;
                    return Ok(());
                }
                _ => {}
            }
            process_text_msg(my_send_stream, msg, &close_key).await.context("处理文本信息出错")?;
        }
        ConnectionType::Img => {}
        ConnectionType::Video => {}
        ConnectionType::File => {}
        ConnectionType::Other => {}
    }
    Ok(())
}

async fn process_text_msg(mut send_stream: Arc<RwLock<SendStream>>, msg: String, close_key: &str) -> Result<()> {
    info!("[服务器] 收到的信息为 {}",msg);
    let msg = msg.trim_end_matches('\0');
    let split: Vec<&str> = msg.split(QUIC_MSG_SPLIT).collect();
    let mut msg_hash_map = HashMap::<String, String>::new();
    info!("实体 {:?}", split);
    //根据收到的消息依次转发
    for single_msg in split.iter() {
        //info!("实体 {:?}", single_msg);
        let text_msg: TextQuicMsg = serde_json::from_str(single_msg).context("序列化json失败")?;

        let user = text_msg.recv_user;
        // 使用 entry API 简化累加操作
        msg_hash_map.entry(user)
            .and_modify(|e| *e += &format!("{}{}", text_msg.raw, QUIC_MSG_SPLIT))
            .or_insert_with(|| format!("{}{}", single_msg, QUIC_MSG_SPLIT));
    }

    for (key, val) in msg_hash_map.iter() {
        info!("获取读锁 {}",key);
        let user_key = "QUIC:SERVER:".to_string() + &*key + ":" + &*ConnectionType::Text.to_string();
        let user_key = user_key.to_uppercase();
        let mut my_send_stream: Option<Arc<RwLock<SendStream>>> = {
            let bind = GLOBAL_QUIC_SERVER_LIST.read().await;
            match bind.get(&user_key) {
                Some(s) => Some(s.send_stream.clone()),
                None => {
                    error!("当前用户不在线: {}", user_key);
                    None
                }
            }
        };

        if let Some(mut current_send_stream) = my_send_stream {
            current_send_stream.write().await.write_all(val.as_bytes()).await?;
            info!("释放读锁");
        } else {
            // 处理 my_send_stream 为 None 的情况
            info!("用户不在线，无法发送消息: {}", user_key);
            // 这里可以添加其他处理逻辑
        }
    }

    Ok(())
}

async fn send_text_msg(send_stream: Arc<Mutex<SendStream>>, mut text: String) {
    send_stream.lock().await.write_all(text.as_bytes()).await.expect("报错了哥们")
}