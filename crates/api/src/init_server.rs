use std::collections::HashMap;
use std::sync::Arc;

use actix_web::middleware::from_fn;
use actix_web::{App, HttpServer, middleware, web};
use common::{init_app_config, init_redis, init_sql_pool, read_global_config, verify_redis};
use email_service::config::{AliyunConfig, EmailServiceConfig, ProviderConfig};
use http_service;
use http_service::middleware::TraceIdMiddleware;
use http_service::utils::auth_middleware::auth_middleware;
use s3_service::client::GlobalS3Client;
use s3_service::config::S3Config;
use tracing::{error, info};

use crate::controller::configure_api_routes;

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

    info!("正在初始化 S3 客户端 - Provider: {}", config.provider);
    let client = GlobalS3Client::init(config)
        .await
        .map_err(|e| anyhow::anyhow!("S3 client initialization failed: {}", e))?;
    info!("S3 客户端初始化成功");
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
        info!("邮件服务未启用,将不会发送验证码");
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
    info!("正在使用阿里云提供商初始化邮件管理器");
    Ok(Arc::new(email_service::EmailManager::new(config)?))
}

/// 初始化服务
pub async fn start_server() -> anyhow::Result<()> {
    init_app_config()?;

    let url = read_global_config!("database", "url");

    let pool = init_sql_pool(&url).await?;

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

    // 任务09 §3.3: 每日清理窗口外消息(启动即跑一轮, 之后每 24h; 分批 10000 循环至清空)
    {
        let db = state.core.db.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tick.tick().await;
                let Ok(now) = common::utils::time::get_now_time_stamp_as_millis() else {
                    error!("[cleanup] 获取时间戳失败, 跳过本轮清理");
                    continue;
                };
                let boundary = now - common::config_str::SYNC_WINDOW_DAYS * 24 * 3600 * 1000;
                let mut total = 0u64;
                loop {
                    match common::models::session_entity::cleanup::delete_messages_before(
                        &db, boundary, 10_000,
                    )
                    .await
                    {
                        Ok(n) if n > 0 => total += n,
                        Ok(_) => break,
                        Err(e) => {
                            error!("[cleanup] 清理批次失败: {}", e);
                            break;
                        }
                    }
                }
                info!("[cleanup] 窗口外消息清理完成: {} 行, boundary={}", total, boundary);
            }
        });
    }

    HttpServer::new(move || {
        App::new()
            .wrap(TraceIdMiddleware)
            .wrap(from_fn(auth_middleware))
            .app_data(web::Data::new(state.clone()))
            .wrap(middleware::Logger::default())
            .configure(http_service::http_service::configure_routes)
            .configure(configure_api_routes)
    })
    // 网关方案: HTTP 面 TLS 终止外移到 nginx(443), actix 只监听明文 HTTP,
    // 仅 compose 内网可达(compose 不再发布 8443)。裸机部署须把 [server] address 改 127.0.0.1:8443。
    .bind(address)?
    .run()
    .await?;
    Ok(())
}
