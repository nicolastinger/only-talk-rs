use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use dashmap::DashMap;
use deadpool_redis::redis::{AsyncCommands, cmd};
use entity::config_str::SYSTEM;
use entity::utils::jwt_util::decode_jwt;
use entity::utils::message_types;
use entity::utils::redis_utils::{acquire_lock, get_redis_conn, release_lock};
use tracing::{error, info, warn};
use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::msg_service::get_connection_by_uuid;
use crate::msg_service::text_msg_service::generate_text_msg;
use tokio::net::UdpSocket;
use tokio::signal;

use crate::nat_ip::model::UserAddressInfo;

pub async fn run_udp_server(connections: Arc<DashMap<String, QuicConnection>>) -> Result<(), anyhow::Error> {
    tokio::spawn(async move {
        let addr_1 = "0.0.0.0:9562";
        let addr_2 = "[::]:9563";
        let addr_3 = "0.0.0.0:9564";
        let addr_4 = "[::]:9565";

        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let handle1 = {
            let shutdown = shutdown_flag.clone();
            let conns = connections.clone();
            tokio::spawn(async move {
                get_p2p_udp_socket_with_shutdown(addr_1, "V4".to_string(), shutdown, conns)
                    .await
                    .expect("9562 Failed to get UDP socket");
            })
        };

        let handle2 = {
            let shutdown = shutdown_flag.clone();
            let conns = connections.clone();
            tokio::spawn(async move {
                get_p2p_udp_socket_with_shutdown(addr_2, "V6".to_string(), shutdown, conns)
                    .await
                    .expect("9563 Failed to get UDP socket");
            })
        };

        let handle3 = {
            let shutdown = shutdown_flag.clone();
            let conns = connections.clone();
            tokio::spawn(async move {
                get_p2p_udp_socket_with_shutdown(addr_3, "V4".to_string(), shutdown, conns)
                    .await
                    .expect("9564 to get UDP socket");
            })
        };

        let handle4 = {
            let shutdown = shutdown_flag.clone();
            let conns = connections.clone();
            tokio::spawn(async move {
                get_p2p_udp_socket_with_shutdown(addr_4, "V6".to_string(), shutdown, conns)
                    .await
                    .expect("9565 to get UDP socket");
            })
        };

        signal::ctrl_c().await.expect("无法注册 Ctrl+C 处理器");
        info!("收到 Ctrl+C 信号，正在关闭服务...");

        shutdown_flag.store(true, Ordering::Relaxed);

        let _ = tokio::join!(handle1, handle2, handle3, handle4);
    });
    Ok(())
}

pub async fn get_p2p_udp_socket_with_shutdown(
    address: &str,
    ip_type: String,
    shutdown: Arc<AtomicBool>,
    connections: Arc<DashMap<String, QuicConnection>>,
) -> anyhow::Result<()> {
    let socket = UdpSocket::bind(address).await?;
    info!("nat服务端已启动，监听地址 {}", address);

    let mut buf = [0u8; 1024];

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("收到 Ctrl+C 信号，正在关闭 {} 服务...", address);
                return Ok(());
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)), if shutdown.load(Ordering::Relaxed) => {
                info!("收到退出信号，正在关闭 {} 服务...", address);
                return Ok(());
            }
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((size, src)) => {
                        let client_ip = src.ip();
                        let client_port = src.port();

                        let udp_addr = format!("{}:{}", client_ip, client_port);
                        info!("收到来自 {}:{} 的消息", udp_addr, size);

                        let res = serde_json::from_slice::<UserAddressInfo>(&buf[..size]);
                        match res {
                           Ok(msg) => {
                              let conns = connections.clone();
                              let ip = ip_type.clone();
                              process_p2p_user_info(udp_addr, ip, msg, conns).await.unwrap_or_else(|err| {
                                error!("处理用户p2p连接失败 {}", err);
                            })
                           },
                           Err(e) => {
                            error!("序列化p2p信息失败，来源{}:{},{}", client_ip, client_port, e);
                            buf[..size].fill(0);
                            continue;
                           }
                        };
                        buf[..size].fill(0);
                    }
                    Err(e) => error!("接收错误: {}", e),
                }
            }
        }
    }
}

pub async fn get_p2p_udp_socket(address: &str, ip_type: String, connections: Arc<DashMap<String, QuicConnection>>) -> anyhow::Result<()> {
    let socket = UdpSocket::bind(address).await?;
    info!("nat服务端已启动，监听地址 {}", address);

    let mut buf = [0u8; 1024];

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("收到 Ctrl+C 信号，正在关闭 {} 服务...", address);
                return Ok(());
            }
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((size, src)) => {
                        let client_ip = src.ip();
                        let client_port = src.port();

                        let udp_addr = format!("{}:{}", client_ip, client_port);

                        match serde_json::from_slice::<UserAddressInfo>(&buf[..size]){
                           Ok(msg) => {
                              let conns = connections.clone();
                              let ip = ip_type.clone();
                              process_p2p_user_info(udp_addr, ip, msg, conns).await.unwrap_or_else(|err| {
                                error!("处理用户p2p连接失败 {}", err);
                            })
                           },
                           Err(e) => {
                            error!("序列化p2p信息失败，来源{}:{},{}", client_ip, client_port, e);
                            buf[..size].fill(0);
                            continue;
                           }
                        };
                        buf[..size].fill(0);
                    }
                    Err(e) => error!("接收错误: {}", e),
                }
            }
        }
    }
}

async fn process_p2p_user_info(
    udp_addr: String,
    ip_type: String,
    mut user_address_info: UserAddressInfo,
    connections: Arc<DashMap<String, QuicConnection>>,
) -> Result<(), anyhow::Error> {
    match decode_jwt(user_address_info.token.as_ref()) {
        Ok(claims) => {
            let uuid = claims.uuid;
            info!("收到来自 {} 的消息, 用户uuid: {}", udp_addr, uuid);
            let key = format!("{}{}_{}", "USER_UDP_ADDRESS_", ip_type, uuid);
            let lock_key = format!("{}{}_{}", "USER_UDP_ADDRESS_LOCK_", ip_type, uuid);
            let lock_key = lock_key.to_uppercase();
            let key = key.to_uppercase();
            let mut acquire_flag = false;
            user_address_info.address = udp_addr;

            {
                let mut conn = get_redis_conn().await?;
                let lock_id =
                    acquire_lock(&mut conn, &lock_key, 30, user_address_info.address.clone())
                        .await?;
                if let Some(item) = lock_id {
                    acquire_flag = true;
                    user_address_info.lock_uuid = item;
                }
            }

            if !acquire_flag {
                info!("进入redis锁");
                let mut conn = get_redis_conn().await?;
                let result: String = conn.get(&lock_key).await?;
                let user_addr = result.split('_').skip(1).collect::<Vec<&str>>().join("");

                info!("获取自身值为 {:?},值2为 {:?}", user_addr, user_address_info);
                if user_addr == user_address_info.address {
                    user_address_info.nat_type = 3;
                } else {
                    user_address_info.nat_type = 4;
                }
                let mut conn = get_redis_conn().await?;
                release_lock(&mut conn, &lock_key, &result).await?;
                user_address_info.is_lock = true;
                user_address_info.token = "".to_string();
            }

            match get_target_user_address_info(&user_address_info.target_uuid, &ip_type).await {
                Ok(mut target_user_address_info) => {
                    info!("进入到服务器和连接端匹配");
                    if target_user_address_info.is_lock && !acquire_flag {
                        {
                            let mut conn = get_redis_conn().await?;
                            conn.del::<_, ()>(&key).await?;
                            let target_key = format!(
                                "{}{}_{}",
                                "USER_UDP_ADDRESS_", ip_type, target_user_address_info.uuid
                            );
                            let target_key = target_key.to_uppercase();
                            conn.del::<_, ()>(&target_key).await?;
                        }
                        match (user_address_info.nat_type, target_user_address_info.nat_type) {
                            (4, 4) => {
                                error!(
                                    "双方均为对称型NAT，无法建立连接!停止处理 {},{}",
                                    user_address_info.uuid, target_user_address_info.uuid
                                );
                                return Ok(());
                            }
                            (3, _) => {
                                user_address_info.is_server = true;
                                target_user_address_info.is_server = false;
                                info!(
                                    "发起方 {}，接收方 {}",
                                    user_address_info.uuid, target_user_address_info.uuid
                                );
                            }
                            (4, 3) => {
                                target_user_address_info.is_server = true;
                                user_address_info.is_server = false;
                                info!(
                                    "接收方 {}，发起方 {}",
                                    target_user_address_info.uuid, user_address_info.uuid
                                );
                            }
                            _ => {
                                error!("匹配为空，不处理");
                                return Ok(());
                            }
                        }
                        let mut server = {
                            if user_address_info.is_server {
                                message_types::MSG_TYPE_P2P_USER_CLIENT
                            } else {
                                message_types::MSG_TYPE_P2P_USER_SERVER
                            }
                        };
                        {
                            let conn = get_connection_by_uuid(
                                &target_user_address_info.uuid,
                                &ConnectionType::Text.to_string(),
                                &connections,
                            )
                            .await?;

                            let user_address_info_vec = serde_json::to_vec(&user_address_info)?;
                            let msg_raw = generate_text_msg(
                                server,
                                user_address_info_vec,
                                target_user_address_info.uuid.clone(),
                                SYSTEM.to_string(),
                            )?;
                            let mut send = conn.open_uni().await?;
                            send.write_all(&msg_raw).await?;
                            send.finish().await?;
                        }

                        {
                            let conn = get_connection_by_uuid(
                                &user_address_info.uuid,
                                &ConnectionType::Text.to_string(),
                                &connections,
                            )
                            .await?;
                            if server == message_types::MSG_TYPE_P2P_USER_CLIENT {
                                server = message_types::MSG_TYPE_P2P_USER_SERVER
                            }

                            let target_user_address_info_vec =
                                serde_json::to_vec(&target_user_address_info)?;
                            let msg_raw = generate_text_msg(
                                server,
                                target_user_address_info_vec,
                                user_address_info.uuid.clone(),
                                SYSTEM.to_string(),
                            )?;
                            let mut send = conn.open_uni().await?;
                            send.write_all(&msg_raw).await?;
                            send.finish().await?;
                        }
                    }
                    info!("转发建立p2p信息完成");
                    return Ok(());
                }
                Err(e) => {
                    warn!("获取目标用户信息失败 {}，等待目标用户上传redis", e);
                }
            }

            {
                info!("插入用户信息到redis中");
                let mut conn = get_redis_conn().await?;
                let user_address_info_json =
                    serde_json::to_string(&user_address_info).unwrap_or_default();
                let _: () = cmd("SET")
                    .arg(&key)
                    .arg(&user_address_info_json)
                    .arg("EX")
                    .arg(60)
                    .query_async(&mut conn)
                    .await
                    .unwrap_or_else(|e| error!("新增用户连接信息失败 {}", e));
            }
        }
        Err(e) => {
            error!("获取token失败 {}", e.backtrace());
        }
    };

    Ok(())
}

async fn get_target_user_address_info(
    target_uuid: &String,
    ip_type: &String,
) -> Result<UserAddressInfo, anyhow::Error> {
    let mut conn = get_redis_conn().await?;
    let key = format!("{}{}_{}", "USER_UDP_ADDRESS_", ip_type, target_uuid);
    let key = key.to_uppercase();
    let result: String = conn.get(&key).await?;
    let mut target_user_address_info: UserAddressInfo = serde_json::from_str(&result)?;
    target_user_address_info.token = String::new();
    Ok(target_user_address_info)
}
