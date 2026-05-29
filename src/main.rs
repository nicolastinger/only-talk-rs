#![deny(clippy::unwrap_used)]
use api::init_server;
use common::tracing::init_tracing;
use common::fatal_panic_async;
use quic_service::init_server::start_server;
use tracing::{debug, error, info};

/// 主函数入口点，使用Tokio异步运行时
#[actix_web::main]
async fn main() {
    // 加载 .env 文件
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("加载 .env 文件失败: {}", e);
    }

    let _guard = init_tracing();

    debug!("日志级别为debug");
    info!("启动应用");

    // 1. 启动 QUIC 服务（ChatNode + NAT UDP + 内网 QUIC，完全自包含）
    let _chat_node = match start_server().await {
        Ok(node) => node,
        Err(e) => fatal_panic_async(&format!("启动quic服务失败: {:?}", e)).await,
    };

    // 2. 启动 HTTP API 服务
    if let Err(e) = init_server::start_server().await {
        error!("启动http服务失败 {}, 堆栈信息 {:?}", e, e.backtrace());
        fatal_panic_async(&format!("启动http服务失败: {:?}", e)).await;
    }
}
