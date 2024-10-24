use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use log::{error, info};
use quinn::SendStream;
use tokio::sync::{Mutex, RwLock};
use crate::common::quic_network_service::make_server_endpoint;
use crate::common::quic_network_service::quic_connection::{ConnectionType, QuicConnection};
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

    //通过原子计数和异步锁共享变量
    let send_stream = Arc::new(RwLock::new(send_stream));
    let recv_stream = Arc::new(RwLock::new(recv_stream));

    let new_connection = QuicConnection {
        is_online: true,
        connection: conn,
        user_id: "".to_string(),
        connection_type: ConnectionType::Text,
        send_stream: send_stream.clone(),
        recv_stream: recv_stream.clone(),
        create_time: 0,
        update_time: 0,
        ipv4addr: "".to_string(),
        ipv6addr: "".to_string(),
    };

    {
        let mut server_book = GLOBAL_QUIC_SERVER_LIST.write().await;
        server_book.insert(new_connection.connection.remote_address().to_string(), new_connection);
    }


    info!("当前的客户端列表 {}",GLOBAL_QUIC_SERVER_LIST.read().await.len());
    info!("[server] 流已接受: ID={}", recv_stream.read().await.id()); // 打印流ID*/

    tokio::spawn(async move {
        // 异步处理流中的数据
        let mut buffer = vec![0u8; 1024 * 1024];  //设置缓冲区为1MB
        let mut my_recv_stream = recv_stream.clone().write().await;
        loop {
            match my_recv_stream.read(&mut buffer).await {
                Ok(Some(length)) => {
                    info!("[服务端] 长度为 {} 流数据: {:?}", length, String::from_utf8_lossy(&buffer[0..length]));
                    send_stream.write().await.write_all("这是服务端发送的信息1111".as_bytes()).await.expect("TODO: panic message");
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
                    error!("[服务端] 读取错误: {},退出流{}", e,my_recv_stream.id());
                    break;
                }
            }
        }
    });

    // 保持主任务运行，等待所有任务完成
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    info!("[服务器] 处理完成");
}

fn process_rec_msg(buffer: &Vec<u8>) {
    let msg = std::str::from_utf8(&buffer).unwrap();
    info!("[服务器] 收到的信息为 {}",msg)
}

async fn send_text_msg(send_stream: Arc<Mutex<SendStream>>, mut text: String) {
    send_stream.lock().await.write_all(text.as_bytes()).await.expect("报错了哥们")
}