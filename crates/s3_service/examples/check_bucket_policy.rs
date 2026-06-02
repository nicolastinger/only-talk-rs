use s3_service::config::S3Config;
use s3_service::S3Client;

#[tokio::main]
async fn main() {
    println!("====== 检查桶策略状态 ======\n");

    let bucket_name = std::env::args().nth(1).unwrap_or_else(|| "user-avatar".to_string());
    println!("目标桶: {}", bucket_name);

    let config = S3Config::default_minio();
    let endpoint_url = config.endpoint_url.clone();
    println!("配置信息:");
    println!("  Endpoint: {}", endpoint_url);

    let client = match S3Client::new(config).await {
        Ok(c) => c,
        Err(e) => {
            println!("✗ S3 客户端创建失败: {}", e);
            return;
        }
    };

    println!("\n1. 获取当前桶策略...");
    match client.inner.get_bucket_policy().bucket(&bucket_name).send().await {
        Ok(result) => {
            let policy = result.policy().unwrap_or_default();
            println!("当前策略:");
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(policy) {
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            } else {
                println!("{}", policy);
            }
        }
        Err(e) => {
            use aws_sdk_s3::error::ProvideErrorMetadata;
            println!("获取策略失败: {:?}", e.meta().message());
        }
    }

    println!("\n2. 尝试使用 MinIO 兼容的公开读策略...");
    
    let minio_public_policy = serde_json::json!({
        "Version": "2012-10-17",
        "Statement": [
            {
                "Effect": "Allow",
                "Principal": "*",
                "Action": ["s3:GetObject"],
                "Resource": [format!("arn:aws:s3:::{}/*", bucket_name)]
            }
        ]
    });

    let policy_str = serde_json::to_string(&minio_public_policy).unwrap();
    println!("新策略内容:");
    println!("{}", serde_json::to_string_pretty(&minio_public_policy).unwrap());

    match client
        .inner
        .put_bucket_policy()
        .bucket(&bucket_name)
        .policy(&policy_str)
        .send()
        .await
    {
        Ok(_) => println!("✓ 策略更新成功"),
        Err(e) => {
            use aws_sdk_s3::error::ProvideErrorMetadata;
            println!("策略更新失败: {:?}", e.meta().message());
        }
    }

    println!("\n3. 验证更新后的策略...");
    match client.inner.get_bucket_policy().bucket(&bucket_name).send().await {
        Ok(result) => {
            let policy = result.policy().unwrap_or_default();
            println!("当前策略:");
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(policy) {
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            } else {
                println!("{}", policy);
            }
        }
        Err(e) => {
            use aws_sdk_s3::error::ProvideErrorMetadata;
            println!("获取策略失败: {:?}", e.meta().message());
        }
    }

    println!("\n4. 列出桶中的文件...");
    match client.inner.list_objects_v2().bucket(&bucket_name).max_keys(5).send().await {
        Ok(result) => {
            let objects = result.contents();
            if objects.is_empty() {
                println!("  桶中没有文件");
            } else {
                for obj in objects {
                    let key = obj.key().unwrap_or_default();
                    println!("  - {}", key);
                    println!("    公开URL: {}/{}/{}", endpoint_url, bucket_name, key);
                }
            }
        }
        Err(e) => {
            println!("列出文件失败: {:?}", e);
        }
    }

    println!("\n====== 完成 ======");
}