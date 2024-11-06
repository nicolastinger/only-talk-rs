mod module;
mod common;
mod marcos;
mod utils;

use module::*;
use common::*;
use quic_network_service::quic_connection::QuicConnection;
use log::{error, info, warn, LevelFilter};
use std::{error::Error, net::SocketAddr};
use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use fast_log::Config;
use lazy_static::lazy_static;
use tokio::sync::RwLock as TokioRwLock;
use quic_network_service::quic_client;
// 创建一个quic服务器维护列表全局变量，使用 RwLock 包装
// 使用 lazy_static 初始化全局共享变量
lazy_static! {
    pub static ref GLOBAL_QUIC_SERVER_LIST: Arc<TokioRwLock<HashMap<String, QuicConnection>>> = Arc::new(TokioRwLock::new(HashMap::new()));
}
static QUIC_MSG_SPLIT: &str = "#$#";   //quic服务解析间隔符
// 主函数入口点，使用Tokio异步运行时
#[actix_web::main]
async fn main() {
    fast_log::init(Config::new().console().level(LevelFilter::Info).file("log/rust_im.log").chan_len(Some(10))).unwrap();

    //quic_network_service::quic_server::init_server();
    // 定义服务器监听地址
    //let addr = "175.178.17.158:4433".parse().unwrap();
    let addr = "127.0.0.1:4433".parse().unwrap();
    tokio::spawn(async move{
        quic_client::run_client(addr).await;
    });

    init_web::start_server().await.expect("初始化失败!");
    info!("运行结束!")
}



