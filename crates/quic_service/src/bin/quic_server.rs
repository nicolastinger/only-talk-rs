//! QUIC 服务独立启动入口
//!
//! 仅启动 QUIC ChatNode + NAT UDP + 内网 QUIC 服务，不启动 HTTP。

use common::tracing::init_tracing;
use common::fatal_panic_async;
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
    info!("starting QUIC service (standalone mode)");

    let chat_node: Arc<ChatNode> = match start_server().await {
        Ok(node) => node,
        Err(e) => fatal_panic_async(&format!("failed to start QUIC service: {:?}", e)).await,
    };

    info!("QUIC service ready, press Ctrl+C to stop");

    // 等待退出信号
    tokio::signal::ctrl_c().await.unwrap_or_else(|e| {
        error!("failed to register Ctrl+C handler: {}", e);
    });

    info!("received shutdown signal, shutting down gracefully...");
    if let Err(e) = chat_node.stop().await {
        error!("failed to shutdown QUIC service: {:?}", e);
    }
    info!("QUIC service stopped");
}
