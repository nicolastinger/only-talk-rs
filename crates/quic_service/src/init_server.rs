use std::sync::Arc;

use tracing::info;

use crate::chat_node::ChatNode;
use crate::config::ChatNodeConfig;
use crate::internal_config::InternalQuicConfig;
use crate::internal_quic_server::run_internal_server;
use crate::lifecycle::ServiceLifecycle;
use crate::nat_ip::nat_udp_service::run_udp_server;

/// 替换 TOML 内容中的 `${VAR_NAME}` 环境变量占位符
fn substitute_env_vars(content: String) -> String {
    let mut result = content;
    loop {
        let Some(start) = result.find("${") else { break };
        let Some(end) = result[start..].find('}') else { break };
        let var_name = &result[start + 2..start + end];
        let var_value = std::env::var(var_name).unwrap_or_default();
        result = result.replace(&format!("${{{}}}", var_name), &var_value);
    }
    result
}

/// 启动 QUIC 服务（ChatNode + NAT UDP + 内网 QUIC），完全自包含
pub async fn start_server() -> anyhow::Result<Arc<ChatNode>> {
    // 读取配置并替换环境变量占位符
    let raw_content = std::fs::read_to_string("./config/app_config.toml")?;
    let resolved_content = substitute_env_vars(raw_content);

    let config = ChatNodeConfig::from_toml_str(&resolved_content)?;
    let mut node = ChatNode::new(config);
    node.init().await?;
    let node = Arc::new(node);
    node.start().await?;

    // 初始化基础设施（幂等操作，api 层后续调用不会重复初始化）
    if let Ok(toml_value) = toml::from_str::<toml::Value>(&resolved_content) {
        // Redis
        if let Some(redis_url) = toml_value
            .get("redis")
            .and_then(|r| r.get("url"))
            .and_then(|v| v.as_str())
        {
            match entity::init_redis(redis_url) {
                Ok(_) => info!("Redis 连接池已就绪"),
                Err(e) => tracing::warn!("Redis 初始化失败: {}", e),
            }
        }

        // 数据库（消息持久化需要）
        if let Some(db_url) = toml_value
            .get("database")
            .and_then(|d| d.get("url"))
            .and_then(|v| v.as_str())
        {
            match entity::init_sql_pool(db_url).await {
                Ok(_) => info!("数据库连接池已就绪"),
                Err(e) => tracing::warn!("数据库初始化失败: {}", e),
            }
        }
    }

    let connections = node.connections();

    // 将外网 QUIC 服务注册到 Redis，供客户端发现
    {
        use deadpool_redis::redis::AsyncCommands;
        use entity::config_str::REDIS_EXTERNAL_QUIC_SERVERS;
        let redis = entity::REDIS_CLIENT.read().await;
        if let Some(redis) = redis.as_ref() {
            if let Ok(mut conn) = redis.get().await {
                let server_addr = node.config().bind_address.to_string();
                let key = format!("{}{}", REDIS_EXTERNAL_QUIC_SERVERS, node.config().server_name);
                let _: Result<(), _> = conn
                    .set_ex::<&str, &str, ()>(&key, &server_addr, 7200)
                    .await;
                info!("外网 QUIC 服务已注册到 Redis: key={} value={}", key, server_addr);
            }
        }
    }

    // 启动 NAT UDP 服务（P2P 打洞）
    run_udp_server(connections.clone()).await?;

    // 启动内网 QUIC 服务
    let internal_config = InternalQuicConfig::from_toml_str(&resolved_content)?;
    let (internal_shutdown_tx, internal_shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        // sender 移入 task 保持存活，避免 receiver 立即读到 close
        let _tx = internal_shutdown_tx;
        run_internal_server(internal_config, connections, internal_shutdown_rx).await;
    });

    Ok(node)
}
