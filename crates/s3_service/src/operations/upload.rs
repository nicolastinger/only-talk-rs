//! Object upload operations module
//!
//! Provides object upload and metadata setting functionality.

use crate::client::S3Client;
use crate::error::S3Error;
use crate::storage::StorageInfo;

/// Upload object to S3
///
/// Upload data to an S3 storage bucket.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
/// - `data`: Data to upload
/// - `content_type`: Optional MIME type
///
/// # Returns
///
/// Returns storage information after successful upload
///
/// # Example
///
/// ```rust,no_run
/// async fn upload_example(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     let data = b"Hello, S3!".to_vec();
///     let info = s3_service::operations::upload_object(
///         client,
///         "my-bucket",
///         "hello.txt",
///         data,
///         Some("text/plain")
///     ).await?;
///     
///     println!("Uploaded: {} ({} bytes)", info.key, info.size);
///     Ok(())
/// }
/// ```
pub async fn upload_object(
    client: &S3Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
    content_type: Option<&str>,
) -> Result<StorageInfo, S3Error> {
    let size = data.len() as i64;

    // Build upload request
    let mut builder = client.inner.put_object().bucket(bucket).key(key);

    // Set content type
    if let Some(ct) = content_type {
        builder = builder.content_type(ct);
    }

    // Execute upload
    let result = builder
        .body(data.into())
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to upload object: {}", e)))?;

    Ok(StorageInfo {
        bucket: Some(bucket.to_string()),
        key: key.to_string(),
        size,
        content_type: content_type.map(|s| s.to_string()),
        etag: result.e_tag().map(|s| s.to_string()),
        storage_type: crate::storage::StorageType::S3,
    })
}

/// Upload object with custom metadata
///
/// In addition to uploading data, also sets custom metadata.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
/// - `data`: Data to upload
/// - `content_type`: MIME type
/// - `metadata`: Custom metadata key-value pairs
///
/// # Metadata Use Cases
///
/// - Store file description
/// - Record uploading user
/// - Save custom attributes
///
/// # Example
///
/// ```rust,no_run
/// use std::collections::HashMap;
///
/// async fn upload_with_meta(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     let mut metadata = HashMap::new();
///     metadata.insert("author".to_string(), "Alice".to_string());
///     metadata.insert("department".to_string(), "Engineering".to_string());
///     
///     let data = b"Document content".to_vec();
///     s3_service::operations::upload_object_with_metadata(
///         client,
///         "my-bucket",
///         "document.pdf",
///         data,
///         Some("application/pdf"),
///         metadata
///     ).await
/// }
/// ```
pub async fn upload_object_with_metadata(
    client: &S3Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
    content_type: Option<&str>,
    metadata: std::collections::HashMap<String, String>,
) -> Result<StorageInfo, S3Error> {
    let size = data.len() as i64;

    // Build upload request
    let mut builder = client.inner.put_object().bucket(bucket).key(key);

    // Set content type
    if let Some(ct) = content_type {
        builder = builder.content_type(ct);
    }

    // Set custom metadata
    for (k, v) in &metadata {
        builder = builder.metadata(k, v);
    }

    // Execute upload
    let result =
        builder.body(data.into()).send().await.map_err(|e| {
            S3Error::AwsError(format!("Failed to upload object with metadata: {}", e))
        })?;

    Ok(StorageInfo {
        bucket: Some(bucket.to_string()),
        key: key.to_string(),
        size,
        content_type: content_type.map(|s| s.to_string()),
        etag: result.e_tag().map(|s| s.to_string()),
        storage_type: crate::storage::StorageType::S3,
    })
}
