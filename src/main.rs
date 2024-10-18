mod module;
mod common;
use module::*;
use common::*;
use log::{error, info, warn, LevelFilter};
use std::{error::Error, net::SocketAddr};
use std::time::Duration;
use fast_log::Config;

// 主函数入口点，使用Tokio异步运行时
#[actix_web::main]
async fn main() {
    fast_log::init(Config::new().console().level(LevelFilter::Info).file("target/test.log").chan_len(Some(100000))).unwrap();

    quic_utils::quic_server::init_server();
    // 定义服务器监听地址
    let addr = "127.0.0.1:4433".parse().unwrap();

    quic_utils::quic_client::run_client(addr).await.unwrap();
    init_web::start_server().await.expect("初始化失败!");
    info!("运行结束!")
}



