use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc};
use std::time::Duration;
use log::{error, info};
use quinn::{Connection, Endpoint, SendStream};
use tokio::sync::Mutex;
use crate::common::quic_network_service::configure_client;
use crate::common::quic_network_service::quic_connection::{FirstQuicMsg, TextQuicMsg};

// 客户端异步函数，尝试与服务器建立QUIC连接
pub async fn run_client(server_addr: SocketAddr) {
    // 创建客户端端点
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap()).expect("infallible");
    endpoint.set_default_client_config(configure_client()); // 设置默认客户端配置

    // 尝试连接到服务器
    let connection = endpoint.connect(server_addr, "onlytalk.cn").unwrap().await.unwrap();
    info!("[client] connected: addr={}", connection.remote_address()); // 打印连接成功的服务器地址

    // 开启一个双向流
    let (mut send_stream, mut _recv_stream) = connection.open_bi().await.unwrap();
    send_stream.set_priority(0).unwrap(); // 设置优先级
    // 异步处理流中的数据
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 1024 * 8];
        loop {
            match _recv_stream.read(&mut buffer).await {
                Ok(Some(length)) => {
                    info!("[客户端] 长度为 {} 流数据: {:?}", length, String::from_utf8_lossy(&buffer[0..length]));
                }
                Ok(None) => {
                    info!("[客户端]没有接收到数据");
                    break;
                }
                Err(e) => {
                    error!("[客户端] 读取错误: {}", e);
                    break;
                }
            }
        }
    });
    match init_send_msg(send_stream).await{
        Ok(_) => {
            info!("客户端初始化连接成功")
        }
        Err(_) => {
            error!("客户端初始化连接失败")
        }
    }
}

async fn init_send_msg(mut send_stream: SendStream)->Result<(), Box<dyn Error>>{
    // 发送消息给服务器

    let mut first_quic_msg = FirstQuicMsg::new();
    first_quic_msg.user_id = "huangjinsheng".to_string();
    first_quic_msg.text_serde_struct = "user_chat_json".to_string();
    send_stream.write_all(serde_json::to_string(&first_quic_msg).unwrap().as_bytes()).await.unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;

    let mut send_stream = Arc::new(Mutex::new(send_stream));
    let mut send = send_stream.clone();

    tokio::spawn(
        async move{
            let text_quic_msg = TextQuicMsg {
                text_type: "chat".to_string(),
                raw: "我是大帅哥".to_string(),
                recv_user: "huangjinsheng".to_string(),
            };
            send.lock().await.write_all(serde_json::to_string(&text_quic_msg).unwrap().as_bytes()).await.unwrap();
        }
    );


    tokio::time::sleep(Duration::from_secs(10)).await;

    send_stream.lock().await.write_all("我是蔡徐坤2".as_bytes()).await.unwrap();

    tokio::time::sleep(Duration::from_secs(100)).await;
    /*tokio::time::sleep(Duration::from_secs(1)).await;
    send_stream1.finish().await?;

    tokio::time::sleep(Duration::from_secs(1)).await;
    // 等待所有任务完成
    endpoint.wait_idle().await;*/

    Ok(())
}

/*pub async fn send_text_msg(send_stream:SendStream) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub async fn get_quic_connection(endpoint: Endpoint) -> Result<Connection, Box<dyn Error>> {

}*/

