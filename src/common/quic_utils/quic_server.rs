use std::net::SocketAddr;
use log::{error, info};
use crate::common::quic_utils::make_server_endpoint;

pub(crate) fn init_server(){
    // 定义服务器监听地址
    let addr = "127.0.0.1:4433".parse().unwrap();

    let res = tokio::spawn(run_server(addr));
}

/// 启动并运行QUIC服务器，持续监听新连接
async fn run_server(addr: SocketAddr) {
    // 创建服务器端点和证书
    let (endpoint, _server_cert) = make_server_endpoint(addr).unwrap();

    // 持续监听新的连接请求
    loop {
        let incoming_conn = endpoint.accept().await.unwrap(); // 接收新的连接请求
        let conn = incoming_conn.await.unwrap(); // 确认连接建立

        info!(
            "[服务端] 连接已接受: 地址={}",
            conn.remote_address() // 打印客户端地址
        );
        // 异步处理每个连接
        handle_conn(conn).await;
        info!("处理完成")
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

    info!("[server] 流已接受: ID={}", recv_stream.id()); // 打印流ID

    // 异步处理流中的数据
    let mut buffer = vec![0u8; 1024 * 8];
    loop {
        match recv_stream.read(&mut buffer).await {
            Ok(Some(length)) => {
                info!("[服务端] 长度为 {} 流数据: {:?}", length, String::from_utf8_lossy(&buffer[0..length]));
                send_stream.write_all("你在干什么".as_bytes()).await.expect("TODO: panic message");
            }
            Ok(None) => {
                info!("[服务端] 流关闭 {}",recv_stream.id());
                break;
            }
            Err(e) => {
                error!("[服务端] 读取错误: {},退出流{}", e,recv_stream.id());
                break;
            }
        }
    }
    info!("[服务端] 已结束运行");
}