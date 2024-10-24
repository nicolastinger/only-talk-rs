use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use log::{error, info};
use quinn::SendStream;
use serde::de::Unexpected::Option;
use tokio::sync::{Mutex, RwLock};
use crate::common::quic_network_service::make_server_endpoint;
use crate::common::quic_network_service::quic_connection::{ConnectionType, FirstQuicMsg, QuicConnection};
use crate::GLOBAL_QUIC_SERVER_LIST;

pub(crate) fn init_server() {
    // 定义服务器监听地址
    let addr = "0.0.0.0:4433".parse().unwrap();

    tokio::spawn(run_server(addr));
}

/// 启动并运行QUIC服务器，持续监听新连接
async fn run_server(addr: SocketAddr) {
    // 创建服务器端点和证书
    let (endpoint, _server_cert) = make_server_endpoint(addr).unwrap();
    info!("quic服务器启动成功,使用地址为: {}", addr);

    // 持续监听新的连接请求
    loop {
        let incoming_conn = endpoint.accept().await.unwrap(); // 接收新的连接请求
        let conn = incoming_conn.await.unwrap(); // 确认连接建立

        info!(
            "[服务端] 连接已接受: 地址={}",
            conn.remote_address() // 打印客户端地址
        );
        tokio::spawn(async move {
            // 异步处理每个连接
            handle_conn(conn).await;
        });
    }
}

async fn handle_conn(conn: quinn::Connection) {
    // 打开双向流
    let (mut send_stream, mut recv_stream) = match conn.accept_bi().await {
        Ok((send, recv)) => (send, recv),
        Err(e) => {
            error!("打开双向流失败: {}", e);
            return;
        }
    };

    let mut first_quic_msg = FirstQuicMsg {
        token: "".to_string(),
        user_id: "".to_string(),
        msg_type: "".to_string(),
        text_serde_struct: "".to_string(),
        dyn_buffer_size: 0,
        file_size: 0,
        file_name: "".to_string(),
        raw_head: "".to_string(),
    };

    let connection_key = first_quic_msg.user_id.clone() + "###" + &*conn.remote_address().to_string() + "###" + &*first_quic_msg.msg_type.clone();

    let close_key = connection_key.clone();

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
                    //send_stream.finish().await.expect("发送终止信号失败");
                    //return;
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

    //通过原子计数和异步锁共享变量
    let send_stream = Arc::new(RwLock::new(send_stream));

    let new_connection = QuicConnection {
        is_online: true,
        connection: conn,
        user_id: "".to_string(),
        connection_type: ConnectionType::Text,
        send_stream: send_stream.clone(),
        create_time: 0,
        update_time: 0,
        ipv4addr: "".to_string(),
        ipv6addr: "".to_string(),
        first_quic_msg,
    };

    {
        let mut server_book = GLOBAL_QUIC_SERVER_LIST.write().await;
        server_book.insert(connection_key, new_connection);
    }


    info!("当前的客户端列表 {}",GLOBAL_QUIC_SERVER_LIST.read().await.len());
    info!("[server] 流已接受: ID={}", recv_stream.id()); // 打印流ID*/

    // 异步处理流中的数据
    let mut buffer = vec![0u8; 1024 * 1024];  //设置缓冲区为1MB
    loop {
        match recv_stream.read(&mut buffer).await {
            Ok(Some(length)) => {
                info!("获取读锁");
                let mut my_send_stream =
                    {
                        let bind = GLOBAL_QUIC_SERVER_LIST.read().await;
                        let send = bind.get(&close_key).unwrap();
                        send.send_stream.clone()
                    };
                info!("释放读锁");
                my_send_stream.write().await.write_all("这是服务端发送的信息1111".as_bytes()).await.expect("TODO: panic message");
                info!("[服务端] 长度为 {} 流数据: {:?}", length, String::from_utf8_lossy(&buffer[0..length]));
                tokio::time::sleep(Duration::from_secs(5)).await;
                info!("开始发送关闭信息");
                send_stream.write().await.finish().await.unwrap();
                process_rec_msg(&buffer);
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

fn process_rec_msg(buffer: &Vec<u8>) {
    let msg = std::str::from_utf8(&buffer).unwrap();
    info!("[服务器] 收到的信息为 {}",msg)
}

async fn send_text_msg(send_stream: Arc<Mutex<SendStream>>, mut text: String) {
    send_stream.lock().await.write_all(text.as_bytes()).await.expect("报错了哥们")
}