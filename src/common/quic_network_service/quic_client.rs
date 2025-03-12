use crate::common::quic_network_service::configure_client;
use crate::common::quic_network_service::models::quic_connection::FirstQuicMsg;
use crate::common::quic_network_service::models::text_msg::{HeadMsg, TextMsg, TextQuicMsg};
use crate::common::quic_network_service::msg_service::text_msg_service::{generate_text_msg, get_text_msg};
use crate::utils::time::get_now_time_stamp_as_millis;
use log::{error, info};
use quinn::{Connection, Endpoint, SendStream};
use std::error::Error;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use validator::HasLen;

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
        .unwrap();
    info!("[client] connected: addr={}", connection.remote_address()); // 打印连接成功的服务器地址

    // 开启一个双向流
    let (mut send_stream, mut _recv_stream) = connection.open_bi().await.unwrap();
    send_stream.set_priority(0).unwrap(); // 设置优先级
    let head_length = 17;
    let buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    // 异步处理流中的数据
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 1024 * 8];
        loop {
            match _recv_stream.read(&mut buffer).await {
                Ok(Some(length)) => {
                    let text_vec = get_text_msg(&mut buffer, length, buffer_msg.clone(), head_length).await.unwrap();
                    info!("服务器返回的消息为 {:?}", text_vec);
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

async fn init_send_msg(mut send_stream: SendStream) -> Result<(), Box<dyn Error>> {
    // 发送消息给服务器

    let mut first_quic_msg = FirstQuicMsg::new();
    first_quic_msg.dyn_header_size = 17;
    first_quic_msg.user_id = "huangxiaoming".to_string();
    first_quic_msg.text_serde_struct = "user_chat_json".to_string();
    first_quic_msg.token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMxMjMiLCJhY2NvdW50IjoiaHVhbmd4aWFvbWluZyIsImV4cCI6MTc0MTE2NjMyOTQ0M30.G0ehd2ZSmoJoSDWZkdQ6oaQU1oFIZtmmzFCXGy1_dbNr3AeSnDpy1Dp_ot0YPqLM2QOQGdhpIVTiOtqgHqNTLL-NS7E9a8NcklPvJGDvysTwkZ9Fwxm8JAqKMqk6ZOhYuAOC1_RT3QVTTaKOqIe2mbD0rLJA6BlIdLXEVCyJmqqJVUTbxHeF1nk1P1p9AUG6-PunXl-5CHa3oDRcLxgeOxWuEud8w5uR1pU0nVUXsAlIHcjdjsaRpVT89CtHRsin_IweUg76n8pvFtswdyk5MpIsScdKZriXY2T8D5fukktJ4EGgigF2BOkVBClkEiLUYTHYzo1AorPV-x2vFny73A".to_string();
    send_stream
        .write_all(serde_json::to_string(&first_quic_msg).unwrap().as_bytes())
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;  //初始化一秒，防止连发元数据

    let mut send_stream = Arc::new(Mutex::new(send_stream));

    let test_msg = generate_text_msg(
        "1".to_string(),
        "上山打老虎".to_string(),
        "liangchaowei".to_string(),
        "huangxiaoming".to_string(),
    )?;

    let test_msg2 = generate_text_msg(
        "1".to_string(),
        "我是蔡徐坤".to_string(),
        "caixukun".to_string(),
        "huangxiaoming".to_string(),
    )?;

    send_msg(test_msg, send_stream.clone()).await.unwrap();
    send_msg(test_msg2.clone(), send_stream.clone()).await.unwrap();
    send_msg(test_msg2.clone(), send_stream.clone()).await.unwrap();
    send_msg(test_msg2.clone(), send_stream.clone()).await.unwrap();
    send_msg(test_msg2, send_stream.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_secs(10000)).await;
    Ok(())
}

//发送文本信息
async fn send_msg(
    text_msg: Vec<u8>,
    send_stream: Arc<Mutex<SendStream>>,
) -> Result<String, String> {
    send_stream
        .lock()
        .await
        .write_all(&text_msg)
        .await
        .map_err(|e| e.to_string())?;
    Ok("success".to_string())
}
