//! QUIC 服务独立入口
//!
//! 仅启动 QUIC ChatNode + NAT UDP + 内部 QUIC 服务，不包含 HTTP。

use std::sync::Arc;

use common::fatal_panic_async;
use common::tracing::init_tracing;
use quic_service::init_server::start_server;
use quic_service::{ChatNode, ServiceLifecycle};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("加载 .env 文件失败: {}", e);
    }

    let _guard = init_tracing();
    info!("正在启动 QUIC 服务（独立模式）");

    let chat_node: Arc<ChatNode> = match start_server().await {
        Ok(node) => node,
        Err(e) => fatal_panic_async(&format!("QUIC 服务启动失败: {:?}", e)).await,
    };

    info!("QUIC 服务已就绪，按 Ctrl+C 停止");

    // 等待退出信号
    tokio::signal::ctrl_c().await.unwrap_or_else(|e| {
        error!("注册 Ctrl+C 处理器失败: {}", e);
    });

    info!("收到关闭信号，正在优雅关闭...");
    if let Err(e) = chat_node.stop().await {
        error!("QUIC 服务关闭失败: {:?}", e);
    }
    info!("QUIC 服务已停止");
}
