use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use log::{error, info};
use quinn::{Connection, Endpoint, SendStream};
use crate::common::quic_utils::configure_client;

// 客户端异步函数，尝试与服务器建立QUIC连接
pub async fn run_client(server_addr: SocketAddr) -> Result<Endpoint, Box<dyn Error>> {
    // 创建客户端端点
    let mut endpoint = Endpoint::client("127.0.0.1:0".parse().unwrap())?;
    endpoint.set_default_client_config(configure_client()); // 设置默认客户端配置

    // 尝试连接到服务器
    let connection = endpoint.connect(server_addr, "onlytalk.cn").unwrap().await.unwrap();
    info!("[client] connected: addr={}", connection.remote_address()); // 打印连接成功的服务器地址

    // 开启一个双向流
    let (mut send_stream1, mut _recv_stream) = connection.open_bi().await?;
    send_stream1.set_priority(0)?; // 设置优先级

    // 异步处理流中的数据
   let mut handle_res = tokio::spawn(async move {
        let mut buffer = vec![0u8; 1024 * 8];
        loop {
            match _recv_stream.read(&mut buffer).await {
                Ok(Some(length)) => {
                    info!("[客户端] 长度为 {} 流数据: {:?}", length, String::from_utf8_lossy(&buffer[0..length]));
                }
                Ok(None) => {
                    info!("[客户端]没有接收到数据")
                }
                Err(e) => {
                    error!("[客户端] 读取错误: {}", e);
                    break;
                }
            }
        }
    });
    // 发送消息给服务器
    send_stream1.write("我是谁".as_bytes()).await.unwrap();

    send_stream1.write("我是蔡徐坤".as_bytes()).await.unwrap();

    handle_res.await?;

    tokio::time::sleep(Duration::from_secs(1)).await;
    send_stream1.finish().await?;

    tokio::time::sleep(Duration::from_secs(1)).await;
    // 等待所有任务完成
    endpoint.wait_idle().await;

    Ok(endpoint)
}

/*pub async fn send_text_msg(send_stream:SendStream) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub async fn get_quic_connection(endpoint: Endpoint) -> Result<Connection, Box<dyn Error>> {

}*/

