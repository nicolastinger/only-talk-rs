use std::sync::Arc;

use common::read_global_config;
use common::state::CoreState;
use tracing::info;

use crate::external::chat_node::ChatNode;
use crate::external::config::ChatNodeConfig;
use crate::external::lifecycle::ServiceLifecycle;
use crate::internal::internal_config::InternalQuicConfig;
use crate::internal::internal_quic_server::run_internal_server;
use crate::nat_ip::nat_udp_service::run_udp_server;

/// 启动 QUIC 服务（ChatNode + NAT UDP + 内部 QUIC），完全自包含
pub async fn start_server() -> anyhow::Result<Arc<ChatNode>> {
    let resolved_content = common::init_app_config()?;

    let redis_url = read_global_config!("redis", "url");
    let redis = match common::init_redis(&redis_url) {
        Ok(pool) => {
            info!("Redis 连接池就绪");
            pool
        }
        Err(e) => return Err(anyhow::anyhow!("Redis 初始化失败: {}", e)),
    };

    let db_url = read_global_config!("database", "url");
    let db = match common::init_sql_pool(&db_url).await {
        Ok(db) => {
            info!("数据库连接池就绪");
            db
        }
        Err(e) => return Err(anyhow::anyhow!("数据库初始化失败: {}", e)),
    };

    let core = CoreState { db, redis };

    let config = ChatNodeConfig::from_toml_str(&resolved_content)?;
    let mut node = ChatNode::new(config, core.clone());
    node.init().await?;
    let node = Arc::new(node);
    node.start().await?;

    let connections = node.connections();
    let server_index = node.config().server_index;

    // 集群：注册外部节点 + 启动 server_count 后台同步 + 节点密钥续期
    {
        let node_address = node.config().node_address.clone();
        if let Err(e) = common::utils::server_count_sync::register_external_node(
            &core.redis,
            server_index,
            &node_address,
        )
        .await
        {
            tracing::warn!("外部 QUIC 节点注册失败: {}", e);
        }
        common::utils::server_count_sync::start_server_count_sync(
            core.redis.clone(),
            server_index,
            node_address,
        );
        info!("server_count 后台同步已启动 (server_index={})", server_index);
    }

    // 启动 NAT 发现 + 客户端 P2P 请求转发 UDP 服务
    run_udp_server(core.clone(), connections.clone()).await?;

    // 启动内部 QUIC 服务
    let internal_config = InternalQuicConfig::from_toml_str(&resolved_content)?;
    let (internal_shutdown_tx, internal_shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        let _tx = internal_shutdown_tx;
        run_internal_server(core, internal_config, connections, internal_shutdown_rx).await;
    });

    Ok(node)
}
