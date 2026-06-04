use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use common::config_str::{PC_PLATFORM, PING, SYSTEM};
use common::utils::jwt_util::generate_access_token;
use common::utils::message_types;
use tracing::{error, info};
use quinn::{Connection, Endpoint, SendStream};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::models::first_quic_msg::FirstQuicMsg;
use crate::models::quic_connection::ConnectionType;
use crate::msg_service::text_msg_service::{generate_text_msg, get_text_msg};
use super::set_server::configure_client;

#[allow(dead_code)]
pub async fn run_client(server_addr: SocketAddr) {
    // Create client endpoint
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap()).unwrap();
    endpoint.set_default_client_config(configure_client()); // Set default client config

    // Try connecting to server
    let connection = match endpoint
        .connect(server_addr, "onlytalk.cn")
    {
        Ok(conn) => match conn.await {
            Ok(c) => c,
            Err(e) => {
                error!("failed to connect to server: {}", e);
                return;
            }
        },
        Err(e) => {
            error!("failed to create connection: {}", e);
            return;
        }
    };
    info!("[client] connected: addr={}", connection.remote_address()); // Print connected server address

    // Open a bi-directional stream for init and receiving
    let (mut send_stream, mut _recv_stream) = match connection.open_bi().await {
        Ok(stream) => stream,
        Err(e) => {
            error!("failed to open bi-directional stream: {}", e);
            return;
        }
    };
    if let Err(e) = send_stream.set_priority(0) {
        error!("failed to set priority: {}", e);
    }
    let head_length = 9;
    let buffer_msg: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    // bidi recv loop (init stream receive)
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
                    info!("[client] no data received");
                    break;
                }
                Err(e) => {
                    error!("[client] read error: {}", e);
                    break;
                }
            }
        }
    });

    // uni stream receive loop (server pushes messages via open_uni)
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
                                error!("[client] uni stream read error: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("[client] uni accept error: {}, continuing to wait", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });
    }

    match init_send_msg(&mut send_stream, connection).await {
        Ok(_) => {
            info!("client init connection successful")
        }
        Err(_) => {
            error!("client init connection failed")
        }
    }

    // Keep bidi send half alive, prevent server from going offline when it sees stream closed
    tokio::spawn(async move {
        let _keep = send_stream;
        std::future::pending::<()>().await;
    });
}

async fn init_send_msg(send_stream: &mut SendStream, conn: Connection) -> Result<(), anyhow::Error> {
    // Send message to server
    let uuid = "01965d95-0ffc-7d23-911e-1111485fb9be".to_string();
    let mut first_quic_msg = FirstQuicMsg::new();
    first_quic_msg.dyn_header_size = 9;
    first_quic_msg.uuid = uuid.clone();
    first_quic_msg.text_serde_struct = "user_chat_json".to_string();
    first_quic_msg.msg_type = ConnectionType::Text;
    let token = generate_access_token(uuid.clone(), PC_PLATFORM.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to get token: {}", e))?;
    first_quic_msg.token = token;

    let first_msg_json = serde_json::to_string(&first_quic_msg)?;
    info!("[client] preparing to send init message: {}", first_msg_json);

    send_stream.write_all(first_msg_json.as_bytes()).await?;
    send_stream.flush().await?; // Ensure data is sent immediately

    info!("[client] init message sent, waiting for server response");

    tokio::time::sleep(Duration::from_secs(1)).await; // Init delay 1 second, prevent sending metadata repeatedly

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

    // Send test messages via on-demand streams
    send_via_new_stream(&conn, &test_msg).await?;
    send_via_new_stream(&conn, &test_msg2).await?;
    send_via_new_stream(&conn, &test_msg2).await?;
    send_via_new_stream(&conn, &test_msg2).await?;
    send_via_new_stream(&conn, &test_msg2).await?;

    // Heartbeat loop - open stream on demand
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            let ping_msg = match generate_text_msg(
                message_types::MSG_TYPE_PING,
                PING.as_bytes().to_vec(),
                SYSTEM.to_string(),
                uuid.clone(),
            ) {
                Ok(m) => m,
                Err(e) => {
                    error!("failed to generate heartbeat message: {}", e);
                    continue;
                }
            };
            match send_via_new_stream(&conn, &ping_msg).await {
                Ok(_) => {
                    info!("sent successfully");
                }
                Err(e) => {
                    error!("failed to send heartbeat: {}", e);
                }
            };
        }
    });
    Ok(())
}

/// Send data via on-demand stream
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
            info!("server response: {:?}", text_vec);
        }
        ConnectionType::Img => {}
        ConnectionType::Video => {}
        ConnectionType::File => {}
        ConnectionType::Other => {}
    }
    Ok(())
}
