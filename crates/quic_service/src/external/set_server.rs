use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use quinn::{ClientConfig, Endpoint, ServerConfig, TransportConfig};
use rustls::{Certificate, PrivateKey, RootCertStore};
use rustls_pemfile::{certs, ec_private_keys, pkcs8_private_keys, rsa_private_keys};

/// Configure QUIC settings for client use.
#[allow(dead_code)]
pub fn configure_client() -> ClientConfig {
    let mut root_store = RootCertStore::empty();
    root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject.as_ref().to_vec(),
            ta.subject_public_key_info.as_ref().to_vec(),
            ta.name_constraints.as_ref().map(|nc| nc.as_ref().to_vec()),
        )
    }));

    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let mut config = ClientConfig::new(Arc::new(crypto));
    let mut time_out_config = TransportConfig::default();
    let idle_timeout =
        Duration::from_secs(190).try_into().unwrap_or_else(|_| panic!("failed to set timeout"));
    time_out_config.max_idle_timeout(Some(idle_timeout));
    time_out_config.max_concurrent_uni_streams(32_u8.into());
    config.transport_config(Arc::from(time_out_config));
    config
}

pub fn make_server_endpoint(
    bind_addr: SocketAddr,
    cert_path: &str,
    key_path: &str,
) -> Result<(Endpoint, Vec<u8>), Box<dyn Error>> {
    let (server_config, server_cert) = configure_server(cert_path, key_path)?;
    let endpoint = Endpoint::server(server_config, bind_addr)?;
    Ok((endpoint, server_cert))
}

pub fn configure_server(
    cert_path: &str,
    key_path: &str,
) -> Result<(ServerConfig, Vec<u8>), Box<dyn Error>> {
    let mut cert_file = BufReader::new(
        File::open(cert_path)
            .map_err(|e| format!("Failed to open PEM file {}: {}", cert_path, e))?,
    );
    let cert_chain: Vec<Certificate> = certs(&mut cert_file)
        .map(|certs| certs.into_iter().map(Certificate).collect())
        .map_err(|_| "Unable to parse certificate file")?;

    let key_file = &mut BufReader::new(
        File::open(key_path)
            .map_err(|e| format!("Failed to open TLS certificate key {}: {}", key_path, e))?,
    );

    let mut keys = load_private_keys(key_file)?;
    if keys.is_empty() {
        return Err("Private key file is empty".into());
    }
    let key = PrivateKey(keys.remove(0));

    let cert_der = cert_chain.first().cloned().ok_or("Certificate chain is empty")?.0;

    let server_config = create_server_config(cert_chain, key)?;

    Ok((server_config, cert_der))
}

pub fn create_server_config(
    cert_chain: Vec<Certificate>,
    key: PrivateKey,
) -> Result<ServerConfig, Box<dyn Error>> {
    let mut server_config = ServerConfig::with_single_cert(cert_chain, key)?;
    let transport_config =
        Arc::get_mut(&mut server_config.transport).ok_or("Failed to get transport config")?;
    transport_config.max_concurrent_uni_streams(32_u8.into());
    let idle_timeout = Duration::from_secs(190).try_into().map_err(|_| "Failed to set timeout")?;
    transport_config.max_idle_timeout(Some(idle_timeout));
    transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
    Ok(server_config)
}

fn load_private_keys(key_file: &mut BufReader<File>) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    key_file
        .seek(SeekFrom::Start(0))
        .map_err(|e| format!("Unable to reset file read position: {}", e))?;
    if let Ok(keys) = rsa_private_keys(key_file) {
        if !keys.is_empty() {
            return Ok(keys);
        }
    }

    key_file
        .seek(SeekFrom::Start(0))
        .map_err(|e| format!("Unable to reset file read position: {}", e))?;
    if let Ok(keys) = ec_private_keys(key_file) {
        if !keys.is_empty() {
            return Ok(keys);
        }
    }

    key_file
        .seek(SeekFrom::Start(0))
        .map_err(|e| format!("Unable to reset file read position: {}", e))?;
    let keys =
        pkcs8_private_keys(key_file).map_err(|e| format!("Unable to read private key: {}", e))?;
    Ok(keys)
}
