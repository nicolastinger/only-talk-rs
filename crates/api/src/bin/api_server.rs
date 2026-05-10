//! HTTP API 服务独立启动入口
//!
//! 仅启动 actix-web HTTPS 服务，不启动 QUIC 相关服务。

use api::init_server::start_server;
use entity::tracing::init_tracing;
use tracing::{error, info};

#[actix_web::main]
async fn main() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("加载 .env 文件失败: {}", e);
    }

    let _guard = init_tracing();
    info!("启动 HTTP API 服务（独立模式）");

    start_server()
        .await
        .unwrap_or_else(|err| error!("启动http服务失败 {}, 堆栈信息 {:?}", err, err.backtrace()));
}
