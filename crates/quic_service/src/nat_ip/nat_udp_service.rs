use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::models::quic_connection::{ConnectionType, QuicConnection};
use crate::msg_service::get_connection_by_uuid;
use crate::msg_service::text_msg_service::generate_text_msg;
use common::config_str::SYSTEM;
use common::read_global_config;
use common::utils::jwt_util::verify_token;
use common::utils::message_types;
use common::utils::redis_utils::{acquire_lock, get_redis_conn, release_lock};
use dashmap::DashMap;
use deadpool_redis::redis::{AsyncCommands, cmd};
use tokio::net::UdpSocket;
use tokio::signal;
use tracing::{error, info, warn};

use crate::nat_ip::model::UserAddressInfo;

pub async fn run_udp_server(
    connections: Arc<DashMap<String, QuicConnection>>,
) -> Result<(), anyhow::Error> {
    let v4_port_1: u16 = read_global_config!("nat_udp", "v4_port_1").parse()?;
    let v6_port_1: u16 = read_global_config!("nat_udp", "v6_port_1").parse()?;
    let v4_port_2: u16 = read_global_config!("nat_udp", "v4_port_2").parse()?;
    let v6_port_2: u16 = read_global_config!("nat_udp", "v6_port_2").parse()?;

    tokio::spawn(async move {
        let addr_1 = format!("0.0.0.0:{}", v4_port_1);
        let addr_2 = format!("[::]:{}", v6_port_1);
        let addr_3 = format!("0.0.0.0:{}", v4_port_2);
        let addr_4 = format!("[::]:{}", v6_port_2);

        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let handle1 = {
            let shutdown = shutdown_flag.clone();
            let conns = connections.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    get_p2p_udp_socket_with_shutdown(&addr_1, "V4".to_string(), shutdown, conns)
                        .await
                {
                    error!("{} UDP socket error: {}", addr_1, e);
                }
            })
        };

        let handle2 = {
            let shutdown = shutdown_flag.clone();
            let conns = connections.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    get_p2p_udp_socket_with_shutdown(&addr_2, "V6".to_string(), shutdown, conns)
                        .await
                {
                    error!("{} UDP socket error: {}", addr_2, e);
                }
            })
        };

        let handle3 = {
            let shutdown = shutdown_flag.clone();
            let conns = connections.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    get_p2p_udp_socket_with_shutdown(&addr_3, "V4".to_string(), shutdown, conns)
                        .await
                {
                    error!("{} UDP socket error: {}", addr_3, e);
                }
            })
        };

        let handle4 = {
            let shutdown = shutdown_flag.clone();
            let conns = connections.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    get_p2p_udp_socket_with_shutdown(&addr_4, "V6".to_string(), shutdown, conns)
                        .await
                {
                    error!("{} UDP socket error: {}", addr_4, e);
                }
            })
        };

        if let Err(e) = signal::ctrl_c().await {
            error!("failed to register Ctrl+C handler: {}", e);
        }
        info!("received Ctrl+C signal, shutting down...");

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
    info!(
        "NAT discovery + client P2P request forwarding service started, listening on {}",
        address
    );

    let mut buf = [0u8; 1024];

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("received Ctrl+C signal, shutting down {} service...", address);
                return Ok(());
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)), if shutdown.load(Ordering::Relaxed) => {
                info!("received shutdown signal, shutting down {} service...", address);
                return Ok(());
            }
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((size, src)) => {
                        let client_ip = src.ip();
                        let client_port = src.port();

                        let udp_addr = format!("{}:{}", client_ip, client_port);
                        info!("received message from {}:{} size={}", client_ip, client_port, size);

                        let res = serde_json::from_slice::<UserAddressInfo>(&buf[..size]);
                        match res {
                           Ok(msg) => {
                              let conns = connections.clone();
                              let ip = ip_type.clone();
                              process_p2p_user_info(udp_addr, ip, msg, conns).await.unwrap_or_else(|err| {
                                error!("failed to process NAT discovery / P2P request forwarding: {}", err);
                            })
                           },
                           Err(e) => {
                            error!("failed to serialize NAT address info, source {}:{},{}", client_ip, client_port, e);
                            buf[..size].fill(0);
                            continue;
                           }
                        };
                        buf[..size].fill(0);
                    }
                    Err(e) => error!("recv error: {}", e),
                }
            }
        }
    }
}

pub async fn get_p2p_udp_socket(
    address: &str,
    ip_type: String,
    connections: Arc<DashMap<String, QuicConnection>>,
) -> anyhow::Result<()> {
    let socket = UdpSocket::bind(address).await?;
    info!(
        "NAT discovery + client P2P request forwarding service started, listening on {}",
        address
    );

    let mut buf = [0u8; 1024];

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("received Ctrl+C signal, shutting down {} service...", address);
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
                                error!("failed to process NAT discovery / P2P request forwarding: {}", err);
                            })
                           },
                           Err(e) => {
                            error!("failed to serialize NAT address info, source {}:{},{}", client_ip, client_port, e);
                            buf[..size].fill(0);
                            continue;
                           }
                        };
                        buf[..size].fill(0);
                    }
                    Err(e) => error!("recv error: {}", e),
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
    match verify_token(user_address_info.token.as_ref()) {
        Ok(claims) => {
            let uuid = claims.uuid;
            info!("received message from {}, user uuid: {}", udp_addr, uuid);
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
                info!("entered redis lock");
                let mut conn = get_redis_conn().await?;
                let result: String = conn.get(&lock_key).await?;
                let user_addr = result.split('_').skip(1).collect::<Vec<&str>>().join("");

                info!("self value: {:?}, value2: {:?}", user_addr, user_address_info);
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
                    info!("matching server and connection endpoint");
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
                                    "Both parties are behind symmetric NAT, connection not possible! Stopping processing {},{}",
                                    user_address_info.uuid, target_user_address_info.uuid
                                );
                                return Ok(());
                            }
                            (3, _) => {
                                user_address_info.is_server = true;
                                target_user_address_info.is_server = false;
                                info!(
                                    "Initiator {}, receiver {}",
                                    user_address_info.uuid, target_user_address_info.uuid
                                );
                            }
                            (4, 3) => {
                                target_user_address_info.is_server = true;
                                user_address_info.is_server = false;
                                info!(
                                    "Receiver {}, initiator {}",
                                    target_user_address_info.uuid, user_address_info.uuid
                                );
                            }
                            _ => {
                                error!("match is empty, skipping");
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
                    info!("P2P connection info forwarding completed");
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        "failed to get target user info: {}, waiting for target user to upload to redis",
                        e
                    );
                }
            }

            {
                info!("inserting user info to redis");
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
                    .unwrap_or_else(|e| error!("failed to add user connection info: {}", e));
            }
        }
        Err(e) => {
            error!("failed to get token: {}", e.backtrace());
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
