//! Object metadata operations module
//!
//! Provides object metadata retrieval, update, and tag management functionality.

use aws_sdk_s3::primitives::DateTime;
use aws_sdk_s3::types::Tag;

use crate::client::S3Client;
use crate::error::S3Error;
use crate::storage::ObjectMetadata;

/// Get object metadata
///
/// Use HEAD operation to get object metadata, without downloading object content.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `key`: Object key name
///
/// # Returns
///
/// Complete object metadata information
///
/// # Performance
///
/// HEAD operation does not transfer object content, fast and low overhead
pub async fn head_object(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<ObjectMetadata, S3Error> {
    let result = client.inner.head_object().bucket(bucket).key(key).send().await.map_err(|e| {
        // Handle object not found error
        if e.as_service_error().map(|se| se.is_not_found()).unwrap_or(false) {
            S3Error::ObjectNotFound(key.to_string())
        } else {
            S3Error::AwsError(format!("Failed to get object metadata: {}", e))
        }
    })?;

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
        last_modified: result.last_modified().map(|t: &DateTime| t.to_string()),
        etag: result.e_tag().map(|s| s.to_string()),
        metadata,
    })
}

/// Check if object exists
///
/// Quickly check if an object exists, without getting complete metadata.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `key`: Object key name
///
/// # Returns
///
/// - `Ok(true)`: Object exists
/// - `Ok(false)`: Object does not exist
pub async fn object_exists(client: &S3Client, bucket: &str, key: &str) -> Result<bool, S3Error> {
    match head_object(client, bucket, key).await {
        Ok(_) => Ok(true),
        Err(S3Error::ObjectNotFound(_)) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Update object metadata
///
/// Update object metadata by copying itself.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `key`: Object key name
/// - `content_type`: Optional new content type
/// - `metadata`: New custom metadata
///
/// # Implementation Details
///
/// S3 does not support direct metadata modification,
/// achieved through COPY operation with metadata replacement
pub async fn update_object_metadata(
    client: &S3Client,
    bucket: &str,
    key: &str,
    content_type: Option<&str>,
    metadata: std::collections::HashMap<String, String>,
) -> Result<(), S3Error> {
    // Copy source: same object
    let copy_source = format!("{}/{}", bucket, key);

    // Build copy request
    let mut builder = client
        .inner
        .copy_object()
        .bucket(bucket)
        .key(key)
        .copy_source(copy_source)
        .metadata_directive(aws_sdk_s3::types::MetadataDirective::Replace);

    // Set content type
    if let Some(ct) = content_type {
        builder = builder.content_type(ct);
    }

    // Set custom metadata
    for (k, v) in &metadata {
        builder = builder.metadata(k, v);
    }

    // Execute update
    builder
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to update object metadata: {}", e)))?;

    Ok(())
}

/// Set object tags
///
/// Add key-value tags to an object for classification and management.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `key`: Object key name
/// - `tags`: Tag key-value pair collection
///
/// # Tag Limits
///
/// - Maximum 50 tags per object
/// - Tag key maximum 128 bytes
/// - Tag value maximum 256 bytes
pub async fn put_object_tagging(
    client: &S3Client,
    bucket: &str,
    key: &str,
    tags: std::collections::HashMap<String, String>,
) -> Result<(), S3Error> {
    // Build tag list
    let tag_set: Vec<Tag> =
        tags.into_iter().map(|(k, v)| Tag::builder().key(k).value(v).build().unwrap()).collect();

    // Build tag configuration
    let tagging = aws_sdk_s3::types::Tagging::builder()
        .set_tag_set(Some(tag_set))
        .build()
        .map_err(|e| S3Error::AwsError(format!("Failed to build tags: {}", e)))?;

    // Apply tags
    client
        .inner
        .put_object_tagging()
        .bucket(bucket)
        .key(key)
        .tagging(tagging)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to set object tags: {}", e)))?;

    Ok(())
}

/// Get object tags
///
/// Get all tags for an object.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `key`: Object key name
///
/// # Returns
///
/// Tag key-value pair collection
pub async fn get_object_tagging(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<std::collections::HashMap<String, String>, S3Error> {
    let result = client
        .inner
        .get_object_tagging()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to get object tags: {}", e)))?;

    // Convert to HashMap
    let tags = result
        .tag_set()
        .iter()
        .map(|t: &Tag| (t.key().to_string(), t.value().to_string()))
        .collect();

    Ok(tags)
}
