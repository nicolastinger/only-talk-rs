use std::sync::Arc;

use common::read_global_config;
use tracing::info;

use crate::external::chat_node::ChatNode;
use crate::external::config::ChatNodeConfig;
use crate::external::lifecycle::ServiceLifecycle;
use crate::internal::internal_config::InternalQuicConfig;
use crate::internal::internal_quic_server::run_internal_server;
use crate::nat_ip::nat_udp_service::run_udp_server;

/// Start QUIC service (ChatNode + NAT UDP + internal QUIC), fully self-contained
pub async fn start_server() -> anyhow::Result<Arc<ChatNode>> {
    let resolved_content = common::init_app_config()?;

    let config = ChatNodeConfig::from_toml_str(&resolved_content)?;
    let mut node = ChatNode::new(config);
    node.init().await?;
    let node = Arc::new(node);
    node.start().await?;

    // Initialize infrastructure (idempotent, api layer subsequent calls won't re-initialize)
    let redis_url = read_global_config!("redis", "url");
    match common::init_redis(&redis_url) {
        Ok(_) => info!("Redis connection pool ready"),
        Err(e) => tracing::warn!("Redis initialization failed: {}", e),
    }

    let db_url = read_global_config!("database", "url");
    match common::init_sql_pool(&db_url).await {
        Ok(_) => info!("database connection pool ready"),
        Err(e) => tracing::warn!("database initialization failed: {}", e),
    }

    let connections = node.connections();
    let server_index = node.config().server_index;

    // Cluster: register external node + start server_count background sync + node key renewal
    {
        let redis = common::REDIS_CLIENT.read().await;
        if let Some(redis) = redis.as_ref() {
            let node_address = node.config().node_address.clone();
            if let Err(e) = common::utils::server_count_sync::register_external_node(
                redis,
                server_index,
                &node_address,
            )
            .await
            {
                tracing::warn!("external QUIC node registration failed: {}", e);
            }
            common::utils::server_count_sync::start_server_count_sync(
                redis.clone(),
                server_index,
                node_address,
            );
            info!("server_count background sync started (server_index={})", server_index);
        }
    }

    // Start NAT discovery + client P2P request forwarding UDP service
    run_udp_server(connections.clone()).await?;

    // Start internal QUIC service
    let internal_config = InternalQuicConfig::from_toml_str(&resolved_content)?;
    let (internal_shutdown_tx, internal_shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        let _tx = internal_shutdown_tx;
        run_internal_server(internal_config, connections, internal_shutdown_rx).await;
    });

    Ok(node)
}
