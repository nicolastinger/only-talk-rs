use s3_service::S3Client;
use s3_service::config::S3Config;

#[tokio::main]
async fn main() {
    let bucket_name = std::env::args().nth(1).unwrap_or_else(|| "user-avatar".to_string());
    let prefix = std::env::args().nth(2).unwrap_or_default();

    let config = S3Config::default_minio();
    let client = S3Client::new(config).await.unwrap();

    println!("列出桶 '{}' 中前缀 '{}' 的文件...\n", bucket_name, prefix);

    let mut continuation_token = None;
    let mut count = 0;

    loop {
        let mut request = client.inner.list_objects_v2().bucket(&bucket_name).max_keys(1000);

        if !prefix.is_empty() {
            request = request.prefix(&prefix);
        }

        if let Some(token) = &continuation_token {
            request = request.continuation_token(token);
        }

        let result = request.send().await.unwrap();

        for obj in result.contents() {
            count += 1;
            println!("  {}", obj.key().unwrap_or_default());
        }

        if result.is_truncated().unwrap_or(false) {
            continuation_token = result.next_continuation_token().map(|s| s.to_string());
        } else {
            break;
        }
    }

    println!("\n总计: {} 个文件", count);
}
