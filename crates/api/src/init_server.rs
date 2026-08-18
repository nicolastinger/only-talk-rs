use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::sync::Arc;

use actix_web::middleware::from_fn;
use actix_web::{App, HttpServer, middleware, web};
use common::{init_app_config, init_redis, init_sql_pool, read_global_config, verify_redis};
use email_service::config::{AliyunConfig, EmailServiceConfig, ProviderConfig};
use http_service;
use http_service::middleware::TraceIdMiddleware;
use http_service::utils::record_bad_http::error_record_middleware;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, ec_private_keys, pkcs8_private_keys, rsa_private_keys};
use s3_service::client::GlobalS3Client;
use s3_service::config::S3Config;
use tracing::{error, info};

use crate::controller::configure_api_routes;

fn read_key_file(path: &str, label: &str) -> anyhow::Result<File> {
    File::open(path).map_err(|e| anyhow::anyhow!("{} not found: {}", label, e))
}

fn reset_file(file: &mut impl Seek) -> anyhow::Result<()> {
    file.seek(SeekFrom::Start(0)).map(|_| ())?;
    Ok(())
}

fn init_cert_file() -> anyhow::Result<(Vec<Certificate>, PrivateKey)> {
    let cert_file =
        &mut BufReader::new(read_key_file("./config/ssl/fullchain.pem", "TLS certificate")?);
    let key_file =
        &mut BufReader::new(read_key_file("./config/ssl/privkey.pem", "TLS certificate key")?);

    let cert_chain = certs(cert_file)
        .map_err(|e| anyhow::anyhow!("Failed to read certificate chain: {}", e))?
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
                        pkcs8_private_keys(key_file).map_err(|e| {
                            anyhow::anyhow!("Unable to read PKCS8 private key: {}", e)
                        })?
                    }
                } else {
                    reset_file(key_file)?;
                    pkcs8_private_keys(key_file)
                        .map_err(|e| anyhow::anyhow!("Unable to read PKCS8 private key: {}", e))?
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
                        .map_err(|e| anyhow::anyhow!("Unable to read PKCS8 private key: {}", e))?
                }
            } else {
                reset_file(key_file)?;
                pkcs8_private_keys(key_file)
                    .map_err(|e| anyhow::anyhow!("Unable to read PKCS8 private key: {}", e))?
            }
        }
    };

    if keys.is_empty() {
        return Err(anyhow::anyhow!("No valid private key found in key file"));
    }

    let key = PrivateKey(keys.remove(0));
    Ok((cert_chain, key))
}

/// 初始化 S3 客户端。
///
/// S3 存储为必选项: 未启用或初始化失败时启动报错。
async fn init_s3_client() -> anyhow::Result<Arc<s3_service::S3Client>> {
    let enabled = common::config_manager::get_config("s3.enabled")
        .unwrap_or_else(|| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if !enabled {
        return Err(anyhow::anyhow!(
            "S3 storage is required but s3.enabled is false, refusing to start"
        ));
    }

    let config = S3Config::from_global_config()
        .map_err(|e| anyhow::anyhow!("failed to read S3 config: {}", e))?;

    info!("initializing S3 client - Provider: {}", config.provider);
    let client = GlobalS3Client::init(config)
        .await
        .map_err(|e| anyhow::anyhow!("S3 client initialization failed: {}", e))?;
    info!("S3 client initialized successfully");
    Ok(client)
}

/// 初始化邮件管理器,从配置加载阿里云提供商。
///
/// 配置中禁用邮件时创建空管理器(发送将失败)。
fn init_email_manager() -> anyhow::Result<Arc<email_service::EmailManager>> {
    let enabled = common::config_manager::get_config("email.enabled")
        .unwrap_or_else(|| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if !enabled {
        info!("email service not enabled, verification codes will not be sent");
        return Ok(Arc::new(email_service::EmailManager::new(EmailServiceConfig::default())?));
    }

    let aliyun = AliyunConfig {
        enabled: true,
        priority: 100,
        access_key_id: read_global_config!("email", "access_key_id"),
        access_key_secret: read_global_config!("email", "access_key_secret"),
        region_id: read_global_config!("email", "region_id"),
        account_name: read_global_config!("email", "account_name"),
        from_alias: Some("OnlyTalk".to_string()),
        ..Default::default()
    };
    let mut providers = HashMap::new();
    providers.insert("aliyun".to_string(), ProviderConfig::Aliyun(aliyun));

    let config = EmailServiceConfig {
        default_provider: Some("aliyun".to_string()),
        providers,
        ..Default::default()
    };
    info!("initializing email manager with aliyun provider");
    Ok(Arc::new(email_service::EmailManager::new(config)?))
}

/// 初始化服务
pub async fn start_server() -> anyhow::Result<()> {
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
            std::io::Error::other("Failed to set certificate and private key")
        })?;

    let redis_url = read_global_config!("redis", "url");
    let redis_pool = init_redis(&redis_url)?;
    verify_redis(&redis_pool).await;

    // 初始化 S3 客户端(必选)
    let s3_client = init_s3_client().await?;

    // 初始化邮件管理器(注册验证码使用阿里云提供商)
    let email = init_email_manager()?;

    let state = http_service::state::AppState {
        core: common::state::CoreState { db: pool, redis: redis_pool },
        s3: s3_client,
        email,
    };

    let address = read_global_config!("server", "address");

    HttpServer::new(move || {
        App::new()
            .wrap(TraceIdMiddleware)
            .wrap(from_fn(error_record_middleware))
            .app_data(web::Data::new(state.clone()))
            .wrap(middleware::Logger::default())
            .configure(http_service::http_service::configure_routes)
            .configure(configure_api_routes)
    })
    .bind_rustls_021(address, config)? // 绑定 HTTPS 端口
    // .bind(address)?
    .run()
    .await?;
    Ok(())
}
