#![deny(clippy::unwrap_used)]
use api::init_server;
use common::tracing::init_tracing;
use common::fatal_panic_async;
use quic_service::init_server::start_server;
use tracing::{debug, error, info};

/// Main entry point, using Tokio async runtime
#[actix_web::main]
async fn main() {
    // Load .env file
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("Failed to load .env file: {}", e);
    }

    let _guard = init_tracing();

    debug!("log level set to debug");
    info!("starting application");

    // 1. Start QUIC service (ChatNode + NAT UDP + internal QUIC, fully self-contained)
    let _chat_node = match start_server().await {
        Ok(node) => node,
        Err(e) => fatal_panic_async(&format!("failed to start QUIC service: {:?}", e)).await,
    };

    // 2. Start HTTP API service
    if let Err(e) = init_server::start_server().await {
        error!("failed to start HTTP service {}, backtrace {:?}", e, e.backtrace());
        fatal_panic_async(&format!("failed to start HTTP service: {:?}", e)).await;
    }
}
