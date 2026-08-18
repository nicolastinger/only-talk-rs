//! HTTP API 服务独立入口
//!
//! 仅启动 actix-web HTTPS 服务,不启动 QUIC 服务。

use api::init_server::start_server;
use common::fatal_panic_async;
use common::tracing::init_tracing;
use tracing::{error, info};

#[actix_web::main]
async fn main() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("加载 .env 文件失败: {}", e);
    }

    let _guard = init_tracing();
    info!("starting HTTP API service (standalone mode)");

    if let Err(e) = start_server().await {
        error!("failed to start HTTP service {}, backtrace {:?}", e, e.backtrace());
        fatal_panic_async(&format!("failed to start HTTP service: {:?}", e)).await;
    }
}
