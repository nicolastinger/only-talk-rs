//! Object copy and move operations module
//!
//! Provides object copy and move functionality.

use crate::client::S3Client;
use crate::error::S3Error;

/// Copy object
///
/// Copy objects between different buckets.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `source_bucket`: Source bucket
/// - `source_key`: Source object key name
/// - `dest_bucket`: Destination bucket
/// - `dest_key`: Destination object key name
///
/// # Example
///
/// ```rust,no_run
/// async fn copy_example(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     s3_service::operations::copy_object(
///         client,
///         "source-bucket",
///         "file.txt",
///         "dest-bucket",
///         "file-copy.txt"
///     ).await
/// }
/// ```
pub async fn copy_object(
    client: &S3Client,
    source_bucket: &str,
    source_key: &str,
    dest_bucket: &str,
    dest_key: &str,
) -> Result<(), S3Error> {
    // Build copy source path: bucket/key format
    let copy_source = format!("{}/{}", source_bucket, source_key);
    client
        .inner
        .copy_object()
        .bucket(dest_bucket)
        .key(dest_key)
        .copy_source(copy_source)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to copy object: {}", e)))?;
    Ok(())
}

/// Move object
///
/// Move an object from one location to another.
/// Implementation: Copy first, then delete source object.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `source_bucket`: Source bucket
/// - `source_key`: Source object key name
/// - `dest_bucket`: Destination bucket
/// - `dest_key`: Destination object key name
///
/// # Notes
///
/// Move operation is atomic, source object is not deleted if copy fails
pub async fn move_object(
    client: &S3Client,
    source_bucket: &str,
    source_key: &str,
    dest_bucket: &str,
    dest_key: &str,
) -> Result<(), S3Error> {
    // First copy object
    copy_object(client, source_bucket, source_key, dest_bucket, dest_key).await?;

    // Only delete source when source and target differ
    if source_bucket != dest_bucket || source_key != dest_key {
        client
            .inner
            .delete_object()
            .bucket(source_bucket)
            .key(source_key)
            .send()
            .await
            .map_err(|e| S3Error::AwsError(format!("Failed to delete source object during move: {}", e)))?;
    }

    Ok(())
}

/// Copy object within same bucket
///
/// Simplified copy operation, source and destination in same bucket.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `source_key`: Source object key name
/// - `dest_key`: Destination object key name
pub async fn copy_object_same_bucket(
    client: &S3Client,
    bucket: &str,
    source_key: &str,
    dest_key: &str,
) -> Result<(), S3Error> {
    copy_object(client, bucket, source_key, bucket, dest_key).await
}

/// Move object within same bucket
///
/// Simplified move operation, source and destination in same bucket.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `source_key`: Source object key name
/// - `dest_key`: Destination object key name
pub async fn move_object_same_bucket(
    client: &S3Client,
    bucket: &str,
    source_key: &str,
    dest_key: &str,
) -> Result<(), S3Error> {
    move_object(client, bucket, source_key, bucket, dest_key).await
}