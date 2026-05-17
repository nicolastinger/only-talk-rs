//! MinIO 集成测试
//!
//! 测试 S3 服务与 MinIO 的集成功能
//! 运行前请确保 MinIO 服务已启动

use s3_service::{S3Client, S3Config, S3Storage, StorageBackend};
use std::sync::Arc;

/// 创建测试用的 S3 客户端
async fn create_test_client() -> S3Client {
    // 从配置文件中读取的默认 MinIO 配置
    let config = S3Config::default_minio();
    S3Client::new(config)
        .await
        .expect("创建 S3 客户端失败")
}

/// 测试初始化客户端
#[tokio::test]
async fn test_client_initialization() {
    let client = create_test_client().await;
    
    // 验证客户端配置
    assert_eq!(client.config.default_bucket, "rust-my-app");
    assert!(client.config.enabled);
    assert!(client.config.force_path_style);
}

/// 测试健康检查
#[tokio::test]
async fn test_health_check() {
    let client = create_test_client().await;
    
    let healthy = client.health_check().await.expect("健康检查失败");
    assert!(healthy, "S3 服务应该可用");
}

/// 测试确保默认存储桶存在
#[tokio::test]
async fn test_ensure_default_bucket() {
    let client = create_test_client().await;
    
    // 这个操作应该是幂等的
    client.ensure_default_bucket()
        .await
        .expect("确保默认桶失败");
}

/// 测试上传和下载文件
#[tokio::test]
async fn test_upload_and_download() {
    let client = Arc::new(create_test_client().await);
    let storage = S3Storage::new(client);
    
    let test_key = "test/integration/test_upload.txt";
    let test_data = b"Hello, MinIO! This is a test file.".to_vec();
    
    // 上传
    let upload_info = storage
        .upload(test_key, test_data.clone(), Some("text/plain"))
        .await
        .expect("上传失败");
    
    assert_eq!(upload_info.key, test_key);
    assert_eq!(upload_info.size, test_data.len() as i64);
    assert!(upload_info.etag.is_some());
    
    // 下载
    let downloaded_data = storage
        .download(test_key)
        .await
        .expect("下载失败");
    
    assert_eq!(downloaded_data, test_data);
    
    // 清理
    storage.delete(test_key).await.expect("删除失败");
}

/// 测试范围下载
#[tokio::test]
async fn test_download_range() {
    let client = Arc::new(create_test_client().await);
    let storage = S3Storage::new(client);
    
    let test_key = "test/integration/test_range.txt";
    let test_data = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_vec();
    
    // 上传
    storage
        .upload(test_key, test_data.clone(), Some("application/octet-stream"))
        .await
        .expect("上传失败");
    
    // 范围下载: 读取前10个字节
    let range_data = storage
        .download_range(test_key, 0, 9)
        .await
        .expect("范围下载失败");
    
    assert_eq!(range_data, b"0123456789");
    
    // 范围下载: 读取中间部分
    let range_data = storage
        .download_range(test_key, 10, 19)
        .await
        .expect("范围下载失败");
    
    assert_eq!(range_data, b"ABCDEFGHIJ");
    
    // 清理
    storage.delete(test_key).await.expect("删除失败");
}

/// 测试列举对象
#[tokio::test]
async fn test_list_objects() {
    let client = Arc::new(create_test_client().await);
    let storage = S3Storage::new(client);
    
    // 上传几个测试文件
    let test_files = vec![
        "test/list/file1.txt",
        "test/list/file2.txt",
        "test/list/file3.txt",
    ];
    
    for key in &test_files {
        storage
            .upload(key, b"test content".to_vec(), Some("text/plain"))
            .await
            .expect("上传失败");
    }
    
    // 列举对象
    let objects = storage
        .list(Some("test/list/"), Some(100))
        .await
        .expect("列举失败");
    
    assert!(objects.len() >= 3);
    
    // 清理
    for key in &test_files {
        storage.delete(key).await.expect("删除失败");
    }
}

/// 测试复制对象
#[tokio::test]
async fn test_copy_object() {
    let client = Arc::new(create_test_client().await);
    let storage = S3Storage::new(client);
    
    let src_key = "test/copy/source.txt";
    let dst_key = "test/copy/destination.txt";
    let test_data = b"This is the source file.".to_vec();
    
    // 上传源文件
    storage
        .upload(src_key, test_data.clone(), Some("text/plain"))
        .await
        .expect("上传失败");
    
    // 复制
    storage.copy(src_key, dst_key).await.expect("复制失败");
    
    // 验证目标文件
    let downloaded = storage.download(dst_key).await.expect("下载失败");
    assert_eq!(downloaded, test_data);
    
    // 清理
    storage.delete(src_key).await.expect("删除失败");
    storage.delete(dst_key).await.expect("删除失败");
}

/// 测试移动对象
#[tokio::test]
async fn test_move_object() {
    let client = Arc::new(create_test_client().await);
    let storage = S3Storage::new(client);
    
    let src_key = "test/move/source.txt";
    let dst_key = "test/move/destination.txt";
    let test_data = b"This file will be moved.".to_vec();
    
    // 上传源文件
    storage
        .upload(src_key, test_data.clone(), Some("text/plain"))
        .await
        .expect("上传失败");
    
    // 移动
    storage.move_object(src_key, dst_key).await.expect("移动失败");
    
    // 验证目标文件存在
    let downloaded = storage.download(dst_key).await.expect("下载失败");
    assert_eq!(downloaded, test_data);
    
    // 验证源文件已删除
    let result = storage.download(src_key).await;
    assert!(result.is_err());
    
    // 清理
    storage.delete(dst_key).await.expect("删除失败");
}

/// 测试获取元数据
#[tokio::test]
async fn test_get_metadata() {
    let client = Arc::new(create_test_client().await);
    let storage = S3Storage::new(client);
    
    let test_key = "test/metadata/file.txt";
    let test_data = b"File with metadata.".to_vec();
    
    // 上传
    storage
        .upload(test_key, test_data.clone(), Some("text/plain"))
        .await
        .expect("上传失败");
    
    // 获取元数据
    let metadata = storage.get_metadata(test_key).await.expect("获取元数据失败");
    
    assert_eq!(metadata.key, test_key);
    assert_eq!(metadata.size, test_data.len() as i64);
    assert_eq!(metadata.content_type, Some("text/plain".to_string()));
    assert!(metadata.etag.is_some());
    
    // 清理
    storage.delete(test_key).await.expect("删除失败");
}

/// 测试批量删除
#[tokio::test]
async fn test_batch_delete() {
    let client = Arc::new(create_test_client().await);
    let storage = S3Storage::new(client);
    
    // 上传多个文件
    let test_files = vec![
        "test/batch/del1.txt",
        "test/batch/del2.txt",
        "test/batch/del3.txt",
    ];
    
    for key in &test_files {
        storage
            .upload(key, b"test content".to_vec(), Some("text/plain"))
            .await
            .expect("上传失败");
    }
    
    // 批量删除
    let failed = storage
        .delete_batch(&test_files)
        .await
        .expect("批量删除失败");
    
    assert!(failed.is_empty(), "应该没有删除失败的文件");
    
    // 验证文件已删除
    for key in &test_files {
        let result = storage.download(key).await;
        assert!(result.is_err(), "文件应该已被删除: {}", key);
    }
}

/// 测试预签名 URL（GET）
#[tokio::test]
async fn test_presigned_url_get() {
    let client = Arc::new(create_test_client().await);
    let storage = S3Storage::new(client);
    
    let test_key = "test/presigned/file.txt";
    let test_data = b"File for presigned URL test.".to_vec();
    
    // 上传
    storage
        .upload(test_key, test_data.clone(), Some("text/plain"))
        .await
        .expect("上传失败");
    
    // 生成预签名 URL
    use s3_service::PresignedMethod;
    let url = storage
        .presigned_url(test_key, std::time::Duration::from_secs(3600), PresignedMethod::Get)
        .await
        .expect("生成预签名 URL 失败");
    
    assert!(url.starts_with("http"), "应该返回有效的 URL");
    assert!(url.contains(test_key), "URL 应该包含对象键");
    
    // 使用预签名 URL 下载文件（需要 HTTP 客户端）
    let response = reqwest::get(&url).await.expect("下载失败");
    assert!(response.status().is_success());
    let downloaded_data = response.bytes().await.expect("读取响应失败");
    assert_eq!(downloaded_data.to_vec(), test_data);
    
    // 清理
    storage.delete(test_key).await.expect("删除失败");
}

/// 测试大文件上传（分片上传）
#[tokio::test]
async fn test_large_file_upload() {
    let client = Arc::new(create_test_client().await);
    let storage = S3Storage::new(client);
    
    let test_key = "test/large/large_file.bin";
    // 生成 15MB 的测试数据（超过 multipart_threshold 的 10MB）
    let test_data: Vec<u8> = (0..15 * 1024 * 1024)
        .map(|i| (i % 256) as u8)
        .collect();
    
    // 使用流式上传（会触发分片上传）
    let upload_info = storage
        .upload_stream(test_key, test_data.clone(), test_data.len() as i64, Some("application/octet-stream"))
        .await
        .expect("大文件上传失败");
    
    assert_eq!(upload_info.key, test_key);
    assert_eq!(upload_info.size, test_data.len() as i64);
    
    // 下载验证
    let downloaded_data = storage.download(test_key).await.expect("下载失败");
    assert_eq!(downloaded_data.len(), test_data.len());
    assert_eq!(downloaded_data, test_data);
    
    // 清理
    storage.delete(test_key).await.expect("删除失败");
}

/// 测试删除不存在的对象
#[tokio::test]
async fn test_delete_nonexistent_object() {
    let client = Arc::new(create_test_client().await);
    let storage = S3Storage::new(client);
    
    // MinIO 和 S3 删除不存在的对象通常不会报错
    let result = storage.delete("test/nonexistent/file.txt").await;
    assert!(result.is_ok(), "删除不存在的对象应该成功");
}

/// 测试下载不存在的对象
#[tokio::test]
async fn test_download_nonexistent_object() {
    let client = Arc::new(create_test_client().await);
    let storage = S3Storage::new(client);
    
    let result = storage.download("test/nonexistent/file.txt").await;
    assert!(result.is_err(), "下载不存在的对象应该失败");
}