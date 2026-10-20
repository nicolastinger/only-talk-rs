use std::fmt;

use async_trait::async_trait;

/// 存储类型枚举
///
/// 定义支持的存储后端类型。
/// 用于区分本地存储和 S3 云存储。
///
/// # 存储类型
///
/// - `Local`: 本地文件系统存储
/// - `S3`: S3 兼容对象存储
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum StorageType {
    /// 本地文件系统存储
    Local,
    /// S3 兼容对象存储
    S3,
}

/// Display trait 实现，提供可读的存储类型名称
impl fmt::Display for StorageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageType::Local => write!(f, "local"),
            StorageType::S3 => write!(f, "s3"),
        }
    }
}

/// 存储信息结构体
///
/// 文件上传成功后的返回信息，包含基本文件元数据。
/// 用于向上层返回上传结果。
///
/// # 字段说明
///
/// - `key`: 对象标识（S3：对象键，本地：相对路径）
/// - `size`: 文件大小（字节）
/// - `content_type`: MIME 类型
/// - `etag`: 实体标签（仅 S3，用于版本校验）
/// - `storage_type`: 存储类型
#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageInfo {
    /// 桶名称
    /// 在 S3 模式下，表示包含对象的桶
    pub bucket: Option<String>,

    /// 对象键
    /// S3 模式：对象键，本地模式：相对路径
    pub key: String,

    /// 文件大小（字节）
    pub size: i64,

    /// MIME 类型
    /// 例如 "image/jpeg"、"application/pdf"
    pub content_type: Option<String>,

    /// ETag（实体标签）
    /// S3 模式下的文件版本标识，用于一致性检查
    pub etag: Option<String>,

    /// 存储类型
    /// 标识文件是存储在本地还是 S3 上
    pub storage_type: StorageType,
}

/// 对象信息结构体
///
/// 列出桶中对象时返回的信息。
/// 包含对象的基本属性。
///
/// # 字段说明
///
/// - `key`: 对象键名
/// - `size`: 对象大小（字节）
/// - `last_modified`: 最后修改时间
/// - `etag`: 实体标签
/// - `storage_class`: 存储类别
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectInfo {
    /// 对象键（路径/文件名）
    pub key: String,

    /// 文件大小（字节）
    pub size: i64,

    /// 最后修改时间
    /// ISO 8601 格式的时间字符串
    pub last_modified: Option<String>,

    /// ETag（实体标签）
    /// 用于验证对象内容是否发生变化
    pub etag: Option<String>,

    /// 存储类别
    /// S3 存储类别，如 STANDARD、GLACIER 等
    pub storage_class: Option<String>,
}

/// 对象元数据结构体
///
/// 包含完整的对象元数据信息，
/// 包括系统元数据和用户自定义元数据。
///
/// # 元数据类型
///
/// ## 系统元数据
/// - `key`: 对象标识
/// - `size`: 对象大小
/// - `content_type`: 内容类型
/// - `last_modified`: 最后修改时间
/// - `etag`: 实体标签
///
/// ## 自定义元数据
/// - `metadata`: 用户定义的键值对
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectMetadata {
    /// 对象键
    pub key: String,

    /// 文件大小（字节）
    pub size: i64,

    /// MIME 类型
    pub content_type: Option<String>,

    /// 最后修改时间
    pub last_modified: Option<String>,

    /// ETag（实体标签）
    pub etag: Option<String>,

    /// 自定义元数据
    /// 上传时用户设置的附加元数据
    pub metadata: std::collections::HashMap<String, String>,
}

/// 预签名 URL 的 HTTP 方法
///
/// 指定预签名 URL 用于哪种操作。
///
/// # 方法说明
///
/// - `Get`: 用于下载对象
/// - `Put`: 用于上传对象
#[derive(Debug, Clone)]
pub enum PresignedMethod {
    /// GET 方法 - 用于下载
    Get,
    /// PUT 方法 - 用于上传
    Put,
}

/// 存储错误类型
///
/// 定义存储后端操作可能返回的错误。
/// 包装 S3 错误和其他类型的错误。
///
/// # 错误类型
///
/// - `S3Error`: S3 服务错误
/// - `IoError`: IO 操作错误
/// - `NotFound`: 文件或对象不存在
/// - `PermissionDenied`: 权限不足
/// - `Other`: 其他错误
#[derive(Debug)]
pub enum StorageError {
    /// S3 服务错误
    S3Error(crate::error::S3Error),

    /// IO 操作错误
    IoError(String),

    /// 文件或对象不存在
    NotFound(String),

    /// 权限不足
    PermissionDenied(String),

    /// 其他错误
    Other(String),
}

/// Display trait 实现
impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::S3Error(e) => write!(f, "S3 error: {}", e),
            StorageError::IoError(msg) => write!(f, "IO error: {}", msg),
            StorageError::NotFound(msg) => write!(f, "Not found: {}", msg),
            StorageError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            StorageError::Other(msg) => write!(f, "Storage error: {}", msg),
        }
    }
}

/// 标准 Error trait 实现
impl std::error::Error for StorageError {}

/// 从 S3Error 转换
impl From<crate::error::S3Error> for StorageError {
    fn from(err: crate::error::S3Error) -> Self {
        StorageError::S3Error(err)
    }
}

/// 从 std::io::Error 转换
impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err.to_string())
    }
}

/// 存储后端抽象 trait
///
/// 定义统一的存储操作接口，支持多种存储后端实现。
/// 所有存储服务（S3、本地等）都需要实现此 trait。
///
/// # 设计模式
///
/// 使用策略模式，通过 trait 抽象存储操作，
/// 上层代码可以透明地在不同存储后端之间切换。
///
/// # 线程安全
///
/// 实现 Send + Sync trait，支持多线程并发访问。
///
/// # 核心方法
///
/// ## 上传/下载
/// - `upload`: 上传数据
/// - `upload_stream`: 流式上传（支持大文件）
/// - `download`: 下载完整数据
/// - `download_range`: 范围下载（支持断点续传）
///
/// ## 删除操作
/// - `delete`: 删除单个对象
/// - `delete_batch`: 批量删除
///
/// ## 列表/查询
/// - `list`: 列出对象
/// - `get_metadata`: 获取元数据
///
/// ## 其他操作
/// - `copy`: 复制对象
/// - `move_object`: 移动对象
/// - `presigned_url`: 生成预签名 URL
/// - `storage_type`: 获取存储类型
///
/// # 示例
///
/// ```rust,no_run
/// use s3_service::storage::{StorageBackend, StorageInfo};
///
/// async fn upload_file<T: StorageBackend>(
///     storage: &T,
///     key: &str,
///     data: Vec<u8>,
/// ) -> Result<StorageInfo, s3_service::StorageError> {
///     storage.upload(key, data, Some("application/octet-stream")).await
/// }
/// ```
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// 将数据上传到存储后端
    ///
    /// # 参数
    ///
    /// - `key`: 对象键 / 文件路径
    /// - `data`: 要上传的数据
    /// - `content_type`: 可选的 MIME 类型
    ///
    /// # 返回值
    ///
    /// 上传后返回存储信息
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError>;

    /// 流式上传数据
    ///
    /// 用于大文件上传，支持分片上传。
    /// S3 实现会自动使用分片上传。
    ///
    /// # 参数
    ///
    /// - `key`: 对象键
    /// - `data`: 数据内容
    /// - `size`: 数据大小（字节）
    /// - `content_type`: MIME 类型
    async fn upload_stream(
        &self,
        key: &str,
        data: Vec<u8>,
        size: i64,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError>;

    /// 下载对象数据
    ///
    /// # 参数
    ///
    /// - `key`: 对象键
    ///
    /// # 返回值
    ///
    /// 对象的完整数据内容
    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    /// 使用 Range 请求头下载对象数据
    ///
    /// 支持断点续传和部分下载。
    ///
    /// # 参数
    ///
    /// - `key`: 对象键
    /// - `start`: 起始字节位置（含）
    /// - `end`: 结束字节位置（含）
    async fn download_range(
        &self,
        key: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<u8>, StorageError>;

    /// 删除对象
    ///
    /// # 参数
    ///
    /// - `key`: 要删除的对象键
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// 批量删除对象
    ///
    /// # 参数
    ///
    /// - `keys`: 要删除的对象键数组
    ///
    /// # 返回值
    ///
    /// 返回删除失败的对象列表
    async fn delete_batch(&self, keys: &[&str]) -> Result<Vec<String>, StorageError>;

    /// 列出对象
    ///
    /// # 参数
    ///
    /// - `prefix`: 可选的前缀过滤器
    /// - `max_keys`: 要返回的最大对象数量
    async fn list(
        &self,
        prefix: Option<&str>,
        max_keys: Option<i32>,
    ) -> Result<Vec<ObjectInfo>, StorageError>;

    /// 复制对象
    ///
    /// # 参数
    ///
    /// - `src`: 源对象键
    /// - `dst`: 目标对象键
    async fn copy(&self, src: &str, dst: &str) -> Result<(), StorageError>;

    /// 移动对象
    ///
    /// 通过复制 + 删除实现。
    ///
    /// # 参数
    ///
    /// - `src`: 源对象键
    /// - `dst`: 目标对象键
    async fn move_object(&self, src: &str, dst: &str) -> Result<(), StorageError>;

    /// 获取对象元数据
    ///
    /// # 参数
    ///
    /// - `key`: 对象键
    ///
    /// # 返回值
    ///
    /// 完整的对象元数据
    async fn get_metadata(&self, key: &str) -> Result<ObjectMetadata, StorageError>;

    /// 生成预签名 URL
    ///
    /// 创建在指定时间内可直接使用的临时访问 URL。
    ///
    /// # 参数
    ///
    /// - `key`: 对象键
    /// - `expires`: 过期时长
    /// - `method`: HTTP 方法（GET/PUT）
    async fn presigned_url(
        &self,
        key: &str,
        expires: std::time::Duration,
        method: PresignedMethod,
    ) -> Result<String, StorageError>;

    /// 构建公开访问 URL
    ///
    /// 适用于公开桶，直接返回无需签名的可访问 S3 URL。
    fn public_url(&self, key: &str) -> String;

    /// 获取存储类型
    ///
    /// # 返回值
    ///
    /// 当前存储后端类型
    fn storage_type(&self) -> StorageType;
}

// ==================== S3Storage S3 存储实现 ====================

use std::sync::Arc;

use crate::client::S3Client;
use crate::error::S3Error;

/// S3 对象存储实现
///
/// 提供基于 AWS S3 及兼容服务（MinIO、阿里云 OSS）的存储实现。
/// 支持完整的对象存储功能，适用于生产环境。
///
/// # 特性
///
/// - 支持大文件分片上传
/// - 支持预签名 URL
/// - 支持元数据和标签管理
/// - 支持 Range 下载（断点续传）
/// - 高可用、分布式部署
///
/// # 线程安全
///
/// 内部使用 Arc 包装的客户端，支持多线程共享。
///
/// # 示例
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use s3_service::{S3Client, S3Config};
/// use s3_service::storage::{S3Storage, StorageBackend};
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let config = S3Config::default_minio();
///     let client = Arc::new(S3Client::new(config).await?);
///     let storage = S3Storage::new(client);
///
///     let info = storage.upload("test.txt", b"hello".to_vec(), None).await?;
///     Ok(())
/// }
/// ```
pub struct S3Storage {
    /// S3 客户端实例
    /// 使用 Arc 包装以支持多线程共享
    client: Arc<S3Client>,

    /// 桶名称
    /// 所有操作都在此桶内进行
    bucket: String,
}

impl S3Storage {
    /// 创建 S3 存储实例
    ///
    /// 使用客户端配置中的默认桶
    ///
    /// # 参数
    ///
    /// - `client`: S3 客户端实例
    pub fn new(client: Arc<S3Client>) -> Self {
        let bucket = client.config.default_bucket.clone();
        S3Storage { client, bucket }
    }

    /// 使用指定桶创建 S3 存储实例
    ///
    /// # 参数
    ///
    /// - `client`: S3 客户端实例
    /// - `bucket`: 指定的桶名称
    pub fn with_bucket(client: Arc<S3Client>, bucket: String) -> Self {
        S3Storage { client, bucket }
    }
}

/// 将 AWS SDK 错误映射为 StorageError
///
/// 统一的错误类型转换函数
fn map_sdk_error(e: impl std::fmt::Display) -> StorageError {
    StorageError::S3Error(S3Error::AwsError(e.to_string()))
}

/// S3Storage 的 StorageBackend trait 实现
///
/// 实现存储后端的全部所需方法，
/// 将操作映射为 S3 API 调用。
#[async_trait]
impl StorageBackend for S3Storage {
    /// 上传对象到 S3
    ///
    /// 自动判断是否使用分片上传：
    /// - 低于阈值：直接上传
    /// - 高于阈值：分片上传
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError> {
        let size = data.len() as i64;

        // 大文件自动走分片上传流程
        if size > self.client.config.multipart_threshold {
            return self.upload_stream(key, data, size, content_type).await;
        }

        // 构建上传请求
        let mut builder = self.client.inner.put_object().bucket(&self.bucket).key(key);

        // 设置内容类型
        if let Some(ct) = content_type {
            builder = builder.content_type(ct);
        }

        // 设置上传数据
        builder = builder.body(data.into());

        // 执行上传
        let result = builder.send().await.map_err(map_sdk_error)?;

        Ok(StorageInfo {
            bucket: Some(self.bucket.clone()),
            key: key.to_string(),
            size,
            content_type: content_type.map(|s| s.to_string()),
            etag: result.e_tag().map(|s| s.to_string()),
            storage_type: StorageType::S3,
        })
    }

    /// 流式上传对象
    ///
    /// 用于大文件上传，支持分片上传
    async fn upload_stream(
        &self,
        key: &str,
        data: Vec<u8>,
        size: i64,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError> {
        // 超过阈值时使用分片上传
        if size > self.client.config.multipart_threshold {
            return crate::operations::multipart_upload(
                &self.client,
                &self.bucket,
                key,
                data,
                size,
                content_type,
            )
            .await
            .map_err(StorageError::from);
        }

        // 小文件直接上传
        self.upload(key, data, content_type).await
    }

    /// 下载对象
    ///
    /// 下载 S3 对象的完整内容
    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let result =
            self.client.inner.get_object().bucket(&self.bucket).key(key).send().await.map_err(
                |e| {
                    // 处理对象不存在错误
                    if e.as_service_error().map(|se| se.is_no_such_key()).unwrap_or(false) {
                        StorageError::NotFound(format!("Object not found: {}", key))
                    } else {
                        map_sdk_error(e)
                    }
                },
            )?;

        // 收集响应流数据
        let data = result.body.collect().await.map_err(map_sdk_error)?;
        Ok(data.into_bytes().to_vec())
    }

    /// 范围下载对象
    ///
    /// 支持断点续传和部分下载
    async fn download_range(
        &self,
        key: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<u8>, StorageError> {
        // 构建 Range 请求头
        let range = format!("bytes={}-{}", start, end);
        let result = self
            .client
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .range(range)
            .send()
            .await
            .map_err(map_sdk_error)?;

        let data = result.body.collect().await.map_err(map_sdk_error)?;
        Ok(data.into_bytes().to_vec())
    }

    /// 删除对象
    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.client
            .inner
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(map_sdk_error)?;
        Ok(())
    }

    /// 批量删除对象
    ///
    /// 调用批量删除 API，返回失败键的列表
    async fn delete_batch(&self, keys: &[&str]) -> Result<Vec<String>, StorageError> {
        let result = crate::operations::delete::delete_objects(&self.client, &self.bucket, keys)
            .await
            .map_err(StorageError::from)?;

        Ok(result.failed.into_iter().map(|e| e.key).collect())
    }

    /// 列出对象
    ///
    /// 列出桶中的对象
    async fn list(
        &self,
        prefix: Option<&str>,
        max_keys: Option<i32>,
    ) -> Result<Vec<ObjectInfo>, StorageError> {
        let result =
            crate::operations::list::list_objects(&self.client, &self.bucket, prefix, max_keys)
                .await
                .map_err(StorageError::from)?;

        Ok(result.objects)
    }

    /// 复制对象
    ///
    /// 在同一桶内复制对象
    async fn copy(&self, src: &str, dst: &str) -> Result<(), StorageError> {
        // 构建复制源：桶/键 格式
        let copy_source = format!("{}/{}", self.bucket, src);
        self.client
            .inner
            .copy_object()
            .bucket(&self.bucket)
            .key(dst)
            .copy_source(copy_source)
            .send()
            .await
            .map_err(map_sdk_error)?;
        Ok(())
    }

    /// 移动对象（复制 + 删除）
    async fn move_object(&self, src: &str, dst: &str) -> Result<(), StorageError> {
        self.copy(src, dst).await?;
        self.delete(src).await
    }

    /// 获取对象元数据
    ///
    /// 使用 HEAD 操作获取对象元数据
    async fn get_metadata(&self, key: &str) -> Result<ObjectMetadata, StorageError> {
        let result =
            self.client.inner.head_object().bucket(&self.bucket).key(key).send().await.map_err(
                |e| {
                    // 处理对象不存在错误
                    if e.as_service_error().map(|se| se.is_not_found()).unwrap_or(false) {
                        StorageError::NotFound(format!("Object not found: {}", key))
                    } else {
                        map_sdk_error(e)
                    }
                },
            )?;

        // 提取自定义元数据
        let mut metadata = std::collections::HashMap::new();
        if let Some(meta) = result.metadata() {
            for (k, v) in meta.iter() {
                metadata.insert(k.clone(), v.clone());
            }
        }

        Ok(ObjectMetadata {
            key: key.to_string(),
            size: result.content_length().unwrap_or(0),
            content_type: result.content_type().map(|s| s.to_string()),
            last_modified: result.last_modified().map(|t| t.to_string()),
            etag: result.e_tag().map(|s| s.to_string()),
            metadata,
        })
    }

    /// 生成预签名 URL
    ///
    /// 创建临时访问 URL
    async fn presigned_url(
        &self,
        key: &str,
        expires: std::time::Duration,
        method: PresignedMethod,
    ) -> Result<String, StorageError> {
        crate::operations::generate_presigned_url(&self.client, &self.bucket, key, expires, method)
            .await
            .map_err(StorageError::from)
    }

    /// 返回存储类型为 S3
    fn storage_type(&self) -> StorageType {
        StorageType::S3
    }

    fn public_url(&self, key: &str) -> String {
        let endpoint = &self.client.config.endpoint_url;
        let bucket = &self.bucket;
        if self.client.config.force_path_style {
            format!("{}/{}/{}", endpoint.trim_end_matches('/'), bucket, key)
        } else {
            // 虚拟主机风格：https://{bucket}.{endpoint_host}/{key}
            let host = endpoint
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .trim_end_matches('/');
            format!("https://{}/{}/{}", bucket, host, key)
        }
    }
}
