use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::sync::Arc;

use actix_files::Files;
use actix_web::middleware::from_fn;
use actix_web::{App, HttpServer, middleware, web};
use deadpool_redis::Pool;
use entity::config_str::{USER_FILE_PUBLIC, USER_FILE_PUBLIC_DIR};
use entity::{init_global_config, init_redis, init_sql_pool, read_global_config};
use http_service;
use http_service::middleware::TraceIdMiddleware;
use http_service::utils::record_bad_http::error_record_middleware;
use s3_service::client::GlobalS3Client;
use s3_service::config::S3Config;
use tracing::{error, info, warn};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, ec_private_keys, pkcs8_private_keys, rsa_private_keys};
use toml::Value;

use quic_service::internal_config::InternalQuicConfig;
use quic_service::internal_quic_server::run_internal_server;

use crate::controller::configure_api_routes;

async fn verify_redis(pool: &Pool) {
    match pool.get().await {
        Ok(mut conn) => {
            let result: Result<String, _> = deadpool_redis::redis::cmd("PING")
                .query_async(&mut conn)
                .await;
            match result {
                Ok(ref s) if s == "PONG" => {
                    info!("Redis 连接成功 (PING: {})", s);
                }
                Ok(s) => {
                    warn!("Redis PING 返回异常: {}", s);
                }
                Err(e) => {
                    error!("Redis 连接失败: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Redis 获取连接失败: {}", e);
        }
    }
}

fn init_cert_file() -> (Vec<Certificate>, PrivateKey) {
    // 加载证书
    let cert_file =
        &mut BufReader::new(File::open("./config/ssl/fullchain.pem").expect("找不到TLS证书"));
    let key_file =
        &mut BufReader::new(File::open("./config/ssl/privkey.pem").expect("找不到TLS证书密钥"));

    // 读取证书链
    let cert_chain = match certs(cert_file) {
        Ok(certs) => {
            info!("读取到 {} 个证书", certs.len());
            certs.into_iter().map(Certificate).collect()
        }
        Err(e) => {
            panic!("noTls: {}", e);
        }
    };

    // 尝试读取不同类型的私钥
    let mut keys = {
        // 读取RSA私钥
        key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
        if let Ok(keys) = rsa_private_keys(key_file) {
            if !keys.is_empty() {
                keys
            } else {
                // 读取EC私钥
                key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
                if let Ok(keys) = ec_private_keys(key_file) {
                    if !keys.is_empty() {
                        keys
                    } else {
                        // 读取PKCS8私钥
                        key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
                        pkcs8_private_keys(key_file).expect("无法读取私钥")
                    }
                } else {
                    // 读取PKCS8私钥
                    key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
                    pkcs8_private_keys(key_file).expect("无法读取私钥")
                }
            }
        } else {
            // 读取EC私钥
            key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
            if let Ok(keys) = ec_private_keys(key_file) {
                if !keys.is_empty() {
                    keys
                } else {
                    // 读取PKCS8私钥
                    key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
                    pkcs8_private_keys(key_file).expect("无法读取私钥")
                }
            } else {
                // 读取PKCS8私钥
                key_file.seek(SeekFrom::Start(0)).expect("无法重置文件读取位置");
                pkcs8_private_keys(key_file).expect("无法读取私钥")
            }
        }
    };

    if keys.is_empty() {
        panic!("私钥文件中没有找到有效的私钥");
    }

    let key = PrivateKey(keys.remove(0));
    (cert_chain, key)
}

/// 初始化S3客户端
async fn init_s3_client() -> Option<Arc<s3_service::S3Client>> {
    // 尝试读取S3配置
    let enabled = entity::config_manager::get_config("s3.enabled")
        .unwrap_or_else(|| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if !enabled {
        info!("S3存储未启用，使用本地存储");
        return None;
    }

    match S3Config::from_global_config() {
        Ok(config) => {
            info!("正在初始化S3客户端 - Provider: {}", config.provider);
            match GlobalS3Client::init(config).await {
                Ok(client) => {
                    info!("S3客户端初始化成功");
                    Some(client)
                }
                Err(e) => {
                    error!("S3客户端初始化失败: {}，将降级为本地存储", e);
                    None
                }
            }
        }
        Err(e) => {
            warn!("读取S3配置失败: {}，使用本地存储", e);
            None
        }
    }
}

/// 替换字符串中的环境变量占位符 ${VAR_NAME} 为实际环境变量值
fn substitute_env_vars(content: String) -> String {
    let mut result = content;
    // 简单替换 ${VAR_NAME} 格式的环境变量
    loop {
        if let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];
                let var_value = std::env::var(var_name).unwrap_or_default();
                result = result.replace(&format!("${{{}}}", var_name), &var_value);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result
}

pub use quic_service::ConnectionsMap;

///初始化服务
pub async fn start_server(connections: ConnectionsMap) -> anyhow::Result<()> {
    // 创建公开文件夹
    let pub_file_path = USER_FILE_PUBLIC_DIR;
    if !std::path::Path::new(pub_file_path).exists() {
        fs::create_dir_all(pub_file_path).expect("创建公开文件夹失败");
    }

    // 读取配置文件内容
    let config_content = fs::read_to_string("./config/app_config.toml").expect("无法读取配置文件");
    // 替换环境变量占位符
    let config_content = substitute_env_vars(config_content);
    // 解析配置文件内容
    let config_value: Value = config_content.parse()?;

    // 将解析后的配置转换为 HashMap
    let config_map: HashMap<String, Value> = config_value.try_into()?;
    
    // 初始化全局配置到 DashMap
    init_global_config!(&config_map);
    
    let url = read_global_config!("database", "url");

    let pool = init_sql_pool(&url).await?;

    let (cert_chain, key) = init_cert_file();

    // 配置 TLS
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| {
            error!("无法设置证书和私钥: {}", e);
            std::io::Error::other("无法设置证书和私钥")
        })?;

    let redis_url = read_global_config!("redis", "url");
    let redis_pool = init_redis(&redis_url)?;
    verify_redis(&redis_pool).await;

    // 启动内网 QUIC 服务 (Redis 已就绪)
    let internal_config = InternalQuicConfig::from_toml("./config/app_config.toml")?;
    let (_internal_shutdown_tx, internal_shutdown_rx) = tokio::sync::watch::channel(false);
    let internal_connections = connections.clone();
    tokio::spawn(async move {
        run_internal_server(internal_config, internal_connections, internal_shutdown_rx).await;
    });

    // 初始化S3客户端
    let s3_client = init_s3_client().await;

    let address = read_global_config!("server", "address");

    let s3_data = match s3_client {
        Some(client) => web::Data::new(client),
        None => {
            warn!("S3客户端未初始化，S3相关功能不可用");
            // 创建一个placeholder，不会实际使用
            let config = S3Config::default_minio();
            web::Data::new(Arc::new(s3_service::S3Client::new(config).await.unwrap_or_else(|_| {
                panic!("S3客户端创建失败")
            })))
        }
    };

    let conns = connections.clone();
    HttpServer::new(move || {
        App::new()
            .wrap(TraceIdMiddleware)
            .wrap(from_fn(error_record_middleware))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(pool.clone()))
            .app_data(s3_data.clone())
            .app_data(web::Data::new(conns.clone()))
            .wrap(middleware::Logger::default())
            .configure(http_service::http_service::configure_routes)
            .configure(configure_api_routes)
            .service(Files::new(USER_FILE_PUBLIC, USER_FILE_PUBLIC_DIR).show_files_listing())
    })
    .bind_rustls_021(address, config)? // 绑定到 HTTPS 端口
    // .bind(address)?
    .run()
    .await?;
    Ok(())
}
