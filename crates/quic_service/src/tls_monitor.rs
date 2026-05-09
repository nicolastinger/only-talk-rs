use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use quinn::Endpoint;
use rustls::{Certificate, PrivateKey};
use rustls_pemfile::{certs, ec_private_keys, pkcs8_private_keys, rsa_private_keys};
use sha2::Digest;
use tokio::sync::watch;
use tokio::time;
use tracing::{error, info, warn};
use x509_parser::prelude::*;

use crate::set_server::create_server_config;

/// TLS证书状态信息
#[derive(Debug, Clone)]
pub struct CertStatus {
    pub not_before: SystemTime,
    pub not_after: SystemTime,
    pub subject: String,
    pub days_remaining: i64,
    pub is_expired: bool,
    pub is_near_expiry: bool,
}

pub fn load_tls_certificates(cert_path: &str, key_path: &str, expiry_warning_days: i64) -> Result<(Vec<Certificate>, PrivateKey, CertStatus), Box<dyn std::error::Error>> {
    // 加载证书
    let mut cert_file = BufReader::new(File::open(cert_path)?);
    let cert_chain: Vec<Certificate> = certs(&mut cert_file)
        .map(|certs| certs.into_iter().map(Certificate).collect())
        .map_err(|_| "无法解析证书文件")?;

    if cert_chain.is_empty() {
        return Err("证书链为空".into());
    }

    // 解析证书以获取有效期信息
    let cert_status = parse_cert_expiry(&cert_chain[0].0, expiry_warning_days)?;

    // 加载私钥
    let mut key_file = BufReader::new(File::open(key_path)?);
    let mut keys = load_private_keys(&mut key_file)?;

    if keys.is_empty() {
        return Err("私钥文件为空".into());
    }

    let key = PrivateKey(keys.remove(0));

    Ok((cert_chain, key, cert_status))
}

/// 加载私钥，尝试不同类型的私钥格式
fn load_private_keys(key_file: &mut BufReader<File>) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    key_file.seek(SeekFrom::Start(0))?;
    if let Ok(keys) = rsa_private_keys(key_file) {
        if !keys.is_empty() {
            return Ok(keys);
        }
    }

    key_file.seek(SeekFrom::Start(0))?;
    if let Ok(keys) = ec_private_keys(key_file) {
        if !keys.is_empty() {
            return Ok(keys);
        }
    }

    key_file.seek(SeekFrom::Start(0))?;
    let keys = pkcs8_private_keys(key_file)?;
    Ok(keys)
}

/// 解析证书的有效期信息
fn parse_cert_expiry(cert_der: &[u8], expiry_warning_days: i64) -> Result<CertStatus, Box<dyn std::error::Error>> {
    let (_, cert) = X509Certificate::from_der(cert_der)?;

    let not_before = cert.validity().not_before.to_datetime().unix_timestamp();
    let not_after = cert.validity().not_after.to_datetime().unix_timestamp();

    let not_before_system = SystemTime::UNIX_EPOCH + Duration::from_secs(not_before as u64);
    let not_after_system = SystemTime::UNIX_EPOCH + Duration::from_secs(not_after as u64);

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs() as i64;

    let days_remaining = (not_after - now) / 86400;
    let is_expired = days_remaining <= 0;
    let is_near_expiry = days_remaining > 0 && days_remaining <= expiry_warning_days;

    let subject = cert
        .subject()
        .iter_common_name()
        .next()
        .map(|cn| cn.as_str().unwrap_or("unknown"))
        .unwrap_or("unknown")
        .to_string();

    Ok(CertStatus {
        not_before: not_before_system,
        not_after: not_after_system,
        subject,
        days_remaining,
        is_expired,
        is_near_expiry,
    })
}

/// 计算文件的SHA256哈希值
fn compute_file_hash(path: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let mut file = BufReader::new(File::open(path)?);
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().into())
}

/// 打印证书有效期信息
fn log_cert_status(status: &CertStatus) {
    info!(
        "TLS证书信息: 主题={}, 生效时间={:?}, 过期时间={:?}, 剩余天数={}",
        status.subject,
        status.not_before,
        status.not_after,
        status.days_remaining
    );

    if status.is_expired {
        error!("TLS证书已过期！");
    } else if status.is_near_expiry {
        warn!(
            "TLS证书将在 {} 天后过期，请尽快更新证书！",
            status.days_remaining
        );
    }
}

/// 启动TLS证书监控任务
/// - 检测证书文件是否更新
/// - 当证书剩余有效期<=阈值时，定时打印警告
/// - 使用Quinn 0.10+的set_server_config()实现热重载
pub fn start_tls_monitor(
    endpoint: Arc<Endpoint>,
    mut shutdown_rx: watch::Receiver<bool>,
    cert_path: String,
    key_path: String,
    watch_interval_secs: u64,
    expiry_warning_days: i64,
    expiry_check_interval_secs: u64,
) {
    tokio::spawn(async move {
        info!("TLS证书监控任务已启动");

        let mut last_cert_hash = compute_file_hash(&cert_path).unwrap_or_else(|e| {
            error!("计算初始证书哈希失败: {}", e);
            [0u8; 32]
        });

        let mut last_expiry_log = SystemTime::now();

        let mut interval = time::interval(Duration::from_secs(watch_interval_secs));

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    info!("TLS证书监控收到关闭信号，正在退出...");
                    return;
                }
                _ = interval.tick() => {
                    let current_hash = match compute_file_hash(&cert_path) {
                        Ok(h) => h,
                        Err(e) => {
                            error!("计算证书文件哈希失败: {}", e);
                            continue;
                        }
                    };

                    if current_hash != last_cert_hash {
                        info!("检测到TLS证书文件已更新，触发quinn热重载机制...");

                        match reload_tls_config(&endpoint, &cert_path, &key_path, expiry_warning_days) {
                            Ok(new_status) => {
                                info!("TLS证书热重载成功");
                                last_cert_hash = current_hash;
                                log_cert_status(&new_status);
                                last_expiry_log = SystemTime::now();
                            }
                            Err(e) => {
                                error!("TLS证书热重载失败: {}", e);
                            }
                        }
                    }

                    match load_tls_certificates(&cert_path, &key_path, expiry_warning_days) {
                        Ok((_, _, status)) => {
                            let now = SystemTime::now();

                            if status.is_near_expiry || status.is_expired {
                                let should_log = now
                                    .duration_since(last_expiry_log)
                                    .map(|d| d.as_secs() >= expiry_check_interval_secs)
                                    .unwrap_or(true);

                                if should_log {
                                    log_cert_status(&status);
                                    last_expiry_log = now;
                                }
                            }
                        }
                        Err(e) => {
                            error!("读取证书状态失败: {}", e);
                        }
                    }
                }
            }
        }
    });
}

/// 热重载TLS配置
/// 使用Quinn 0.10+的set_server_config() API
fn reload_tls_config(
    endpoint: &Arc<Endpoint>,
    cert_path: &str,
    key_path: &str,
    expiry_warning_days: i64,
) -> Result<CertStatus, Box<dyn std::error::Error>> {
    let (cert_chain, key, cert_status) = load_tls_certificates(cert_path, key_path, expiry_warning_days)?;

    let server_config = create_server_config(cert_chain, key)?;

    endpoint.set_server_config(Some(server_config));

    info!("已通过set_server_config()更新TLS配置，新连接将使用新证书");
    Ok(cert_status)
}
