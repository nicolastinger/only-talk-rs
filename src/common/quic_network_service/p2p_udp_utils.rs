use crate::utils::jwt_util::decode_jwt;
use crate::REDIS_CLIENT;
use anyhow::anyhow;
use deadpool_redis::redis::{cmd, RedisResult};
use log::{error, info};
use tokio::net::UdpSocket;
use tokio::signal;

/// p2p通信使用udp端口
pub async fn get_p2p_udp_socket() -> anyhow::Result<()> {
    // 绑定到所有网络接口的 9562 端口
    let socket = UdpSocket::bind("0.0.0.0:9562").await?;
    info!("udp服务端已启动，监听端口 9562...");

    let mut buf = [0u8; 1024];

    loop {
        tokio::select! {
                _ = signal::ctrl_c() => {
                    info!("收到 Ctrl+C 信号，正在关闭服务...");
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
                            let token = String::from_utf8_lossy(&buf[..size]);

                            match decode_jwt(token.as_ref()){
                                Ok(account) => {
                                    info!("收到来自 {}:{} 的消息, 用户uuid: {}",client_ip,client_port,account);
                                    // 用户的udp连接地址
                                    let key = format!("{}{}", "USER_UDP_ADDRESS_", account);
                                    let key = key.to_uppercase();
                                    let redis_client = REDIS_CLIENT.read().await;
                                    let redis_conn = redis_client.as_ref().ok_or(anyhow!("redis客户端错误"))?;
                                    let mut conn = redis_conn.get().await?;
                                    // 设置10分钟超时
                                    cmd("SET").arg(&key).arg(&udp_addr).arg("EX").arg(600).query_async(&mut conn).await.unwrap_or_else(|e| {
                                       error!("新增用户连接信息失败 {}",e.to_string())
                                    });
                                },
                                Err(e) => {
                                    error!("获取token失败 {}",e.backtrace());

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
