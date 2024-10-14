mod module;
mod common;
use module::*;
use common::*;
use log::{error, info, warn, LevelFilter};
use std::{error::Error, net::SocketAddr};
use std::time::Duration;
use fast_log::Config;
use lazy_static::lazy_static;
use quinn::{ClientConfig, Endpoint, ServerConfig};
use rbatis::rbatis::RBatis;
use rbdc_mysql::MysqlDriver;
use std::sync::{Arc, Mutex};
use redis::{Client, Connection, Commands, RedisResult, RedisError};

lazy_static! {
    pub static ref RB: Arc<Mutex<RBatis>> = {
        let rb = RBatis::new();
        Arc::new(Mutex::new(rb))
    };
    pub static ref REDIS_CONNECTION: Arc<Mutex<Connection>> = {
        let client = Client::open("redis://175.178.17.158:10279/").expect("Failed to connect to Redis");
        let mut con = client.get_connection().expect("Failed to get Redis connection");

        // 进行身份验证
        redis::cmd("AUTH")
            .arg("REDACTED_REDIS_PASSWORD") // 替换为你的实际密码
            .query::<()>(&mut con)
            .expect("Failed to authenticate with Redis");

        Arc::new(Mutex::new(con))
    };
}
// 主函数入口点，使用Tokio异步运行时
#[actix_web::main]
async fn main() {
    fast_log::init(Config::new().console().level(LevelFilter::Info).file("target/test.log").chan_len(Some(100000))).unwrap();

    let mut rb = RB.lock();
    rb.unwrap().link(MysqlDriver{},"mysql://rust_dev:REDACTED_DB_PASSWORD_REMOTE@175.178.17.158:10222/rust_dev").await.unwrap();

    // 定义服务器监听地址

    quic_utils::quic_server::init_server();
    // 定义服务器监听地址
    let addr = "127.0.0.1:4433".parse().unwrap();
    quic_utils::quic_client::run_client(addr).await.unwrap();

    tokio::time::sleep(Duration::from_secs(10)).await;

    quic_utils::quic_client::run_client(addr).await.unwrap();
    init_web::start_server().await.expect("初始化失败!");
    info!("运行结束!")
}



