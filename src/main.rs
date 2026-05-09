#![deny(clippy::unwrap_used)]
use api::init_server;
use entity::tracing::init_tracing;
use quic_service::nat_ip::nat_udp_service::run_udp_server;
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

    // 1. 创建并启动 QUIC ChatNode
    let chat_node = start_server().await.expect("启动quic服务失败");

    // 2. NAT UDP 服务使用 connections
    let connections = chat_node.connections();
    run_udp_server(connections.clone()).await.expect("启动UDP服务器失败");

    // 3. HTTP API 服务 (内部会启动内网 QUIC 服务)
    init_server::start_server(connections)
        .await
        .unwrap_or_else(|err| error!("启动http服务失败 {}, 堆栈信息 {:?}", err, err.backtrace()));
}
