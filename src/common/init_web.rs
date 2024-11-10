use std::env;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::sync::Arc;
use actix_web::{get, middleware, post, web, App, HttpResponse, HttpServer, Responder};
use fast_log::Config;
use fast_log::consts::LogSize;
use fast_log::plugin::file_split::RollingType;
use fast_log::plugin::packer::LogPacker;
use log::{error, info, LevelFilter};
use deadpool_redis::{Pool, PoolError,Config as dp_config, Runtime};
use deadpool_redis::redis::{cmd, RedisResult};
use deadpool_redis::redis::ExpireOption::NONE;
use rbatis::RBatis;
use rbdc_mysql::MysqlDriver;
use redis::RedisError;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls::server::NoClientAuth;
use rustls_pemfile::{certs, ec_private_keys, rsa_private_keys, pkcs8_private_keys};
use crate::common::quic_network_service;
use crate::module;

pub(crate) struct AppState {
    pub(crate) redis_pool: Arc<Pool>,
}

fn init_redis() -> Pool {
    // 创建 Redis 连接池
    let mut config = dp_config::from_url("redis://:REDACTED_REDIS_PASSWORD@175.178.17.158:10279/");
    let pool = config.create_pool(Some(Runtime::Tokio1)).expect("Failed to create Redis pool");

    pool
}

fn init_cert_file() -> (Vec<Certificate>,PrivateKey) {
    // 加载证书
    let cert_file = &mut BufReader::new(File::open("config/TLS/onlytalk.cn.pem").expect("找不到TLS证书"));
    let key_file = &mut BufReader::new(File::open("config/TLS/onlytalk.cn.key").expect("找不到TLS证书密钥"));

    // 读取证书链
    let cert_chain = match certs(cert_file) {
        Ok(certs) => {
            info!("读取到 {} 个证书", certs.len());
            certs.into_iter().map(Certificate).collect()
        },
        Err(e) => {
            panic!("无法读取证书文件: {}", e);
        }
    };

    // 读取私钥
    let mut key_content = String::new();
    key_file.read_to_string(&mut key_content).unwrap();

    key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");

    // 从.key文件加载私钥
    let mut key_file = BufReader::new(File::open("config/TLS/onlytalk.cn.key").expect("打开key文件失败"));
    let mut keys = rsa_private_keys(&mut key_file).or_else(|_| ec_private_keys(&mut key_file))
        .map_err(|_| "无法解析私钥文件").unwrap();

    if keys.is_empty() {
        panic!("私钥文件中没有找到有效的私钥");
    }

    let key = PrivateKey(keys.remove(0));
    (cert_chain, key)
}

async fn init_sql_pool() -> RBatis {
    let rb = RBatis::new();
    // 创建连接池
    let database_url = "mysql://rust_dev:REDACTED_DB_PASSWORD_REMOTE@175.178.17.158:10222/rust_dev";
    rb.init(MysqlDriver{},database_url).unwrap();
    rb
}

//初始化异步web容器
pub async fn start_server() -> std::io::Result<()> {
    let pool = init_sql_pool().await;

    let (cert_chain, key) = init_cert_file();

    // 配置 TLS
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| {
            eprintln!("无法设置证书和私钥: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "无法设置证书和私钥")
        })?;

    let redis_pool = init_redis();
    quic_network_service::quic_server::init_server(redis_pool.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(init_redis().clone()))
            .app_data(web::Data::new(pool.clone()))
            // 设置中间件，让actix-web打印日志
            .wrap(middleware::Logger::default())
            .route("/", web::get().to(home))
            .configure(module::configure_routes)
        // 这里可以继续添加其他路由
    })
        .bind_rustls_021("127.0.0.1:8443",config)? // 绑定到 HTTPS 端口
        .run()
        .await
}


// 假设有一个结构体来表示用户信息，这里简化处理，直接返回一个静态字符串
pub async fn home() -> String {
    info!("Home");
    "hello,world!".to_string()
}