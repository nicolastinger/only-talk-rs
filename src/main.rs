mod module;
mod common;
mod marcos;
mod utils;

use common::*;
use common::quic_network_service::models::quic_connection::QuicConnection;
use log::{error, info, warn, LevelFilter};
use std::{error::Error, net::SocketAddr};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crc::{Crc};
use crc_catalog::Algorithm;
use fast_log::Config;
use lazy_static::lazy_static;
use tokio::sync::RwLock as TokioRwLock;
use quic_network_service::quic_client;
rust_i18n::i18n!("locales");
// 创建一个quic服务器维护列表全局变量，使用 RwLock 包装，后期采用dashMap
// 使用 lazy_static 初始化全局共享变量
lazy_static! {
    pub static ref GLOBAL_QUIC_SERVER_LIST: Arc<TokioRwLock<HashMap<String, QuicConnection>>> = Arc::new(TokioRwLock::new(HashMap::new()));
}

// 创建CRC-16/X25计算器
const X25: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

/// 主函数入口点，使用Tokio异步运行时
#[actix_web::main]
async fn main() {
    fast_log::init(Config::new().console().level(LevelFilter::Info).file("log/rust_im.log").chan_len(Some(10))).unwrap();

    //let addr = "124.220.82.185:4433".parse().unwrap();
    let addr = "127.0.0.1:4433".parse().unwrap();
     tokio::spawn(async move{
         quic_client::run_client(addr).await;
     });

    init_server::start_server().await.unwrap_or_else(|err| error!("错误信息 {}, 堆栈信息 {:?}", err, err.backtrace()));
}



