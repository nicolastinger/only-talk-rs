use s3_service::config::S3Config;
use s3_service::S3Client;

#[tokio::main]
async fn main() {
    println!("====== S3 连接诊断 ======\n");

    // 1. 加载配置
    let config = S3Config::default_minio();
    println!("配置信息:");
    println!("  Provider: {:?}", config.provider);
    println!("  Endpoint: {}", config.endpoint_url);
    println!("  Access Key: {}", config.access_key_id);
    println!("  Region: {}", config.region);
    println!("  Default Bucket: {}", config.default_bucket);
    println!("  Force Path Style: {}", config.force_path_style);

    // 2. 创建客户端
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

    // 3. 测试连接 - 列出所有桶
    println!("\n正在测试连接（列出所有桶）...");
    match client.inner.list_buckets().send().await {
        Ok(result) => {
            println!("✓ 连接成功！");
            let buckets = result.buckets();
            println!("  当前桶数量: {}", buckets.len());
            for bucket in buckets {
                println!("  - {}", bucket.name().unwrap_or("unknown"));
            }
        }
        Err(e) => {
            println!("✗ 连接失败: {:?}", e);
            println!("\n可能的原因:");
            println!("  1. 网络连接问题 - 检查端点 URL 是否正确");
            println!("  2. S3 服务未启动 - 确认 MinIO 是否运行");
            println!("  3. 防火墙阻止 - 检查端口是否开放");
            println!("  4. TLS/SSL 问题 - HTTP vs HTTPS");
            return;
        }
    }

    // 4. 检查默认桶是否存在
    println!("\n正在检查默认桶 '{}'...", client.config.default_bucket);
    match client.inner.head_bucket().bucket(&client.config.default_bucket).send().await {
        Ok(_) => {
            println!("✓ 默认桶已存在");
        }
        Err(e) => {
            println!("✗ 默认桶不存在或无法访问");
            println!("  错误: {:?}", e);
            println!("\n尝试创建桶...");

            // 5. 尝试创建桶
            match client.inner.create_bucket().bucket(&client.config.default_bucket).send().await {
                Ok(_) => {
                    println!("✓ 桶创建成功");
                }
                Err(create_err) => {
                    println!("✗ 桶创建失败");
                    println!("  错误: {:?}", create_err);
                    println!("\n可能的原因:");
                    println!("  1. 权限不足 - 检查 Access Key 和 Secret Key");
                    println!("  2. 桶名已被占用");
                    println!("  3. 区域配置错误");
                    println!("  4. MinIO 版本不兼容");

                    use aws_sdk_s3::error::ProvideErrorMetadata;
                    let meta = create_err.meta();
                    println!("\n错误详情:");
                    println!("  错误码: {:?}", meta.code());
                    println!("  错误消息: {:?}", meta.message());
                }
            }
        }
    }

    // 6. 测试上传和下载
    println!("\n正在测试文件上传...");
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
            println!("✓ 文件上传成功");

            // 尝试下载
            println!("\n正在测试文件下载...");
            match client.inner.get_object()
                .bucket(&client.config.default_bucket)
                .key(test_key)
                .send()
                .await
            {
                Ok(result) => {
                    let body = result.body.collect().await.unwrap().into_bytes();
                    println!("✓ 文件下载成功，大小: {} bytes", body.len());

                    // 清理测试文件
                    let _ = client.inner.delete_object()
                        .bucket(&client.config.default_bucket)
                        .key(test_key)
                        .send()
                        .await;
                    println!("✓ 测试文件已清理");
                }
                Err(e) => {
                    println!("✗ 文件下载失败: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ 文件上传失败: {:?}", e);
        }
    }

    // 7. 测试创建聊天桶
    println!("\n正在测试创建聊天文件桶...");
    let chat_buckets = vec![
        "chat-file-preview",
        "chat-file-origin",
    ];

    for bucket_name in chat_buckets {
        println!("\n  检查桶: {}", bucket_name);

        let exists = client.inner.head_bucket().bucket(bucket_name).send().await.is_ok();
        if exists {
            println!("  ✓ 桶已存在");
            continue;
        }

        match client.inner.create_bucket().bucket(bucket_name).send().await {
            Ok(_) => println!("  ✓ 创建成功"),
            Err(e) => {
                println!("  ✗ 创建失败: {:?}", e);
            }
        }
    }

    println!("\n====== 诊断完成 ======");
}