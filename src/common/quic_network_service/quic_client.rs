use crate::common::quic_network_service::configure_client;
use crate::common::quic_network_service::models::quic_connection::{ConnectionType, FirstQuicMsg};
use crate::common::quic_network_service::models::text_msg::{HeadMsg, TextMsg, TextQuicMsg};
use crate::common::quic_network_service::msg_service::text_msg_service::{generate_text_msg, get_text_msg};
use log::{error, info};
use quinn::{Endpoint, SendStream};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

// 客户端异步函数，尝试与服务器建立QUIC连接
pub async fn run_client(server_addr: SocketAddr) {
    // 创建客户端端点
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap()).expect("infallible");
    endpoint.set_default_client_config(configure_client()); // 设置默认客户端配置

    // 尝试连接到服务器
    let connection = endpoint
        .connect(server_addr, "onlytalk.cn")
        .unwrap()
        .await
        .map_err(|_| "Failed to connect to server".to_string())
        .unwrap();
    info!("[client] connected: addr={}", connection.remote_address()); // 打印连接成功的服务器地址

    // 开启一个双向流
    let (mut send_stream, mut _recv_stream) = connection.open_bi().await.unwrap();
    send_stream.set_priority(0).unwrap(); // 设置优先级
    let head_length = 9;
    let buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    // 异步处理流中的数据
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 1024 * 8];
        loop {
            match _recv_stream.read(&mut buffer).await {
                Ok(Some(length)) => {
                    match process_rec_msg(&mut buffer, length, &ConnectionType::Text,buffer_msg.clone(), head_length).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("[client] Failed to process_rec_msg {}", e);
                        }
                    };
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
    match init_send_msg(send_stream).await {
        Ok(_) => {
            info!("客户端初始化连接成功")
        }
        Err(_) => {
            error!("客户端初始化连接失败")
        }
    }
}

async fn init_send_msg(mut send_stream: SendStream) -> Result<(), anyhow::Error> {
    // 发送消息给服务器

    let mut first_quic_msg = FirstQuicMsg::new();
    first_quic_msg.dyn_header_size = 9;
    first_quic_msg.user_id = "caixukun".to_string();
    first_quic_msg.text_serde_struct = "user_chat_json".to_string();
    first_quic_msg.token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJzdWIiOjEyMzEyMywiYWNjb3VudCI6ImNhaXh1a3VuIiwiZXhwIjoxNzQ1NTQ0NTA5NTE0fQ.cQ7vdPHIzYSASDkFsc3xLh6-AqLlh0uqR5mGhDCwFzlVwNcUTGK_vtA5PHpcCkRLKx4seqSRGhDOzfIYfAmaKlc0e8TSAy6I_j6VRbtkpIS2S-_nCeilSVZEtZXWW-PrRoBaGrAMZakuOOEsN-pxlW7lTkoMtTbalAy6KoligeDOj1PZw-n2tlX9-Wc9Ks_2rsAfYK0uFXE-FlhIMwScyxuxuG42Un6DJBrXmh17E8Lyj4hYM8V7Bd8UQKMWJBSwdelynAJHboYVX8Pn-cjn-YOcXk3TvM25Y-Ymvggj-c_T2i8zwP-DdycrMwxfbWkCn9t5aDGWMSFal6mtcgdIMw".to_string();
    send_stream
        .write_all(serde_json::to_string(&first_quic_msg)?.as_bytes())
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;  //初始化一秒，防止连发元数据

    let send_stream = Arc::new(Mutex::new(send_stream));

    let test_msg = generate_text_msg(
        "1".to_string(),
        "上山打老虎".to_string(),
        "caixukun".to_string(),
        "huangxiaoming".to_string(),
    )?;

    let test_msg2 = generate_text_msg(
        "1".to_string(),
        "我是蔡徐坤".to_string(),
        "caixukun".to_string(),
        "huangxiaoming".to_string(),
    )?;

    send_msg(test_msg, send_stream.clone()).await?;
    send_msg(test_msg2.clone(), send_stream.clone()).await?;
    send_msg(test_msg2.clone(), send_stream.clone()).await?;
    send_msg(test_msg2.clone(), send_stream.clone()).await?;
    send_msg(test_msg2, send_stream.clone()).await?;

    let send_stream_ping = send_stream.clone();
    tokio::spawn(async move {
        loop {
            //一分钟发送心跳
            tokio::time::sleep(Duration::from_secs(6)).await;
            info!("发送心跳");
            match send_stream_ping.lock().await.write_all("ping".as_bytes()).await {
              Ok(_) => {
                  info!("发送成功");
              },
              Err(e) => {
                  error!("发送心跳失败 {}", e);
              }
            };
        }
    });
    Ok(())
}

//发送文本信息
async fn send_msg(
    text_msg: Vec<u8>,
    send_stream: Arc<Mutex<SendStream>>,
) -> Result<String, anyhow::Error> {
    send_stream
        .lock()
        .await
        .write_all(&text_msg)
        .await?;
    Ok("success".to_string())
}

async fn process_rec_msg(
    buffer: &mut Vec<u8>,
    length: usize,
    msg_type: &ConnectionType,
    buffer_msg: Arc<Mutex<Vec<u8>>>,
    head_length: usize
) -> anyhow::Result<()>{
    match msg_type {
        ConnectionType::Text => {
            if length < 16 {
                let msg = String::from_utf8_lossy(&buffer[0..length]).to_string();
                match msg.as_str() {
                    "pong" => {
                        info!("接受心跳信息");
                        return Ok(());
                    }
                    _ => {}
                }
            }
            let text_vec = get_text_msg(buffer, length, buffer_msg, head_length).await?;
            info!("服务器返回的消息为 {:?}", text_vec);
        }
        ConnectionType::Img => {}
        ConnectionType::Video => {}
        ConnectionType::File => {}
        ConnectionType::Other => {}
    }
    Ok(())
}
