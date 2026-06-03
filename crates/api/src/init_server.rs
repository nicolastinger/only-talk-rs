use std::fs;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::sync::Arc;

use actix_files::Files;
use actix_web::middleware::from_fn;
use actix_web::{App, HttpServer, middleware, web};
use common::config_str::{USER_FILE_PUBLIC, USER_FILE_PUBLIC_DIR};
use common::{init_app_config, init_redis, init_sql_pool, read_global_config, verify_redis};
use http_service;
use http_service::middleware::TraceIdMiddleware;
use http_service::utils::record_bad_http::error_record_middleware;
use s3_service::client::GlobalS3Client;
use s3_service::config::S3Config;
use tracing::{error, info, warn};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, ec_private_keys, pkcs8_private_keys, rsa_private_keys};
use crate::controller::configure_api_routes;

fn read_key_file(path: &str, label: &str) -> anyhow::Result<File> {
    File::open(path).map_err(|e| anyhow::anyhow!("找不到{}: {}", label, e))
}

fn reset_file(file: &mut impl Seek) -> anyhow::Result<()> {
    file.seek(SeekFrom::Start(0)).map(|_| ())?;
    Ok(())
}

fn init_cert_file() -> anyhow::Result<(Vec<Certificate>, PrivateKey)> {
    let cert_file = &mut BufReader::new(read_key_file("./config/ssl/fullchain.pem", "TLS证书")?);
    let key_file = &mut BufReader::new(read_key_file("./config/ssl/privkey.pem", "TLS证书密钥")?);

    let cert_chain = certs(cert_file)
        .map_err(|e| anyhow::anyhow!("读取证书链失败: {}", e))?
        .into_iter()
        .map(Certificate)
        .collect::<Vec<_>>();
    info!("loaded {} certificates", cert_chain.len());

    // 尝试读取不同类型的私钥
    let mut keys = {
        reset_file(key_file)?;
        if let Ok(keys) = rsa_private_keys(key_file) {
            if !keys.is_empty() {
                keys
            } else {
                reset_file(key_file)?;
                if let Ok(keys) = ec_private_keys(key_file) {
                    if !keys.is_empty() {
                        keys
                    } else {
                        reset_file(key_file)?;
                        pkcs8_private_keys(key_file)
                            .map_err(|e| anyhow::anyhow!("无法读取PKCS8私钥: {}", e))?
                    }
                } else {
                    reset_file(key_file)?;
                    pkcs8_private_keys(key_file)
                        .map_err(|e| anyhow::anyhow!("无法读取PKCS8私钥: {}", e))?
                }
            }
        } else {
            reset_file(key_file)?;
            if let Ok(keys) = ec_private_keys(key_file) {
                if !keys.is_empty() {
                    keys
                } else {
                    reset_file(key_file)?;
                    pkcs8_private_keys(key_file)
                        .map_err(|e| anyhow::anyhow!("无法读取PKCS8私钥: {}", e))?
                }
            } else {
                reset_file(key_file)?;
                pkcs8_private_keys(key_file)
                    .map_err(|e| anyhow::anyhow!("无法读取PKCS8私钥: {}", e))?
            }
        }
    };

    if keys.is_empty() {
        return Err(anyhow::anyhow!("私钥文件中没有找到有效的私钥"));
    }

    let key = PrivateKey(keys.remove(0));
    Ok((cert_chain, key))
}

/// 初始化S3客户端
async fn init_s3_client() -> Option<Arc<s3_service::S3Client>> {
    // 尝试读取S3配置
    let enabled = common::config_manager::get_config("s3.enabled")
        .unwrap_or_else(|| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if !enabled {
        info!("S3 storage not enabled, using local storage");
        return None;
    }

    match S3Config::from_global_config() {
        Ok(config) => {
            info!("initializing S3 client - Provider: {}", config.provider);
            match GlobalS3Client::init(config).await {
                Ok(client) => {
                    info!("S3 client initialized successfully");
                    Some(client)
                }
                Err(e) => {
                    error!("S3 client initialization failed: {}, falling back to local storage", e);
                    None
                }
            }
        }
        Err(e) => {
            warn!("failed to read S3 config: {}, using local storage", e);
            None
        }
    }
}

///初始化服务
pub async fn start_server() -> anyhow::Result<()> {
    // 创建公开文件夹
    let pub_file_path = USER_FILE_PUBLIC_DIR;
    if !std::path::Path::new(pub_file_path).exists() {
        fs::create_dir_all(pub_file_path)
            .map_err(|e| anyhow::anyhow!("创建公开文件夹失败: {}", e))?;
    }

    init_app_config()?;

    let url = read_global_config!("database", "url");

    let pool = init_sql_pool(&url).await?;

    let (cert_chain, key) = init_cert_file()?;

    // 配置 TLS
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| {
            error!("failed to set certificate and private key: {}", e);
            std::io::Error::other("无法设置证书和私钥")
        })?;

    let redis_url = read_global_config!("redis", "url");
    let redis_pool = init_redis(&redis_url)?;
    verify_redis(&redis_pool).await;

    // 初始化S3客户端
    let s3_client = init_s3_client().await;

    let address = read_global_config!("server", "address");

    let s3_data = match s3_client {
        Some(client) => web::Data::new(client),
        None => {
            warn!("S3 client not initialized, S3-related features unavailable");
            // 创建一个placeholder，不会实际使用
            let config = S3Config::default_minio();
            web::Data::new(Arc::new(s3_service::S3Client::new(config).await?))
        }
    };

    HttpServer::new(move || {
        App::new()
            .wrap(TraceIdMiddleware)
            .wrap(from_fn(error_record_middleware))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(pool.clone()))
            .app_data(s3_data.clone())
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
