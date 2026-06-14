//! QUIC service independent entry point
//!
//! Starts only QUIC ChatNode + NAT UDP + internal QUIC service, no HTTP.

use common::fatal_panic_async;
use common::tracing::init_tracing;
use quic_service::init_server::start_server;
use quic_service::{ChatNode, ServiceLifecycle};
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("Failed to load .env file: {}", e);
    }

    let _guard = init_tracing();
    info!("starting QUIC service (standalone mode)");

    let chat_node: Arc<ChatNode> = match start_server().await {
        Ok(node) => node,
        Err(e) => fatal_panic_async(&format!("failed to start QUIC service: {:?}", e)).await,
    };

    info!("QUIC service ready, press Ctrl+C to stop");

    // Wait for exit signal
    tokio::signal::ctrl_c().await.unwrap_or_else(|e| {
        error!("failed to register Ctrl+C handler: {}", e);
    });

    info!("received shutdown signal, shutting down gracefully...");
    if let Err(e) = chat_node.stop().await {
        error!("failed to shutdown QUIC service: {:?}", e);
    }
    info!("QUIC service stopped");
}
