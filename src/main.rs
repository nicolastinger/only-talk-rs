#![deny(clippy::unwrap_used)]
use api::init_server;
use common::fatal_panic_async;
use common::tracing::init_tracing;
use quic_service::init_server::start_server;
use tracing::{debug, error, info};

/// 主入口点，使用 Tokio 异步运行时
#[actix_web::main]
async fn main() {
    // 加载 .env 文件
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("加载 .env 文件失败: {}", e);
    }

    let _guard = init_tracing();

    debug!("日志级别已设置为 debug");
    info!("应用启动中");

    // 1. Start QUIC service (ChatNode + NAT UDP + internal QUIC, fully self-contained)
    let _chat_node = match start_server().await {
        Ok(node) => node,
        Err(e) => fatal_panic_async(&format!("QUIC 服务启动失败: {:?}", e)).await,
    };

    // 2. Start HTTP API service
    if let Err(e) = init_server::start_server().await {
        error!("HTTP 服务启动失败 {}, backtrace {:?}", e, e.backtrace());
        fatal_panic_async(&format!("HTTP 服务启动失败: {:?}", e)).await;
    }
}
