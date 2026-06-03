//! S3 服务单元测试
//!
//! 测试配置解析、错误处理等不需要实际服务的功能

use crate::config::S3Provider;
use crate::error::S3Error;

// 集成测试需要的导入
#[cfg(feature = "integration-test")]
use crate::S3Client;
#[cfg(feature = "integration-test")]
use aws_sdk_s3::error::ProvideErrorMetadata;
#[cfg(feature = "integration-test")]
use aws_sdk_s3::operation::RequestId;

/// 测试 S3Provider 解析
#[test]
fn test_s3_provider_from_str() {
    // 测试 MinIO
    assert_eq!(S3Provider::from_str("minio").unwrap(), S3Provider::MinIO);
    assert_eq!(S3Provider::from_str("MINIO").unwrap(), S3Provider::MinIO);
    assert_eq!(S3Provider::from_str("MinIO").unwrap(), S3Provider::MinIO);
    
    // 测试 AliyunOSS
    assert_eq!(S3Provider::from_str("aliyun_oss").unwrap(), S3Provider::AliyunOSS);
    assert_eq!(S3Provider::from_str("aliyun").unwrap(), S3Provider::AliyunOSS);
    assert_eq!(S3Provider::from_str("oss").unwrap(), S3Provider::AliyunOSS);
    
    // 测试 AwsS3
    assert_eq!(S3Provider::from_str("aws_s3").unwrap(), S3Provider::AwsS3);
    assert_eq!(S3Provider::from_str("aws").unwrap(), S3Provider::AwsS3);
    
    // 测试无效值
    assert!(S3Provider::from_str("invalid").is_err());
    assert!(S3Provider::from_str("").is_err());
}

/// 测试 S3Provider Display 实现
#[test]
fn test_s3_provider_display() {
    assert_eq!(format!("{}", S3Provider::MinIO), "minio");
    assert_eq!(format!("{}", S3Provider::AliyunOSS), "aliyun_oss");
    assert_eq!(format!("{}", S3Provider::AwsS3), "aws_s3");
}

/// 测试默认 MinIO 配置
#[test]
fn test_default_minio_config() {
    let config = crate::config::S3Config::default_minio();
    
    assert_eq!(config.provider, S3Provider::MinIO);
    assert_eq!(config.endpoint_url, "http://xxxx");
    assert_eq!(config.access_key_id, "xxxx");
    assert_eq!(config.secret_access_key, "xxxxx");
    assert_eq!(config.region, "us-east-1");
    assert_eq!(config.default_bucket, "only-talk-rs");
    assert!(config.force_path_style);
    assert!(config.enabled);
    assert_eq!(config.presign_expire_seconds, 3600);
    assert_eq!(config.multipart_threshold, 10 * 1024 * 1024);
    assert_eq!(config.multipart_chunk_size, 5 * 1024 * 1024);
    assert_eq!(config.max_concurrent_uploads, 10);
}

/// 测试 S3Error Display 实现
#[test]
fn test_s3_error_display() {
    assert_eq!(
        format!("{}", S3Error::AwsError("test error".to_string())),
        "AWS SDK错误: test error"
    );
    
    assert_eq!(
        format!("{}", S3Error::ConfigError("invalid config".to_string())),
        "S3配置错误: invalid config"
    );
    
    assert_eq!(
        format!("{}", S3Error::BucketNotFound("my-bucket".to_string())),
        "存储桶不存在: my-bucket"
    );
    
    assert_eq!(
        format!("{}", S3Error::ObjectNotFound("file.txt".to_string())),
        "对象不存在: file.txt"
    );
    
    assert_eq!(
        format!("{}", S3Error::PermissionDenied("access denied".to_string())),
        "权限不足: access denied"
    );
}

/// 测试 S3Error From<std::io::Error> 实现
#[test]
fn test_s3_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let s3_err: S3Error = io_err.into();
    
    match s3_err {
        S3Error::IoError(msg) => assert!(msg.contains("file not found")),
        _ => panic!("expected IoError type"),
    }
}

/// 测试 StorageType 枚举
#[test]
fn test_storage_type() {
    use crate::storage::StorageType;
    
    assert_eq!(format!("{}", StorageType::Local), "local");
    assert_eq!(format!("{}", StorageType::S3), "s3");
}

/// 测试 PresignedMethod 枚举
#[test]
fn test_presigned_method() {
    use crate::storage::PresignedMethod;
    
    // 简单验证枚举存在且可以创建
    let _get = PresignedMethod::Get;
    let _put = PresignedMethod::Put;
}

// ==================== 集成测试（需要实际 S3 服务）====================

/// 诊断 S3 连接问题
/// 
/// 使用方法：
/// ```bash
/// cargo test --package s3_service --lib tests::diagnose_s3_connection --features "integration-test" -- --nocapture
/// ```
#[cfg(feature = "integration-test")]
#[tokio::test]
async fn diagnose_s3_connection() {
    use crate::config::S3Config;
    use crate::client::S3Client;
    use tracing::info;

    // 初始化日志
    let _ = tracing_subscriber::fmt::try_init();

    info!("====== S3 Connection Diagnostic Test ======");

    // 1. 加载配置
    let config = S3Config::default_minio();
    info!("Configuration:");
    info!("  Provider: {:?}", config.provider);
    info!("  Endpoint: {}", config.endpoint_url);
    info!("  Access Key: {}", config.access_key_id);
    info!("  Region: {}", config.region);
    info!("  Default Bucket: {}", config.default_bucket);
    info!("  Force Path Style: {}", config.force_path_style);
    
    // 2. 创建客户端
    info!("\nCreating S3 client...");
    let client = match S3Client::new(config).await {
        Ok(c) => {
            info!("✓ S3 client created successfully");
            c
        }
        Err(e) => {
            info!("✗ S3 client creation failed: {}", e);
            panic!("failed to create S3 client");
        }
    };
    
    // 3. 测试连接 - 列出所有桶
    info!("\nTesting connection (listing all buckets)...");
    match client.inner.list_buckets().send().await {
        Ok(result) => {
            info!("✓ Connection successful!");
            let buckets = result.buckets();
            info!("  Current bucket count: {}", buckets.len());
            for bucket in buckets {
                info!("  - {}", bucket.name().unwrap_or("unknown"));
            }
        }
        Err(e) => {
            info!("✗ Connection failed: {:?}", e);
            info!("\nPossible reasons:");
            info!("  1. Network connection issue - check if endpoint URL is correct");
            info!("  2. S3 service not started - confirm MinIO is running");
            info!("  3. Firewall blocked - check if port is open");
            info!("  4. TLS/SSL issue - HTTP vs HTTPS");
            return;
        }
    }
    
    // 4. 检查默认桶是否存在
    info!("\nChecking default bucket '{}'...", client.config.default_bucket);
    match client.inner.head_bucket().bucket(&client.config.default_bucket).send().await {
        Ok(_) => {
            info!("✓ Default bucket already exists");
        }
        Err(e) => {
            info!("✗ Default bucket does not exist or inaccessible: {:?}", e);
            info!("\nAttempting to create bucket...");
            
            // 5. 尝试创建桶
            match client.inner.create_bucket().bucket(&client.config.default_bucket).send().await {
                Ok(_) => {
                    info!("✓ Bucket created successfully");
                }
                Err(create_err) => {
                    info!("✗ Bucket creation failed: {:?}", create_err);
                    info!("\nPossible reasons:");
                    info!("  1. Insufficient permissions - check Access Key and Secret Key");
                    info!("  2. Bucket name already taken");
                    info!("  3. Region configuration error");
                    info!("  4. MinIO version incompatible");

                    // 尝试获取更详细的错误信息
                    let meta = create_err.meta();
                    info!("\nError details:");
                    info!("  Error code: {:?}", meta.code());
                    info!("  Error message: {:?}", meta.message());
                    info!("  Request ID: {:?}", meta.request_id());
                }
            }
        }
    }
    
    // 6. 测试上传和下载
    info!("\nTesting file upload...");
    let test_key = "test/diagnostic-test.txt";
    let test_data = b"Hello, S3 Diagnostic Test!";
    
    match client.inner.put_object()
        .bucket(&client.config.default_bucket)
        .key(test_key)
        .body(bytes::Bytes::from_static(test_data).into())
        .send()
        .await
    {
        Ok(_) => {
            info!("✓ File uploaded successfully");
            
            // 尝试下载
            info!("\nTesting file download...");
            match client.inner.get_object()
                .bucket(&client.config.default_bucket)
                .key(test_key)
                .send()
                .await
            {
                Ok(result) => {
                    let body = result.body.collect().await.unwrap().into_bytes();
                    info!("✓ File downloaded successfully, size: {} bytes", body.len());
                    
                    // 清理测试文件
                    let _ = client.inner.delete_object()
                        .bucket(&client.config.default_bucket)
                        .key(test_key)
                        .send()
                        .await;
                    info!("✓ Test file cleaned up");
                }
                Err(e) => {
                    info!("✗ File download failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            info!("✗ File upload failed: {:?}", e);
        }
    }
    
    info!("\n====== Diagnostic Complete ======");
}

/// 测试桶创建权限
#[cfg(feature = "integration-test")]
#[tokio::test]
async fn test_bucket_permissions() {
    use crate::config::S3Config;

    let config = S3Config::default_minio();
    let client = S3Client::new(config).await.expect("无法创建客户端");
    
    // 测试创建多个桶
    let test_buckets = vec![
        "test-bucket-1",
        "test-bucket-2",
        "chat-file-preview",
        "chat-file-origin",
    ];
    
    for bucket_name in test_buckets {
        println!("\n测试创建桶: {}", bucket_name);
        
        // 先检查是否存在
        let exists = client.inner.head_bucket().bucket(bucket_name).send().await.is_ok();
        if exists {
            println!("  ✓ 桶已存在");
            continue;
        }
        
        // 尝试创建
        match client.inner.create_bucket().bucket(bucket_name).send().await {
            Ok(_) => println!("  ✓ 创建成功"),
            Err(e) => {
                println!("  ✗ 创建失败: {:?}", e);
                let meta = e.meta();
                println!("  错误码: {:?}", meta.code());
                println!("  错误消息: {:?}", meta.message());
            }
        }
    }
}

/// 测试不同区域的桶创建
#[cfg(feature = "integration-test")]
#[tokio::test]
async fn test_different_regions() {
    use crate::config::{S3Config, S3Provider};

    let regions = vec!["us-east-1", "eu-west-1", "ap-northeast-1"];
    
    for region in regions {
        println!("\n测试区域: {}", region);
        
        let mut config = S3Config::default_minio();
        config.region = region.to_string();
        
        match S3Client::new(config).await {
            Ok(client) => {
                println!("  ✓ 客户端创建成功（区域: {}）", region);
                
                // 尝试列出桶
                match client.inner.list_buckets().send().await {
                    Ok(_) => println!("  ✓ 连接成功"),
                    Err(e) => println!("  ✗ 连接失败: {:?}", e),
                }
            }
            Err(e) => {
                println!("  ✗ 客户端创建失败: {}", e);
            }
        }
    }
}