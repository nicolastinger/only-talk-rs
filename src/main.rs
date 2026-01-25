#![deny(clippy::unwrap_used)]
use api::init_server;
use fast_log::Config;
use log::{error, LevelFilter};
use port_service::p2p_service::p2p_udp_service::run_udp_server;
use quic_service::init_server::start_server;

/// 主函数入口点，使用Tokio异步运行时
#[actix_web::main]
async fn main() {
    fast_log::init(
        Config::new().console().level(LevelFilter::Info).file("log/rust_im.log").chan_len(Some(10)),
    )
    .expect("初始化日志失败");

    // 运行UDP服务器
    run_udp_server().await.expect("启动UDP服务器失败");
    // 启动quic服务
    start_server().await.expect("启动quic服务失败");
    // 启动HTTP服务器并等待其完成
    init_server::start_server()
        .await
        .unwrap_or_else(|err| error!("启动http服务失败 {}, 堆栈信息 {:?}", err, err.backtrace()));
}
