use std::fmt;

use async_trait::async_trait;

/// 存储类型枚举
///
/// 定义支持的存储后端类型。
/// 用于区分本地存储和S3云存储。
///
/// # 存储类型
///
/// - `Local`: 本地文件系统存储
/// - `S3`: S3兼容对象存储
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum StorageType {
    /// 本地文件系统存储
    Local,
    /// S3兼容对象存储
    S3,
}

/// Display trait实现,提供可读的存储类型名称
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
/// 文件上传成功后返回的信息,包含文件的基本元数据。
/// 用于向上层返回上传结果。
///
/// # 字段说明
///
/// - `key`: 对象标识符(S3为object key,本地为相对路径)
/// - `size`: 文件大小(字节)
/// - `content_type`: MIME类型
/// - `etag`: 实体标签(S3专用,用于版本控制)
/// - `storage_type`: 存储类型
#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageInfo {
    /// 存储桶名称
    /// S3模式下表示对象所在的桶
    pub bucket: Option<String>,
    
    /// 对象key
    /// S3模式为object key,本地模式为相对路径
    pub key: String,
    
    /// 文件大小(字节)
    pub size: i64,
    
    /// MIME类型
    /// 如: "image/jpeg", "application/pdf"
    pub content_type: Option<String>,
    
    /// ETag(实体标签)
    /// S3模式的文件版本标识,用于一致性校验
    pub etag: Option<String>,
    
    /// 存储类型
    /// 标识文件存储在本地还是S3
    pub storage_type: StorageType,
}

/// 对象信息结构体
///
/// 列举存储桶中的对象时返回的信息。
/// 包含对象的基本属性。
///
/// # 字段说明
///
/// - `key`: 对象键名
/// - `size`: 对象大小(字节)
/// - `last_modified`: 最后修改时间
/// - `etag`: 实体标签
/// - `storage_class`: 存储类型
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectInfo {
    /// 对象key(路径/文件名)
    pub key: String,
    
    /// 文件大小(字节)
    pub size: i64,
    
    /// 最后修改时间
    /// ISO 8601格式的时间字符串
    pub last_modified: Option<String>,
    
    /// ETag(实体标签)
    /// 用于验证对象内容是否改变
    pub etag: Option<String>,
    
    /// 存储类型
    /// S3存储类型,如STANDARD, GLACIER等
    pub storage_class: Option<String>,
}

/// 对象元数据结构体
///
/// 包含对象的完整元数据信息,
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
/// - `metadata`: 用户自定义的键值对
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectMetadata {
    /// 对象key
    pub key: String,
    
    /// 文件大小(字节)
    pub size: i64,
    
    /// MIME类型
    pub content_type: Option<String>,
    
    /// 最后修改时间
    pub last_modified: Option<String>,
    
    /// ETag(实体标签)
    pub etag: Option<String>,
    
    /// 自定义元数据
    /// 用户上传时设置的额外元数据
    pub metadata: std::collections::HashMap<String, String>,
}

/// 预签名URL的HTTP方法
///
/// 指定预签名URL用于哪种操作。
///
/// # 方法说明
///
/// - `Get`: 用于下载对象
/// - `Put`: 用于上传对象
#[derive(Debug, Clone)]
pub enum PresignedMethod {
    /// GET方法 - 用于下载
    Get,
    /// PUT方法 - 用于上传
    Put,
}

/// 存储错误类型
///
/// 定义存储后端操作可能返回的错误。
/// 包装了S3错误和其他类型的错误。
///
/// # 错误类型
///
/// - `S3Error`: S3服务错误
/// - `IoError`: IO操作错误
/// - `NotFound`: 文件或对象不存在
/// - `PermissionDenied`: 权限不足
/// - `Other`: 其他错误
#[derive(Debug)]
pub enum StorageError {
    /// S3服务错误
    S3Error(crate::error::S3Error),
    
    /// IO操作错误
    IoError(String),
    
    /// 文件或对象不存在
    NotFound(String),
    
    /// 权限不足
    PermissionDenied(String),
    
    /// 其他错误
    Other(String),
}

/// Display trait实现
impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::S3Error(e) => write!(f, "S3错误: {}", e),
            StorageError::IoError(msg) => write!(f, "IO错误: {}", msg),
            StorageError::NotFound(msg) => write!(f, "未找到: {}", msg),
            StorageError::PermissionDenied(msg) => write!(f, "权限不足: {}", msg),
            StorageError::Other(msg) => write!(f, "存储错误: {}", msg),
        }
    }
}

/// 实现标准Error trait
impl std::error::Error for StorageError {}

/// 从S3Error转换
impl From<crate::error::S3Error> for StorageError {
    fn from(err: crate::error::S3Error) -> Self {
        StorageError::S3Error(err)
    }
}

/// 从std::io::Error转换
impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err.to_string())
    }
}

/// 存储后端抽象trait
///
/// 定义统一的存储操作接口,支持多种存储后端实现。
/// 所有存储服务(S3、本地等)都需要实现此trait。
///
/// # 设计模式
///
/// 使用策略模式,通过trait抽象存储操作,
/// 上层代码可以透明地切换不同的存储后端。
///
/// # 线程安全
///
/// 实现了Send + Sync trait,支持多线程并发访问。
///
/// # 核心方法
///
/// ## 上传下载
/// - `upload`: 上传数据
/// - `upload_stream`: 流式上传(支持大文件)
/// - `download`: 下载完整数据
/// - `download_range`: Range下载(支持断点续传)
///
/// ## 删除操作
/// - `delete`: 删除单个对象
/// - `delete_batch`: 批量删除
///
/// ## 列举查询
/// - `list`: 列举对象
/// - `get_metadata`: 获取元数据
///
/// ## 其他操作
/// - `copy`: 复制对象
/// - `move_object`: 移动对象
/// - `presigned_url`: 生成预签名URL
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
    /// 上传数据到存储后端
    ///
    /// # 参数
    ///
    /// - `key`: 对象键名/文件路径
    /// - `data`: 要上传的数据
    /// - `content_type`: 可选的MIME类型
    ///
    /// # 返回值
    ///
    /// 返回上传后的存储信息
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError>;

    /// 流式上传数据
    ///
    /// 用于大文件上传,支持分片上传。
    /// S3实现会自动使用分片上传。
    ///
    /// # 参数
    ///
    /// - `key`: 对象键名
    /// - `data`: 数据内容
    /// - `size`: 数据大小(字节)
    /// - `content_type`: MIME类型
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
    /// - `key`: 对象键名
    ///
    /// # 返回值
    ///
    /// 对象的完整数据内容
    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    /// Range下载对象数据
    ///
    /// 支持断点续传和部分下载。
    ///
    /// # 参数
    ///
    /// - `key`: 对象键名
    /// - `start`: 起始字节位置(包含)
    /// - `end`: 结束字节位置(包含)
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
    /// - `key`: 要删除的对象键名
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// 批量删除对象
    ///
    /// # 参数
    ///
    /// - `keys`: 要删除的对象键名数组
    ///
    /// # 返回值
    ///
    /// 返回删除失败的对象列表
    async fn delete_batch(&self, keys: &[&str]) -> Result<Vec<String>, StorageError>;

    /// 列举对象
    ///
    /// # 参数
    ///
    /// - `prefix`: 可选的前缀过滤
    /// - `max_keys`: 最大返回数量
    async fn list(
        &self,
        prefix: Option<&str>,
        max_keys: Option<i32>,
    ) -> Result<Vec<ObjectInfo>, StorageError>;

    /// 复制对象
    ///
    /// # 参数
    ///
    /// - `src`: 源对象键名
    /// - `dst`: 目标对象键名
    async fn copy(&self, src: &str, dst: &str) -> Result<(), StorageError>;

    /// 移动对象
    ///
    /// 通过复制+删除实现。
    ///
    /// # 参数
    ///
    /// - `src`: 源对象键名
    /// - `dst`: 目标对象键名
    async fn move_object(&self, src: &str, dst: &str) -> Result<(), StorageError>;

    /// 获取对象元数据
    ///
    /// # 参数
    ///
    /// - `key`: 对象键名
    ///
    /// # 返回值
    ///
    /// 对象的完整元数据
    async fn get_metadata(&self, key: &str) -> Result<ObjectMetadata, StorageError>;

    /// 生成预签名URL
    ///
    /// 创建一个临时访问URL,可在指定时间内直接访问。
    ///
    /// # 参数
    ///
    /// - `key`: 对象键名
    /// - `expires`: 过期时间
    /// - `method`: HTTP方法(GET/PUT)
    async fn presigned_url(
        &self,
        key: &str,
        expires: std::time::Duration,
        method: PresignedMethod,
    ) -> Result<String, StorageError>;

    /// 构建公开访问URL
    ///
    /// 适用于公开桶，直接返回可访问的S3 URL，无需签名。
    fn public_url(&self, key: &str) -> String;

    /// 获取存储类型
    ///
    /// # 返回值
    ///
    /// 当前存储后端的类型
    fn storage_type(&self) -> StorageType;
}

// ==================== LocalStorage 本地存储实现 ====================

use common::config_str::USER_FILE_PUBLIC_DIR;

/// 本地文件存储实现
///
/// 提供基于本地文件系统的存储实现,
/// 主要用于开发和测试环境。
///
/// # 特性
///
/// - 简单易用,无需外部依赖
/// - 性能高,本地IO速度快
/// - 不支持真正的预签名URL(返回本地路径)
/// - 不适合生产环境(无法分布式部署)
///
/// # 存储路径
///
/// 默认使用USER_FILE_PUBLIC_DIR作为基础目录,
/// 所有文件都存储在此目录下。
///
/// # 示例
///
/// ```rust,no_run
/// use s3_service::storage::{LocalStorage, StorageBackend};
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let storage = LocalStorage::new();
///     let info = storage.upload("test.txt", b"hello".to_vec(), None).await?;
///     Ok(())
/// }
/// ```
pub struct LocalStorage {
    /// 存储基础目录
    /// 所有文件的根路径
    base_dir: String,
}

impl LocalStorage {
    /// 创建本地存储实例
    ///
    /// 使用默认的USER_FILE_PUBLIC_DIR作为基础目录
    pub fn new() -> Self {
        LocalStorage {
            base_dir: USER_FILE_PUBLIC_DIR.to_string(),
        }
    }

    /// 使用指定目录创建存储实例
    ///
    /// # 参数
    ///
    /// - `base_dir`: 自定义的基础目录路径
    pub fn with_base_dir(base_dir: String) -> Self {
        LocalStorage { base_dir }
    }

    /// 构建完整文件路径
    ///
    /// 将基础目录和key组合成完整路径
    fn full_path(&self, key: &str) -> String {
        format!("{}/{}", self.base_dir, key)
    }
}

/// Default trait实现
impl Default for LocalStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// LocalStorage的StorageBackend trait实现
///
/// 实现所有存储后端必需的方法,
/// 将操作映射到本地文件系统操作。
#[async_trait]
impl StorageBackend for LocalStorage {
    /// 上传文件到本地文件系统
    ///
    /// 实际上是将数据写入本地文件。
    /// 会自动创建所需的目录结构。
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError> {
        let path = self.full_path(key);
        
        // 确保父目录存在,不存在则创建
        if let Some(parent) = std::path::Path::new(&path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // 写入文件数据
        tokio::fs::write(&path, &data).await?;
        
        Ok(StorageInfo {
            bucket: None,  // 本地存储无bucket概念
            key: key.to_string(),
            size: data.len() as i64,
            content_type: content_type.map(|s| s.to_string()),
            etag: None,  // 本地存储不支持ETag
            storage_type: StorageType::Local,
        })
    }

    /// 流式上传(本地存储直接调用upload)
    ///
    /// 本地存储不需要流式处理,直接写入即可
    async fn upload_stream(
        &self,
        key: &str,
        data: Vec<u8>,
        _size: i64,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError> {
        // 本地存储直接写入,不区分普通上传和流式上传
        self.upload(key, data, content_type).await
    }

    /// 下载文件
    ///
    /// 读取本地文件的全部内容
    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let path = self.full_path(key);
        let data = tokio::fs::read(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(format!("文件不存在: {}", key))
            } else {
                StorageError::IoError(e.to_string())
            }
        })?;
        Ok(data)
    }

    /// Range下载(读取文件的部分内容)
    ///
    /// 支持断点续传场景
    async fn download_range(
        &self,
        key: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<u8>, StorageError> {
        let path = self.full_path(key);
        // 读取完整文件
        let data = tokio::fs::read(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(format!("文件不存在: {}", key))
            } else {
                StorageError::IoError(e.to_string())
            }
        })?;
        
        // 提取指定范围的数据
        let start = start as usize;
        let end = std::cmp::min(end as usize, data.len());
        if start >= data.len() || start > end {
            return Ok(Vec::new());
        }
        Ok(data[start..end].to_vec())
    }

    /// 删除文件
    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let path = self.full_path(key);
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    StorageError::NotFound(format!("文件不存在: {}", key))
                } else {
                    StorageError::IoError(e.to_string())
                }
            })
    }

    /// 批量删除文件
    ///
    /// 返回删除失败的文件列表
    async fn delete_batch(&self, keys: &[&str]) -> Result<Vec<String>, StorageError> {
        let mut failed = Vec::new();
        for key in keys {
            if let Err(_) = self.delete(key).await {
                failed.push(key.to_string());
            }
        }
        Ok(failed)
    }

    /// 列举文件
    ///
    /// 列出基础目录下的文件
    async fn list(
        &self,
        prefix: Option<&str>,
        max_keys: Option<i32>,
    ) -> Result<Vec<ObjectInfo>, StorageError> {
        let mut result = Vec::new();
        let base = std::path::Path::new(&self.base_dir);

        if !base.exists() {
            return Ok(result);
        }

        // 读取目录内容
        let mut entries = tokio::fs::read_dir(&self.base_dir).await?;
        let mut count = 0i32;
        let max = max_keys.unwrap_or(1000);

        while let Some(entry) = entries.next_entry().await? {
            if count >= max {
                break;
            }

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // 获取文件名
            let key = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // 前缀过滤
            if let Some(p) = prefix {
                if !key.starts_with(p) {
                    continue;
                }
            }

            // 获取文件元数据
            let metadata = tokio::fs::metadata(&path).await?;
            result.push(ObjectInfo {
                key,
                size: metadata.len() as i64,
                last_modified: metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs().to_string()),
                etag: None,
                storage_class: None,
            });
            count += 1;
        }

        Ok(result)
    }

    /// 复制文件
    async fn copy(&self, src: &str, dst: &str) -> Result<(), StorageError> {
        let src_path = self.full_path(src);
        let dst_path = self.full_path(dst);
        
        // 确保目标目录存在
        if let Some(parent) = std::path::Path::new(&dst_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // 执行文件复制
        tokio::fs::copy(&src_path, &dst_path)
            .await
            .map_err(|e| StorageError::IoError(e.to_string()))?;
        Ok(())
    }

    /// 移动文件(复制+删除)
    async fn move_object(&self, src: &str, dst: &str) -> Result<(), StorageError> {
        self.copy(src, dst).await?;
        self.delete(src).await
    }

    /// 获取文件元数据
    async fn get_metadata(&self, key: &str) -> Result<ObjectMetadata, StorageError> {
        let path = self.full_path(key);
        let metadata = tokio::fs::metadata(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(format!("文件不存在: {}", key))
            } else {
                StorageError::IoError(e.to_string())
            }
        })?;

        Ok(ObjectMetadata {
            key: key.to_string(),
            size: metadata.len() as i64,
            content_type: None,
            last_modified: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs().to_string()),
            etag: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    /// 生成预签名URL
    ///
    /// 本地存储不支持真正的预签名URL,
    /// 返回本地文件访问路径
    async fn presigned_url(
        &self,
        key: &str,
        _expires: std::time::Duration,
        _method: PresignedMethod,
    ) -> Result<String, StorageError> {
        // 本地存储不支持预签名URL,返回本地文件路径
        Ok(format!("/resources/pub_file/{}", key))
    }

    /// 返回存储类型为Local
    fn storage_type(&self) -> StorageType {
        StorageType::Local
    }

    fn public_url(&self, key: &str) -> String {
        format!("/resources/pub_file/{}", key)
    }
}

// ==================== S3Storage S3存储实现 ====================

use std::sync::Arc;

use crate::client::S3Client;
use crate::error::S3Error;

/// S3对象存储实现
///
/// 提供基于AWS S3及兼容服务(MinIO、阿里云OSS)的存储实现。
/// 支持完整的对象存储功能,适合生产环境使用。
///
/// # 特性
///
/// - 支持大文件分片上传
/// - 支持预签名URL
/// - 支持元数据和标签管理
/// - 支持Range下载(断点续传)
/// - 高可用、分布式部署
///
/// # 线程安全
///
/// 内部使用Arc包装的客户端,支持多线程共享。
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
    /// S3客户端实例
    /// Arc包装,支持多线程共享
    client: Arc<S3Client>,
    
    /// 存储桶名称
    /// 所有操作都在此桶中进行
    bucket: String,
}

impl S3Storage {
    /// 创建S3存储实例
    ///
    /// 使用客户端配置中的默认桶
    ///
    /// # 参数
    ///
    /// - `client`: S3客户端实例
    pub fn new(client: Arc<S3Client>) -> Self {
        let bucket = client.config.default_bucket.clone();
        S3Storage { client, bucket }
    }

    /// 使用指定桶创建S3存储实例
    ///
    /// # 参数
    ///
    /// - `client`: S3客户端实例
    /// - `bucket`: 指定的存储桶名称
    pub fn with_bucket(client: Arc<S3Client>, bucket: String) -> Self {
        S3Storage { client, bucket }
    }
}

/// 将AWS SDK错误映射为StorageError
///
/// 统一错误类型转换函数
fn map_sdk_error(e: impl std::fmt::Display) -> StorageError {
    StorageError::S3Error(S3Error::AwsError(e.to_string()))
}

/// S3Storage的StorageBackend trait实现
///
/// 实现所有存储后端必需的方法,
/// 将操作映射到S3 API调用。
#[async_trait]
impl StorageBackend for S3Storage {
    /// 上传对象到S3
    ///
    /// 自动判断是否使用分片上传:
    /// - 小于阈值: 直接上传
    /// - 大于阈值: 使用分片上传
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
        let mut builder = self
            .client
            .inner
            .put_object()
            .bucket(&self.bucket)
            .key(key);

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
    /// 用于大文件上传,支持分片上传
    async fn upload_stream(
        &self,
        key: &str,
        data: Vec<u8>,
        size: i64,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError> {
        // 超过分片阈值使用分片上传
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
    /// 下载S3对象的完整内容
    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let result = self
            .client
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                // 处理对象不存在的错误
                if e.as_service_error().map(|se| se.is_no_such_key()).unwrap_or(false) {
                    StorageError::NotFound(format!("对象不存在: {}", key))
                } else {
                    map_sdk_error(e)
                }
            })?;

        // 收集响应流数据
        let data = result.body.collect().await.map_err(map_sdk_error)?;
        Ok(data.into_bytes().to_vec())
    }

    /// Range下载对象
    ///
    /// 支持断点续传和部分下载
    async fn download_range(
        &self,
        key: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<u8>, StorageError> {
        // 构建Range请求头
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
    /// 调用批量删除API,返回失败的key列表
    async fn delete_batch(&self, keys: &[&str]) -> Result<Vec<String>, StorageError> {
        let result = crate::operations::delete::delete_objects(
            &self.client, &self.bucket, keys,
        )
        .await
        .map_err(StorageError::from)?;

        Ok(result.failed.into_iter().map(|e| e.key).collect())
    }

    /// 列举对象
    ///
    /// 列出存储桶中的对象
    async fn list(
        &self,
        prefix: Option<&str>,
        max_keys: Option<i32>,
    ) -> Result<Vec<ObjectInfo>, StorageError> {
        let result = crate::operations::list::list_objects(
            &self.client, &self.bucket, prefix, max_keys,
        )
        .await
        .map_err(StorageError::from)?;

        Ok(result.objects)
    }

    /// 复制对象
    ///
    /// 在同一桶内复制对象
    async fn copy(&self, src: &str, dst: &str) -> Result<(), StorageError> {
        // 构建复制源: bucket/key格式
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

    /// 移动对象(复制+删除)
    async fn move_object(&self, src: &str, dst: &str) -> Result<(), StorageError> {
        self.copy(src, dst).await?;
        self.delete(src).await
    }

    /// 获取对象元数据
    ///
    /// 使用HEAD操作获取对象元数据
    async fn get_metadata(&self, key: &str) -> Result<ObjectMetadata, StorageError> {
        let result = self
            .client
            .inner
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                // 处理对象不存在的错误
                if e.as_service_error()
                    .map(|se| se.is_not_found())
                    .unwrap_or(false)
                {
                    StorageError::NotFound(format!("对象不存在: {}", key))
                } else {
                    map_sdk_error(e)
                }
            })?;

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

    /// 生成预签名URL
    ///
    /// 创建临时访问URL
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

    /// 返回存储类型为S3
    fn storage_type(&self) -> StorageType {
        StorageType::S3
    }

    fn public_url(&self, key: &str) -> String {
        let endpoint = &self.client.config.endpoint_url;
        let bucket = &self.bucket;
        if self.client.config.force_path_style {
            format!("{}/{}/{}", endpoint.trim_end_matches('/'), bucket, key)
        } else {
            // virtual-hosted style: https://{bucket}.{endpoint_host}/{key}
            let host = endpoint.trim_start_matches("http://").trim_start_matches("https://").trim_end_matches('/');
            format!("https://{}/{}/{}", bucket, host, key)
        }
    }
}
