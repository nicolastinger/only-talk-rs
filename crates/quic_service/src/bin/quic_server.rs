//! QUIC 服务独立启动入口
//!
//! 仅启动 QUIC ChatNode + NAT UDP + 内网 QUIC 服务，不启动 HTTP。

use common::tracing::init_tracing;
use quic_service::init_server::start_server;
use quic_service::{ChatNode, ServiceLifecycle};
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("加载 .env 文件失败: {}", e);
    }

    let _guard = init_tracing();
    info!("启动 QUIC 服务（独立模式）");

    let chat_node: Arc<ChatNode> = start_server().await.expect("启动quic服务失败");

    info!("QUIC 服务已就绪，按 Ctrl+C 停止");

    // 等待退出信号
    tokio::signal::ctrl_c().await.expect("无法注册 Ctrl+C 处理器");

    info!("收到退出信号，正在优雅关闭...");
    if let Err(e) = chat_node.stop().await {
        error!("关闭 QUIC 服务失败: {:?}", e);
    }
    info!("QUIC 服务已停止");
}
