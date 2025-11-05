use crc::Crc;
use deadpool_redis::Pool as RedisPool;
use fast_log::Config;
use api::init_server;
use lazy_static::lazy_static;
use log::{error, LevelFilter};
use port_service::p2p_service::p2p_udp_service::run_udp_server;
use quic_service::init_server::start_server;
use quic_service::models::quic_connection::QuicConnection;
use rbatis::RBatis;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;

use port_service as p2p_service;
use quic_service;

/// 主函数入口点，使用Tokio异步运行时
#[actix_web::main]
async fn main() {
    fast_log::init(
        Config::new()
            .console()
            .level(LevelFilter::Info)
            .file("log/rust_im.log")
            .chan_len(Some(10)),
    )
    .unwrap();

    // 运行UDP服务器
    run_udp_server().await.expect("启动UDP服务器失败");
    // 启动quic服务
    start_server().await.expect("启动quic服务失败");
    // 启动HTTP服务器并等待其完成
    init_server::start_server()
        .await
        .unwrap_or_else(|err| error!("启动http服务失败 {}, 堆栈信息 {:?}", err, err.backtrace()));
}