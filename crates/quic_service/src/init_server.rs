use std::sync::Arc;

use common::read_global_config;
use deadpool_redis::redis::AsyncCommands;
use tracing::info;

use crate::external::chat_node::ChatNode;
use crate::external::config::ChatNodeConfig;
use crate::internal::internal_config::InternalQuicConfig;
use crate::internal::internal_quic_server::run_internal_server;
use crate::external::lifecycle::ServiceLifecycle;
use crate::nat_ip::nat_udp_service::run_udp_server;

/// 启动 QUIC 服务（ChatNode + NAT UDP + 内网 QUIC），完全自包含
pub async fn start_server() -> anyhow::Result<Arc<ChatNode>> {
    let resolved_content = common::init_app_config()?;

    let config = ChatNodeConfig::from_toml_str(&resolved_content)?;
    let mut node = ChatNode::new(config);
    node.init().await?;
    let node = Arc::new(node);
    node.start().await?;

    // 初始化基础设施（幂等操作，api 层后续调用不会重复初始化）
    let redis_url = read_global_config!("redis", "url");
    match common::init_redis(&redis_url) {
        Ok(_) => info!("Redis 连接池已就绪"),
        Err(e) => tracing::warn!("Redis 初始化失败: {}", e),
    }

    let db_url = read_global_config!("database", "url");
    match common::init_sql_pool(&db_url).await {
        Ok(_) => info!("数据库连接池已就绪"),
        Err(e) => tracing::warn!("数据库初始化失败: {}", e),
    }

    let connections = node.connections();
    let server_index = node.config().server_index;

    // 集群：注册外网节点 + 启动 server_count 后台同步 + 节点 key 续期
    {
        let redis = common::REDIS_CLIENT.read().await;
        if let Some(redis) = redis.as_ref() {
            let node_address = node.config().node_address.clone();
            if let Err(e) =
                common::utils::server_count_sync::register_external_node(redis, server_index, &node_address).await
            {
                tracing::warn!("外网 QUIC 节点注册失败: {}", e);
            }
            common::utils::server_count_sync::start_server_count_sync(
                redis.clone(),
                server_index,
                node_address,
            );
            info!("server_count 后台同步已启动 (server_index={})", server_index);
        }
    }

    // 启动 NAT 发现 + 客户端 P2P 请求转发 UDP 服务
    run_udp_server(connections.clone()).await?;

    // 启动内网 QUIC 服务
    let internal_config = InternalQuicConfig::from_toml_str(&resolved_content)?;
    let (internal_shutdown_tx, internal_shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        let _tx = internal_shutdown_tx;
        run_internal_server(internal_config, connections, internal_shutdown_rx).await;
    });

    // 预热到其他节点的内网 QUIC 连接
    {
        let own_index = server_index;
        tokio::spawn(async move {
            let redis_guard = common::REDIS_CLIENT.read().await;
            if let Some(redis) = redis_guard.as_ref() {
                if let Ok(mut conn) = redis.get().await {
                    let prefix = common::config_str::REDIS_INTERNAL_QUIC_SERVERS;
                    let pattern = format!("{}*", prefix);
                    let keys: Result<Vec<String>, _> =
                        deadpool_redis::redis::cmd("KEYS").arg(&pattern).query_async(&mut conn).await;
                    if let Ok(keys) = keys {
                        let mut addrs = Vec::new();
                        for key in keys {
                            let index_str = key.strip_prefix(prefix).unwrap_or_default();
                            if let Ok(idx) = index_str.parse::<u32>() {
                                if idx == own_index {
                                    continue;
                                }
                                if let Ok(addr_str) = conn.get::<_, String>(&key).await {
                                    if let Ok(addr) = addr_str.parse::<std::net::SocketAddr>() {
                                        addrs.push(addr);
                                    }
                                }
                            }
                        }
                        if !addrs.is_empty() {
                            common::utils::internal_quic_client::get_pool().warmup(addrs).await;
                        }
                    }
                }
            }
        });
    }

    Ok(node)
}
