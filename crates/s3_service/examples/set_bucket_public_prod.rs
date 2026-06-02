use s3_service::config::{S3Config, S3Provider};
use s3_service::S3Client;

#[tokio::main]
async fn main() {
    println!("====== 设置桶公开读策略 ======\n");

    let args: Vec<String> = std::env::args().collect();
    
    let endpoint = args.get(1).cloned().unwrap_or_else(|| "http://165.154.45.156:19000".to_string());
    let access_key = args.get(2).cloned().unwrap_or_else(|| "REDACTED_S3_ACCESS_KEY".to_string());
    let secret_key = args.get(3).cloned().unwrap_or_else(|| "REDACTED_S3_SECRET_KEY_V2".to_string());
    let bucket_name = args.get(4).cloned().unwrap_or_else(|| "user-avatar".to_string());

    println!("目标服务器: {}", endpoint);
    println!("目标桶: {}", bucket_name);
    println!("Access Key: {}", access_key);

    let config = S3Config {
        provider: S3Provider::MinIO,
        endpoint_url: endpoint.clone(),
        access_key_id: access_key,
        secret_access_key: secret_key,
        region: "us-east-1".to_string(),
        default_bucket: bucket_name.clone(),
        chat_file_preview_bucket: "chat-file-preview".to_string(),
        chat_file_origin_bucket: "chat-file-origin".to_string(),
        user_avatar_bucket: "user-avatar".to_string(),
        group_avatar_bucket: "group-avatar".to_string(),
        force_path_style: true,
        enabled: true,
        presign_expire_seconds: 3600,
        multipart_threshold: 10 * 1024 * 1024,
        multipart_chunk_size: 5 * 1024 * 1024,
        max_concurrent_uploads: 10,
    };

    println!("\n正在创建 S3 客户端...");
    let client = match S3Client::new(config).await {
        Ok(c) => {
            println!("✓ S3 客户端创建成功");
            c
        }
        Err(e) => {
            println!("✗ S3 客户端创建失败: {}", e);
            return;
        }
    };

    println!("\n正在检查桶 '{}' 是否存在...", bucket_name);
    let bucket_exists = client
        .inner
        .head_bucket()
        .bucket(&bucket_name)
        .send()
        .await
        .is_ok();

    if !bucket_exists {
        println!("  桶不存在，正在创建...");
        match client
            .inner
            .create_bucket()
            .bucket(&bucket_name)
            .send()
            .await
        {
            Ok(_) => println!("  ✓ 桶创建成功"),
            Err(e) => {
                println!("  ✗ 桶创建失败: {:?}", e);
                return;
            }
        }
    } else {
        println!("  ✓ 桶已存在");
    }

    println!("\n正在设置公开读策略...");

    let public_policy = serde_json::json!({
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

    let policy_str = serde_json::to_string(&public_policy).unwrap();
    println!("策略内容:\n{}", serde_json::to_string_pretty(&public_policy).unwrap());

    match client
        .inner
        .put_bucket_policy()
        .bucket(&bucket_name)
        .policy(&policy_str)
        .send()
        .await
    {
        Ok(_) => {
            println!("\n✓ 公开读策略设置成功！");
            println!("  现在可以通过以下方式公开访问桶中的文件:");
            println!("  {}/{}/<object-key>", endpoint.trim_end_matches('/'), bucket_name);
        }
        Err(e) => {
            println!("\n✗ 策略设置失败: {:?}", e);

            use aws_sdk_s3::error::ProvideErrorMetadata;
            let meta = e.meta();
            println!("\n错误详情:");
            println!("  错误码: {:?}", meta.code());
            println!("  错误消息: {:?}", meta.message());
        }
    }

    println!("\n验证公开访问...");
    match client.inner.list_objects_v2().bucket(&bucket_name).max_keys(3).send().await {
        Ok(result) => {
            let objects = result.contents();
            if objects.is_empty() {
                println!("  桶中没有文件");
            } else {
                for obj in objects {
                    let key = obj.key().unwrap_or_default();
                    println!("  测试文件: {}", key);
                    println!("  公开URL: {}/{}/{}", endpoint.trim_end_matches('/'), bucket_name, key);
                }
            }
        }
        Err(e) => {
            println!("列出文件失败: {:?}", e);
        }
    }

    println!("\n====== 完成 ======");
}