//! HTTP API 服务独立启动入口
//!
//! 仅启动 actix-web HTTPS 服务，不启动 QUIC 相关服务。

use api::init_server::start_server;
use common::tracing::init_tracing;
use common::fatal_panic_async;
use tracing::{error, info};

#[actix_web::main]
async fn main() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("加载 .env 文件失败: {}", e);
    }

    let _guard = init_tracing();
    info!("启动 HTTP API 服务（独立模式）");

    if let Err(e) = start_server().await {
        error!("启动http服务失败 {}, 堆栈信息 {:?}", e, e.backtrace());
        fatal_panic_async(&format!("启动http服务失败: {:?}", e)).await;
    }
}
