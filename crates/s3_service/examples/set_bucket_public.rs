use s3_service::S3Client;
use s3_service::config::S3Config;

#[tokio::main]
async fn main() {
    println!("====== 设置桶公开读策略 ======\n");

    let bucket_name = std::env::args().nth(1).unwrap_or_else(|| "user-avatar".to_string());
    println!("目标桶: {}", bucket_name);

    let config = S3Config::default_minio();
    println!("配置信息:");
    println!("  Provider: {:?}", config.provider);
    println!("  Endpoint: {}", config.endpoint_url);
    println!("  Access Key: {}", config.access_key_id);

    let endpoint_url = config.endpoint_url.clone();
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
    let bucket_exists = client.inner.head_bucket().bucket(&bucket_name).send().await.is_ok();

    if !bucket_exists {
        println!("  桶不存在，正在创建...");
        match client.inner.create_bucket().bucket(&bucket_name).send().await {
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
                "Principal": {
                    "AWS": ["*"]
                },
                "Action": ["s3:GetObject"],
                "Resource": [format!("arn:aws:s3:::{}/*", bucket_name)]
            }
        ]
    });

    let policy_str = serde_json::to_string(&public_policy).unwrap();
    println!("策略内容:\n{}", serde_json::to_string_pretty(&public_policy).unwrap());

    match client.inner.put_bucket_policy().bucket(&bucket_name).policy(&policy_str).send().await {
        Ok(_) => {
            println!("\n✓ 公开读策略设置成功！");
            println!("  现在可以通过以下方式公开访问桶中的文件:");
            println!("  {}/{}/<object-key>", endpoint_url, bucket_name);
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

    println!("\n====== 完成 ======");
}
