use crate::p2p_service::model::{P2pInitMsg, UserAddressInfo};
use crate::quic_service::models::quic_connection::ConnectionType;
use crate::quic_service::models::text_msg::MessageType;
use crate::quic_service::msg_service::get_send_stream_by_uuid;
use crate::quic_service::msg_service::text_msg_service::generate_text_msg;
use crate::utils::global_static_str::SYSTEM;
use crate::utils::jwt_util::decode_jwt;
use crate::utils::message_types;
use crate::utils::redis_utils::{acquire_lock, get_redis_conn, release_lock};
use crate::{GLOBAL_QUIC_SERVER_LIST, REDIS_CLIENT};
use anyhow::anyhow;
use deadpool_redis::redis::{cmd, AsyncCommands, RedisResult};
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::signal;

pub async fn run_udp_server() -> Result<(), anyhow::Error> {
    tokio::spawn(async {
        let addr_1 = "0.0.0.0:9562";
        let addr_2 = "[::]:9563";
        let addr_3 = "0.0.0.0:9564";
        let addr_4 = "[::]:9565";

        // 创建一个共享的退出标志
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        // 启动udp连接1
        let handle1 = {
            let shutdown = shutdown_flag.clone();
            tokio::spawn(async move {
                get_p2p_udp_socket_with_shutdown(addr_1, "V4".to_string(), shutdown)
                    .await
                    .expect("9562 Failed to get UDP socket");
            })
        };

        let handle2 = {
            let shutdown = shutdown_flag.clone();
            tokio::spawn(async move {
                get_p2p_udp_socket_with_shutdown(addr_2, "V6".to_string(), shutdown)
                    .await
                    .expect("9563 Failed to get UDP socket");
            })
        };

        let handle3 = {
            let shutdown = shutdown_flag.clone();
            tokio::spawn(async move {
                get_p2p_udp_socket_with_shutdown(addr_3, "V4".to_string(), shutdown)
                    .await
                    .expect("9564 to get UDP socket");
            })
        };

        let handle4 = {
            let shutdown = shutdown_flag.clone();
            tokio::spawn(async move {
                get_p2p_udp_socket_with_shutdown(addr_4, "V6".to_string(), shutdown)
                    .await
                    .expect("9565 to get UDP socket");
            })
        };

        // 等待 Ctrl+C 信号
        signal::ctrl_c().await.expect("无法注册 Ctrl+C 处理器");
        info!("收到 Ctrl+C 信号，正在关闭服务...");

        // 设置退出标志
        shutdown_flag.store(true, Ordering::Relaxed);

        // 等待所有任务完成
        let _ = tokio::join!(handle1, handle2, handle3, handle4);
    });
    Ok(())
}

/// p2p通信使用udp端口
pub async fn get_p2p_udp_socket_with_shutdown(
    address: &str,
    ip_type: String,
    shutdown: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    // 绑定到所有网络接口的 udp 端口
    let socket = UdpSocket::bind(address).await?;
    info!("udp服务端已启动，监听地址 {}", address);

    let mut buf = [0u8; 1024];

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("收到 Ctrl+C 信号，正在关闭 {} 服务...", address);
                return Ok(());
            }
            // 检查共享的退出标志
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)), if shutdown.load(Ordering::Relaxed) => {
                info!("收到退出信号，正在关闭 {} 服务...", address);
                return Ok(());
            }
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((size, src)) => {
                        // 提取客户端信息
                        let client_ip = src.ip();
                        let client_port = src.port();

                        let udp_addr = format!("{}:{}", client_ip, client_port);

                        // 转换消息为字符串（自动处理非 UTF-8 字符）[2](@ref)
                        //let user_address_info = String::from_utf8_lossy(&buf[..size]);

                        match serde_json::from_slice::<UserAddressInfo>(&buf[..size]){
                           Ok(msg) => {
                              process_p2p_user_info(udp_addr, ip_type.clone(), msg).await.unwrap_or_else(|err| {
                                error!("处理用户p2p连接失败 {}", err);
                            })
                           },
                           Err(e) => {
                            error!("序列化p2p信息失败，来源{}:{},{}", client_ip, client_port, e.to_string());
                            buf[..size].fill(0);
                            continue;
                           }
                        };
                        // 清空缓冲区（避免残留数据）
                        buf[..size].fill(0);
                    }
                    Err(e) => error!("接收错误: {}", e),
                }
            }
        }
    }
}

/// p2p通信使用udp端口
pub async fn get_p2p_udp_socket(address: &str, ip_type: String) -> anyhow::Result<()> {
    // 绑定到所有网络接口的 udp 端口
    let socket = UdpSocket::bind(address).await?;
    info!("udp服务端已启动，监听地址 {}", address);

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
                        // 提取客户端信息
                        let client_ip = src.ip();
                        let client_port = src.port();

                        let udp_addr = format!("{}:{}", client_ip, client_port);

                        // 转换消息为字符串（自动处理非 UTF-8 字符）[2](@ref)
                        //let user_address_info = String::from_utf8_lossy(&buf[..size]);

                        match serde_json::from_slice::<UserAddressInfo>(&buf[..size]){
                           Ok(msg) => {
                              process_p2p_user_info(udp_addr, ip_type.clone(), msg).await.unwrap_or_else(|err| {
                                error!("处理用户p2p连接失败 {}", err);
                            })
                           },
                           Err(e) => {
                            error!("序列化p2p信息失败，来源{}:{},{}", client_ip, client_port, e.to_string());
                            buf[..size].fill(0);
                            continue;
                           }
                        };
                        // 清空缓冲区（避免残留数据）
                        buf[..size].fill(0);
                    }
                    Err(e) => error!("接收错误: {}", e),
                }
            }
        }
    }
}

/// 处理用户p2p连接
// 给对应用户id的redis加锁，成功则新增数据写入，失败则读取数据并对本次的
async fn process_p2p_user_info(
    udp_addr: String,
    ip_type: String,
    mut user_address_info: UserAddressInfo,
) -> Result<(), anyhow::Error> {
    match decode_jwt(user_address_info.token.as_ref()) {
        Ok(uuid) => {
            info!("收到来自 {} 的消息, 用户uuid: {}", udp_addr, uuid);
            // 用户的udp连接地址
            let key = format!("{}{}_{}", "USER_UDP_ADDRESS_", ip_type, uuid);
            let lock_key = format!("{}{}_{}", "USER_UDP_ADDRESS_LOCK_", ip_type, uuid);
            let lock_key = lock_key.to_uppercase();
            let key = key.to_uppercase();
            let mut acquire_flag = false;
            user_address_info.address = udp_addr;

            // 针对这个key加30秒过期的redis锁
            {
                let mut conn = get_redis_conn().await?;
                let lock_id =
                    acquire_lock(&mut conn, &lock_key, 30, user_address_info.address.clone())
                        .await?;
                if lock_id.is_some() {
                    acquire_flag = true;
                    user_address_info.lock_uuid = lock_id.unwrap();
                }
            }

            // 加锁失败，代表之前有录入过消息，对比两次端口得出是否对称型nat
            if !acquire_flag {
                info!("进入redis锁");
                let mut conn = get_redis_conn().await?;
                let result: String = conn.get(&lock_key).await?;
                let user_addr = result.split('_').skip(1).collect::<Vec<&str>>().join("");

                info!("获取自身值为 {:?},值2为 {:?}", user_addr, user_address_info);
                if user_addr == user_address_info.address {
                    user_address_info.nat_type = 3; //ip端口限制型
                } else {
                    user_address_info.nat_type = 4; //对称型
                }
                let mut conn = get_redis_conn().await?;
                // 手动释放锁
                release_lock(&mut conn, &lock_key, &result).await?;
                // 标记为已经释放锁了，可以给目标用户使用
                user_address_info.is_lock = true;
                // 避免传输用户token敏感信息
                user_address_info.token = "".to_string();
            }

            // 获取目标用户的ip地址
            match get_target_user_address_info(&user_address_info.target_uuid, &ip_type).await {
                Ok(mut target_user_address_info) => {
                    info!("进入到服务器和连接端匹配");
                    // 只有当检测完NAT类型后再进行比较
                    if target_user_address_info.is_lock == true && !acquire_flag {
                        {
                            let mut conn = get_redis_conn().await?;
                            // 删除自身
                            conn.del::<_, ()>(&key).await?;
                            // 删除对方
                            let target_key = format!(
                                "{}{}_{}",
                                "USER_UDP_ADDRESS_", ip_type, target_user_address_info.uuid
                            );
                            let target_key = target_key.to_uppercase();
                            conn.del::<_, ()>(&target_key).await?;
                        }
                        match (
                            user_address_info.nat_type,
                            target_user_address_info.nat_type,
                        ) {
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
                            let mut result = message_types::MSG_TYPE_P2P_USER_CLIENT;
                            // 本用户是服务端，向对方发送客户端信息
                            if user_address_info.is_server {
                                result = message_types::MSG_TYPE_P2P_USER_CLIENT
                            } else {
                                result = message_types::MSG_TYPE_P2P_USER_SERVER
                            }
                            result
                        };
                        {
                            // 获取目标方发送流
                            let my_send_stream = get_send_stream_by_uuid(
                                &target_user_address_info.uuid,
                                &ConnectionType::Text.to_string(),
                            )
                            .await?;

                            let user_address_info_vec = serde_json::to_vec(&user_address_info)?;
                            let msg_raw = generate_text_msg(
                                server,
                                user_address_info_vec,
                                target_user_address_info.uuid.clone(),
                                SYSTEM.to_string(),
                            )?;
                            my_send_stream.write().await.write_all(&msg_raw).await?;
                        }

                        {
                            let my_send_stream = get_send_stream_by_uuid(
                                &user_address_info.uuid,
                                &ConnectionType::Text.to_string(),
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
                            my_send_stream.write().await.write_all(&msg_raw).await?;
                        }
                    }
                    info!("转发建立p2p信息完成");
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        "获取目标用户信息失败 {}，等待目标用户上传redis",
                        e.to_string()
                    );
                }
            }

            {
                info!("插入用户信息到redis中");
                let mut conn = get_redis_conn().await?;
                let user_address_info_json =
                    serde_json::to_string(&user_address_info).unwrap_or(String::new());
                // 设置1分钟超时
                let _: () = cmd("SET")
                    .arg(&key)
                    .arg(&user_address_info_json)
                    .arg("EX")
                    .arg(60)
                    .query_async(&mut conn)
                    .await
                    .unwrap_or_else(|e| error!("新增用户连接信息失败 {}", e.to_string()));
            }
        }
        Err(e) => {
            error!("获取token失败 {}", e.backtrace());
        }
    };

    Ok(())
}

/// 获取目标用户地址
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
