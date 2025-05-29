use tokio::net::UdpSocket;
use log::{error, info};
use tokio::signal;

/// p2p通信使用udp端口
pub async fn get_p2p_udp_socket() -> anyhow::Result<()> {
    // 绑定到所有网络接口的 9562 端口[6](@ref)
    let socket = UdpSocket::bind("0.0.0.0:9562").await?;
    info!("服务端已启动，监听端口 9562...");

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

                        // 转换消息为字符串（自动处理非 UTF-8 字符）[2](@ref)
                        let message = String::from_utf8_lossy(&buf[..size]);

                        // 格式化输出客户端信息[6](@ref)
                        info!("收到来自 {}:{} 的消息：{}",
                                 client_ip,
                                 client_port,
                                 message);

                        // 清空缓冲区（避免残留数据）
                        buf[..size].fill(0);
                    }
                    Err(e) => error!("接收错误: {}", e),
                }
            }
        }
    }
}