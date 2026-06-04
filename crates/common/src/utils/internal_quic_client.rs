use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use quinn::{ClientConfig, Endpoint, TransportConfig};
use rustls::client::{ServerCertVerified, ServerCertVerifier};
use rustls::{Certificate, ServerName};
use tracing::{debug, error, info, warn};

use crate::utils::internal_quic_msg::{InternalQuicRequest, InternalQuicResponse};

/// Skip server certificate verification (internal services use self-signed certs)
#[derive(Debug)]
pub struct SkipServerVerification;

impl ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &Certificate,
        _intermediates: &[Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}

/// Create QUIC client config that skips server cert verification
pub fn make_internal_client_config() -> Result<ClientConfig> {
    debug!("[internal QUIC client] [single chat] creating client config (skipping cert verification)");
    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    let mut transport = TransportConfig::default();
    transport.max_idle_timeout(Some(Duration::from_secs(30).try_into()?));
    let mut config = ClientConfig::new(Arc::new(crypto));
    config.transport_config(Arc::new(transport));

    debug!("[internal QUIC client] [single chat] client config complete idle_timeout=30s");
    Ok(config)
}

/// Send a request to the internal QUIC service and receive a response
///
/// Connect -> Send request -> Read response -> Close connection
pub async fn send_internal_quic_msg(
    server_addr: SocketAddr,
    request: InternalQuicRequest,
) -> Result<InternalQuicResponse> {
    info!(
        "[internal QUIC client] [single chat] sending internal request target_user={} msg_type={} preferred_index={}",
        request.target_user, request.msg_type, request.preferred_index
    );

    // Create client config
    let client_config = make_internal_client_config()?;
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);
    debug!("[internal QUIC client] [single chat] endpoint created");

    // Establish connection
    info!("[internal QUIC client] [single chat] connecting to internal node {}", server_addr);
    let connection = endpoint
        .connect(server_addr, "localhost")?
        .await
        .map_err(|e| {
            error!("[internal QUIC client] [single chat] connection to internal node {} failed: {}", server_addr, e);
            anyhow::anyhow!("Internal QUIC connection to {} failed: {}", server_addr, e)
        })?;
    info!("[internal QUIC client] [single chat] connected to internal node {}", server_addr);

    // open bi-directional stream
    let (mut send_stream, mut recv_stream) = connection.open_bi().await?;
    debug!("[internal QUIC client] [single chat] bi-directional stream opened");

    // serialize and send request
    let body = bincode::serialize(&request)?;
    let body_len = body.len();
    debug!("[internal QUIC client] [single chat] request serialized size={} bytes", body_len);

    send_stream.write_all(&body).await?;
    debug!("[internal QUIC client] [single chat] request sent {} bytes", body_len);
    send_stream.finish().await?;
    debug!("[internal QUIC client] [single chat] send stream closed");

    // read response
    let mut buf = vec![0u8; 1024 * 16];
    debug!("[internal QUIC client] [single chat] waiting for response...");
    match recv_stream.read(&mut buf).await? {
        Some(len) => {
            debug!("[internal QUIC client] [single chat] response received size={} bytes", len);
            let resp: InternalQuicResponse = bincode::deserialize(&buf[..len])?;
            info!(
                "[internal QUIC client] [single chat] response parsed status={} delivered={:?} message={:?}",
                resp.status,
                resp.delivered,
                resp.message
            );
            Ok(resp)
        }
        None => {
            warn!("[internal QUIC client] [single chat] server closed stream, no response returned");
            Ok(InternalQuicResponse::error("server returned no response"))
        }
    }
}