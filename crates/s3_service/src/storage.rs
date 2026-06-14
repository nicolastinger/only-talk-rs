use std::fmt;

use async_trait::async_trait;

/// Storage type enum
///
/// Defines supported storage backend types.
/// Used to distinguish between local storage and S3 cloud storage.
///
/// # Storage Types
///
/// - `Local`: Local filesystem storage
/// - `S3`: S3-compatible object storage
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum StorageType {
    /// Local filesystem storage
    Local,
    /// S3-compatible object storage
    S3,
}

/// Display trait implementation, provides readable storage type name
impl fmt::Display for StorageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageType::Local => write!(f, "local"),
            StorageType::S3 => write!(f, "s3"),
        }
    }
}

/// Storage information struct
///
/// Information returned after successful file upload, containing basic file metadata.
/// Used to return upload results to upper layers.
///
/// # Field Description
///
/// - `key`: Object identifier (S3: object key, local: relative path)
/// - `size`: File size (bytes)
/// - `content_type`: MIME type
/// - `etag`: Entity tag (S3 only, for version control)
/// - `storage_type`: Storage type
#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageInfo {
    /// Bucket name
    /// In S3 mode, represents the bucket containing the object
    pub bucket: Option<String>,

    /// Object key
    /// S3 mode: object key, local mode: relative path
    pub key: String,

    /// File size (bytes)
    pub size: i64,

    /// MIME type
    /// e.g., "image/jpeg", "application/pdf"
    pub content_type: Option<String>,

    /// ETag (entity tag)
    /// File version identifier in S3 mode, used for consistency check
    pub etag: Option<String>,

    /// Storage type
    /// Identifies whether file is stored locally or on S3
    pub storage_type: StorageType,
}

/// Object information struct
///
/// Information returned when listing objects in a bucket.
/// Contains basic object attributes.
///
/// # Field Description
///
/// - `key`: Object key name
/// - `size`: Object size (bytes)
/// - `last_modified`: Last modification time
/// - `etag`: Entity tag
/// - `storage_class`: Storage class
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectInfo {
    /// Object key (path/filename)
    pub key: String,

    /// File size (bytes)
    pub size: i64,

    /// Last modification time
    /// ISO 8601 format time string
    pub last_modified: Option<String>,

    /// ETag (entity tag)
    /// Used to verify whether object content has changed
    pub etag: Option<String>,

    /// Storage class
    /// S3 storage class, such as STANDARD, GLACIER, etc.
    pub storage_class: Option<String>,
}

/// Object metadata struct
///
/// Contains complete object metadata information,
/// including system metadata and user-defined metadata.
///
/// # Metadata Types
///
/// ## System Metadata
/// - `key`: Object identifier
/// - `size`: Object size
/// - `content_type`: Content type
/// - `last_modified`: Last modification time
/// - `etag`: Entity tag
///
/// ## Custom Metadata
/// - `metadata`: User-defined key-value pairs
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectMetadata {
    /// Object key
    pub key: String,

    /// File size (bytes)
    pub size: i64,

    /// MIME type
    pub content_type: Option<String>,

    /// Last modification time
    pub last_modified: Option<String>,

    /// ETag (entity tag)
    pub etag: Option<String>,

    /// Custom metadata
    /// Additional metadata set by user during upload
    pub metadata: std::collections::HashMap<String, String>,
}

/// Pre-signed URL HTTP method
///
/// Specifies which operation the pre-signed URL is used for.
///
/// # Method Description
///
/// - `Get`: Used for downloading objects
/// - `Put`: Used for uploading objects
#[derive(Debug, Clone)]
pub enum PresignedMethod {
    /// GET method - for download
    Get,
    /// PUT method - for upload
    Put,
}

/// Storage error type
///
/// Defines errors that may be returned by storage backend operations.
/// Wraps S3 errors and other types of errors.
///
/// # Error Types
///
/// - `S3Error`: S3 service error
/// - `IoError`: IO operation error
/// - `NotFound`: File or object does not exist
/// - `PermissionDenied`: Insufficient permissions
/// - `Other`: Other error
#[derive(Debug)]
pub enum StorageError {
    /// S3 service error
    S3Error(crate::error::S3Error),

    /// IO operation error
    IoError(String),

    /// File or object does not exist
    NotFound(String),

    /// Insufficient permissions
    PermissionDenied(String),

    /// Other error
    Other(String),
}

/// Display trait implementation
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

/// Standard Error trait implementation
impl std::error::Error for StorageError {}

/// Convert from S3Error
impl From<crate::error::S3Error> for StorageError {
    fn from(err: crate::error::S3Error) -> Self {
        StorageError::S3Error(err)
    }
}

/// Convert from std::io::Error
impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err.to_string())
    }
}

/// Storage backend abstract trait
///
/// Defines unified storage operation interface, supports multiple storage backend implementations.
/// All storage services (S3, local, etc.) need to implement this trait.
///
/// # Design Pattern
///
/// Uses strategy pattern, abstracts storage operations through trait,
/// upper-level code can transparently switch between different storage backends.
///
/// # Thread Safety
///
/// Implements Send + Sync trait, supports multi-thread concurrent access.
///
/// # Core Methods
///
/// ## Upload/Download
/// - `upload`: Upload data
/// - `upload_stream`: Stream upload (supports large files)
/// - `download`: Download complete data
/// - `download_range`: Range download (supports resumable)
///
/// ## Delete Operations
/// - `delete`: Delete single object
/// - `delete_batch`: Batch delete
///
/// ## List/Query
/// - `list`: List objects
/// - `get_metadata`: Get metadata
///
/// ## Other Operations
/// - `copy`: Copy object
/// - `move_object`: Move object
/// - `presigned_url`: Generate pre-signed URL
/// - `storage_type`: Get storage type
///
/// # Example
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
    /// Upload data to storage backend
    ///
    /// # Parameters
    ///
    /// - `key`: Object key / file path
    /// - `data`: Data to upload
    /// - `content_type`: Optional MIME type
    ///
    /// # Returns
    ///
    /// Returns storage information after upload
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError>;

    /// Stream upload data
    ///
    /// Used for large file uploads, supports multipart uploads.
    /// S3 implementation will automatically use multipart uploads.
    ///
    /// # Parameters
    ///
    /// - `key`: Object key
    /// - `data`: Data content
    /// - `size`: Data size (bytes)
    /// - `content_type`: MIME type
    async fn upload_stream(
        &self,
        key: &str,
        data: Vec<u8>,
        size: i64,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError>;

    /// Download object data
    ///
    /// # Parameters
    ///
    /// - `key`: Object key
    ///
    /// # Returns
    ///
    /// Complete data content of the object
    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    /// Download object data with Range header
    ///
    /// Supports resumable and partial downloads.
    ///
    /// # Parameters
    ///
    /// - `key`: Object key
    /// - `start`: Start byte position (inclusive)
    /// - `end`: End byte position (inclusive)
    async fn download_range(
        &self,
        key: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<u8>, StorageError>;

    /// Delete object
    ///
    /// # Parameters
    ///
    /// - `key`: Object key to delete
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// Batch delete objects
    ///
    /// # Parameters
    ///
    /// - `keys`: Array of object keys to delete
    ///
    /// # Returns
    ///
    /// Returns list of objects that failed to delete
    async fn delete_batch(&self, keys: &[&str]) -> Result<Vec<String>, StorageError>;

    /// List objects
    ///
    /// # Parameters
    ///
    /// - `prefix`: Optional prefix filter
    /// - `max_keys`: Maximum number of objects to return
    async fn list(
        &self,
        prefix: Option<&str>,
        max_keys: Option<i32>,
    ) -> Result<Vec<ObjectInfo>, StorageError>;

    /// Copy object
    ///
    /// # Parameters
    ///
    /// - `src`: Source object key
    /// - `dst`: Destination object key
    async fn copy(&self, src: &str, dst: &str) -> Result<(), StorageError>;

    /// Move object
    ///
    /// Implemented via copy + delete.
    ///
    /// # Parameters
    ///
    /// - `src`: Source object key
    /// - `dst`: Destination object key
    async fn move_object(&self, src: &str, dst: &str) -> Result<(), StorageError>;

    /// Get object metadata
    ///
    /// # Parameters
    ///
    /// - `key`: Object key
    ///
    /// # Returns
    ///
    /// Complete object metadata
    async fn get_metadata(&self, key: &str) -> Result<ObjectMetadata, StorageError>;

    /// Generate pre-signed URL
    ///
    /// Creates a temporary access URL that can be used directly within a specified time.
    ///
    /// # Parameters
    ///
    /// - `key`: Object key
    /// - `expires`: Expiration duration
    /// - `method`: HTTP method (GET/PUT)
    async fn presigned_url(
        &self,
        key: &str,
        expires: std::time::Duration,
        method: PresignedMethod,
    ) -> Result<String, StorageError>;

    /// Build public access URL
    ///
    /// Suitable for public buckets, directly returns an accessible S3 URL without signature.
    fn public_url(&self, key: &str) -> String;

    /// Get storage type
    ///
    /// # Returns
    ///
    /// The current storage backend type
    fn storage_type(&self) -> StorageType;
}

// ==================== LocalStorage Local Storage Implementation ====================

use common::config_str::USER_FILE_PUBLIC_DIR;

/// Local file storage implementation
///
/// Provides file system-based storage implementation,
/// primarily used for development and testing environments.
///
/// # Features
///
/// - Simple to use, no external dependencies required
/// - High performance, local I/O is fast
/// - Does not support true pre-signed URLs (returns local path)
/// - Not suitable for production (cannot be distributed)
///
/// # Storage Path
///
/// Uses USER_FILE_PUBLIC_DIR as the base directory by default,
/// all files are stored under this directory.
///
/// # Example
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
    /// Storage base directory
    /// Root path for all files
    base_dir: String,
}

impl LocalStorage {
    /// Create local storage instance
    ///
    /// Uses the default USER_FILE_PUBLIC_DIR as base directory
    pub fn new() -> Self {
        LocalStorage { base_dir: USER_FILE_PUBLIC_DIR.to_string() }
    }

    /// Create storage instance with specified directory
    ///
    /// # Parameters
    ///
    /// - `base_dir`: Custom base directory path
    pub fn with_base_dir(base_dir: String) -> Self {
        LocalStorage { base_dir }
    }

    /// Build full file path
    ///
    /// Combines base directory and key into full path
    fn full_path(&self, key: &str) -> String {
        format!("{}/{}", self.base_dir, key)
    }
}

/// Default trait implementation
impl Default for LocalStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// StorageBackend trait implementation for LocalStorage
///
/// Implements all required methods for storage backends,
/// maps operations to local file system operations.
#[async_trait]
impl StorageBackend for LocalStorage {
    /// Upload file to local file system
    ///
    /// Writes data to local file.
    /// Automatically creates required directory structure.
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError> {
        let path = self.full_path(key);

        // Ensure parent directory exists, create if not
        if let Some(parent) = std::path::Path::new(&path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write file data
        tokio::fs::write(&path, &data).await?;

        Ok(StorageInfo {
            bucket: None, // Local storage has no bucket concept
            key: key.to_string(),
            size: data.len() as i64,
            content_type: content_type.map(|s| s.to_string()),
            etag: None, // Local storage does not support ETag
            storage_type: StorageType::Local,
        })
    }

    /// Stream upload (local storage calls upload directly)
    ///
    /// Local storage does not require streaming, writes directly
    async fn upload_stream(
        &self,
        key: &str,
        data: Vec<u8>,
        _size: i64,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError> {
        // Local storage writes directly, no distinction between regular and stream upload
        self.upload(key, data, content_type).await
    }

    /// Download file
    ///
    /// Reads full content of local file
    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let path = self.full_path(key);
        let data = tokio::fs::read(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(format!("File not found: {}", key))
            } else {
                StorageError::IoError(e.to_string())
            }
        })?;
        Ok(data)
    }

    /// Range download (reads partial file content)
    ///
    /// Supports resumable download scenarios
    async fn download_range(
        &self,
        key: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<u8>, StorageError> {
        let path = self.full_path(key);
        // Read full file
        let data = tokio::fs::read(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(format!("File not found: {}", key))
            } else {
                StorageError::IoError(e.to_string())
            }
        })?;

        // Extract data in specified range
        let start = start as usize;
        let end = std::cmp::min(end as usize, data.len());
        if start >= data.len() || start > end {
            return Ok(Vec::new());
        }
        Ok(data[start..end].to_vec())
    }

    /// Delete file
    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let path = self.full_path(key);
        tokio::fs::remove_file(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(format!("File not found: {}", key))
            } else {
                StorageError::IoError(e.to_string())
            }
        })
    }

    /// Batch delete files
    ///
    /// Returns list of files that failed to delete
    async fn delete_batch(&self, keys: &[&str]) -> Result<Vec<String>, StorageError> {
        let mut failed = Vec::new();
        for key in keys {
            if let Err(_) = self.delete(key).await {
                failed.push(key.to_string());
            }
        }
        Ok(failed)
    }

    /// List files
    ///
    /// Lists files under the base directory
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

        // Read directory contents
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

            // Get file name
            let key = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();

            // Prefix filter
            if let Some(p) = prefix {
                if !key.starts_with(p) {
                    continue;
                }
            }

            // Get file metadata
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

    /// Copy file
    async fn copy(&self, src: &str, dst: &str) -> Result<(), StorageError> {
        let src_path = self.full_path(src);
        let dst_path = self.full_path(dst);

        // Ensure target directory exists
        if let Some(parent) = std::path::Path::new(&dst_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Execute file copy
        tokio::fs::copy(&src_path, &dst_path)
            .await
            .map_err(|e| StorageError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Move file (copy + delete)
    async fn move_object(&self, src: &str, dst: &str) -> Result<(), StorageError> {
        self.copy(src, dst).await?;
        self.delete(src).await
    }

    /// Get file metadata
    async fn get_metadata(&self, key: &str) -> Result<ObjectMetadata, StorageError> {
        let path = self.full_path(key);
        let metadata = tokio::fs::metadata(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(format!("File not found: {}", key))
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

    /// Generate pre-signed URL
    ///
    /// Local storage does not support true pre-signed URLs,
    /// returns local file access path
    async fn presigned_url(
        &self,
        key: &str,
        _expires: std::time::Duration,
        _method: PresignedMethod,
    ) -> Result<String, StorageError> {
        // Local storage does not support pre-signed URLs, returns local file path
        Ok(format!("/resources/pub_file/{}", key))
    }

    /// Returns storage type as Local
    fn storage_type(&self) -> StorageType {
        StorageType::Local
    }

    fn public_url(&self, key: &str) -> String {
        format!("/resources/pub_file/{}", key)
    }
}

// ==================== S3Storage S3 Storage Implementation ====================

use std::sync::Arc;

use crate::client::S3Client;
use crate::error::S3Error;

/// S3 object storage implementation
///
/// Provides storage implementation based on AWS S3 and compatible services (MinIO, Aliyun OSS).
/// Supports complete object storage functionality, suitable for production environments.
///
/// # Features
///
/// - Supports large file multipart uploads
/// - Supports pre-signed URLs
/// - Supports metadata and tag management
/// - Supports Range downloads (resumable)
/// - High availability, distributed deployment
///
/// # Thread Safety
///
/// Uses client wrapped in Arc internally, supports multi-threaded sharing.
///
/// # Example
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
    /// S3 client instance
    /// Wrapped in Arc for multi-threaded sharing
    client: Arc<S3Client>,

    /// Bucket name
    /// All operations are performed within this bucket
    bucket: String,
}

impl S3Storage {
    /// Create S3 storage instance
    ///
    /// Uses default bucket from client configuration
    ///
    /// # Parameters
    ///
    /// - `client`: S3 client instance
    pub fn new(client: Arc<S3Client>) -> Self {
        let bucket = client.config.default_bucket.clone();
        S3Storage { client, bucket }
    }

    /// Create S3 storage instance with specified bucket
    ///
    /// # Parameters
    ///
    /// - `client`: S3 client instance
    /// - `bucket`: Specified bucket name
    pub fn with_bucket(client: Arc<S3Client>, bucket: String) -> Self {
        S3Storage { client, bucket }
    }
}

/// Map AWS SDK errors to StorageError
///
/// Unified error type conversion function
fn map_sdk_error(e: impl std::fmt::Display) -> StorageError {
    StorageError::S3Error(S3Error::AwsError(e.to_string()))
}

/// StorageBackend trait implementation for S3Storage
///
/// Implements all required methods for storage backends,
/// maps operations to S3 API calls.
#[async_trait]
impl StorageBackend for S3Storage {
    /// Upload object to S3
    ///
    /// Automatically determines whether to use multipart upload:
    /// - Below threshold: direct upload
    /// - Above threshold: multipart upload
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError> {
        let size = data.len() as i64;

        // Large files automatically use multipart upload flow
        if size > self.client.config.multipart_threshold {
            return self.upload_stream(key, data, size, content_type).await;
        }

        // Build upload request
        let mut builder = self.client.inner.put_object().bucket(&self.bucket).key(key);

        // Set content type
        if let Some(ct) = content_type {
            builder = builder.content_type(ct);
        }

        // Set upload data
        builder = builder.body(data.into());

        // Execute upload
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

    /// Stream upload object
    ///
    /// Used for large file uploads, supports multipart uploads
    async fn upload_stream(
        &self,
        key: &str,
        data: Vec<u8>,
        size: i64,
        content_type: Option<&str>,
    ) -> Result<StorageInfo, StorageError> {
        // Use multipart upload when exceeding threshold
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

        // Direct upload for small files
        self.upload(key, data, content_type).await
    }

    /// Download object
    ///
    /// Downloads full content of S3 object
    async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let result =
            self.client.inner.get_object().bucket(&self.bucket).key(key).send().await.map_err(
                |e| {
                    // Handle object not found error
                    if e.as_service_error().map(|se| se.is_no_such_key()).unwrap_or(false) {
                        StorageError::NotFound(format!("Object not found: {}", key))
                    } else {
                        map_sdk_error(e)
                    }
                },
            )?;

        // Collect response stream data
        let data = result.body.collect().await.map_err(map_sdk_error)?;
        Ok(data.into_bytes().to_vec())
    }

    /// Range download object
    ///
    /// Supports resumable and partial downloads
    async fn download_range(
        &self,
        key: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<u8>, StorageError> {
        // Build Range request header
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

    /// Delete object
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

    /// Batch delete objects
    ///
    /// Calls batch delete API, returns list of failed keys
    async fn delete_batch(&self, keys: &[&str]) -> Result<Vec<String>, StorageError> {
        let result = crate::operations::delete::delete_objects(&self.client, &self.bucket, keys)
            .await
            .map_err(StorageError::from)?;

        Ok(result.failed.into_iter().map(|e| e.key).collect())
    }

    /// List objects
    ///
    /// Lists objects in the bucket
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

    /// Copy object
    ///
    /// Copy object within the same bucket
    async fn copy(&self, src: &str, dst: &str) -> Result<(), StorageError> {
        // Build copy source: bucket/key format
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

    /// Move object (copy + delete)
    async fn move_object(&self, src: &str, dst: &str) -> Result<(), StorageError> {
        self.copy(src, dst).await?;
        self.delete(src).await
    }

    /// Get object metadata
    ///
    /// Uses HEAD operation to get object metadata
    async fn get_metadata(&self, key: &str) -> Result<ObjectMetadata, StorageError> {
        let result =
            self.client.inner.head_object().bucket(&self.bucket).key(key).send().await.map_err(
                |e| {
                    // Handle object not found error
                    if e.as_service_error().map(|se| se.is_not_found()).unwrap_or(false) {
                        StorageError::NotFound(format!("Object not found: {}", key))
                    } else {
                        map_sdk_error(e)
                    }
                },
            )?;

        // Extract custom metadata
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

    /// Generate pre-signed URL
    ///
    /// Creates temporary access URL
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

    /// Returns storage type as S3
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
            let host = endpoint
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .trim_end_matches('/');
            format!("https://{}/{}/{}", bucket, host, key)
        }
    }
}
