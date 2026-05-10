use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use entity::config_str::{PC_PLATFORM, PING, SYSTEM};
use entity::utils::jwt_util::get_jwt;
use entity::utils::message_types;
use tracing::{error, info};
use quinn::{Connection, Endpoint, SendStream};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::models::first_quic_msg::FirstQuicMsg;
use crate::models::quic_connection::ConnectionType;
use crate::msg_service::text_msg_service::{generate_text_msg, get_text_msg};
use crate::set_server::configure_client;

#[allow(dead_code)]
pub async fn run_client(server_addr: SocketAddr) {
    // 创建客户端端点
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().expect("infallible")).expect("infallible");
    endpoint.set_default_client_config(configure_client()); // 设置默认客户端配置

    // 尝试连接到服务器
    let connection = endpoint
        .connect(server_addr, "onlytalk.cn")
        .expect("Failed to create endpoint")
        .await
        .expect("Failed to connect to server");
    info!("[client] connected: addr={}", connection.remote_address()); // 打印连接成功的服务器地址

    // 开启一个双向流用于初始化和接收
    let (send_stream, mut _recv_stream) =
        connection.open_bi().await.expect("Failed to open stream");
    send_stream.set_priority(0).expect("Failed to set priority"); // 设置优先级
    let head_length = 9;
    let buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    // bidi recv loop（初始化流接收）
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 1024 * 8];
        loop {
            match _recv_stream.read(&mut buffer).await {
                Ok(Some(length)) => {
                    match process_rec_msg(
                        &mut buffer,
                        length,
                        &ConnectionType::Text,
                        buffer_msg.clone(),
                        head_length,
                    )
                    .await
                    {
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

    // uni stream 接收循环（服务端通过 open_uni 推送消息）
    {
        let conn_for_uni = connection.clone();
        tokio::spawn(async move {
            let uni_buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            loop {
                match conn_for_uni.accept_uni().await {
                    Ok(mut recv) => {
                        let mut buf = vec![0u8; 1024 * 8];
                        match recv.read(&mut buf).await {
                            Ok(Some(length)) => {
                                let _ = process_rec_msg(
                                    &mut buf,
                                    length,
                                    &ConnectionType::Text,
                                    uni_buffer_msg.clone(),
                                    head_length,
                                )
                                .await;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                error!("[client] uni流读取错误: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("[client] uni accept 错误: {}", e);
                        break;
                    }
                }
            }
        });
    }

    match init_send_msg(send_stream, connection).await {
        Ok(_) => {
            info!("客户端初始化连接成功")
        }
        Err(_) => {
            error!("客户端初始化连接失败")
        }
    }
}

async fn init_send_msg(mut send_stream: SendStream, conn: Connection) -> Result<(), anyhow::Error> {
    // 发送消息给服务器
    let uuid = "01965d95-0ffc-7d23-911e-1111485fb9be".to_string();
    let mut first_quic_msg = FirstQuicMsg::new();
    first_quic_msg.dyn_header_size = 9;
    first_quic_msg.uuid = uuid.clone();
    first_quic_msg.text_serde_struct = "user_chat_json".to_string();
    first_quic_msg.msg_type = ConnectionType::Text;
    let token = get_jwt(uuid.clone(), PC_PLATFORM.to_string()).expect("获取token失败");
    first_quic_msg.token = token;

    let first_msg_json = serde_json::to_string(&first_quic_msg)?;
    info!("[客户端] 准备发送初始化消息: {}", first_msg_json);

    send_stream.write_all(first_msg_json.as_bytes()).await?;
    send_stream.flush().await?; // 确保数据被立即发送

    info!("[客户端] 初始化消息发送完成，等待服务器响应");

    tokio::time::sleep(Duration::from_secs(1)).await; //初始化一秒，防止连发元数据

    let test_msg = generate_text_msg(
        message_types::MSG_TYPE_TEXT,
        "上山打老虎".as_bytes().to_vec(),
        uuid.clone(),
        uuid.clone(),
    )?;

    let test_msg2 = generate_text_msg(
        message_types::MSG_TYPE_TEXT,
        "我是蔡徐坤".as_bytes().to_vec(),
        uuid.clone(),
        uuid.clone(),
    )?;

    // 按需开流发送测试消息
    send_via_new_stream(&conn, &test_msg).await?;
    send_via_new_stream(&conn, &test_msg2).await?;
    send_via_new_stream(&conn, &test_msg2).await?;
    send_via_new_stream(&conn, &test_msg2).await?;
    send_via_new_stream(&conn, &test_msg2).await?;

    // 心跳循环 - 按需开流
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let ping_msg = generate_text_msg(
                message_types::MSG_TYPE_PING,
                PING.as_bytes().to_vec(),
                SYSTEM.to_string(),
                uuid.clone(),
            )
            .expect("");
            match send_via_new_stream(&conn, &ping_msg).await {
                Ok(_) => {
                    info!("发送成功");
                }
                Err(e) => {
                    error!("发送心跳失败 {}", e);
                }
            };
        }
    });
    Ok(())
}

/// 按需开流发送数据
async fn send_via_new_stream(conn: &Connection, data: &[u8]) -> Result<(), anyhow::Error> {
    let mut send = conn.open_uni().await?;
    send.write_all(data).await?;
    send.finish().await?;
    Ok(())
}

async fn process_rec_msg(
    buffer: &mut Vec<u8>,
    length: usize,
    msg_type: &ConnectionType,
    buffer_msg: Arc<Mutex<Vec<u8>>>,
    head_length: usize,
) -> anyhow::Result<()> {
    match msg_type {
        ConnectionType::Text => {
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
