mod common;
mod http_service;
mod marcos;
pub(crate) mod p2p_service;
pub(crate) mod quic_service;
mod utils;

use crate::quic_service::init_server::start_server;
use crc::Crc;
use deadpool_redis::Pool as RedisPool;
use fast_log::Config;
use http_service::init_server;
use lazy_static::lazy_static;
use log::{error, LevelFilter};
use p2p_service::p2p_udp_service::run_udp_server;
use quic_service::models::quic_connection::QuicConnection;
use rbatis::RBatis;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;

rust_i18n::i18n!("locales");
// 创建一个quic服务器维护列表全局变量，使用 RwLock 包装，后期采用dashMap
// 使用 lazy_static 初始化全局共享变量
lazy_static! {
    pub static ref GLOBAL_QUIC_SERVER_LIST: Arc<TokioRwLock<HashMap<String, QuicConnection>>> =
        Arc::new(TokioRwLock::new(HashMap::new()));
    pub static ref REDIS_CLIENT: Arc<TokioRwLock<Option<RedisPool>>> =
        Arc::new(TokioRwLock::new(None));
    pub static ref RBATIS_DATABASE: Arc<TokioRwLock<Option<RBatis>>> =
        Arc::new(TokioRwLock::new(None));
}

// 创建CRC-16/X25计算器
const X25: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

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
