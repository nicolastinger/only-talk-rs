//! Object delete operations module
//!
//! Provides single and batch object deletion functionality.

use aws_sdk_s3::types::{Error as S3DeleteError, DeletedObject};

use crate::client::S3Client;
use crate::error::S3Error as AppS3Error;

/// Delete single object
///
/// Delete the specified object from a bucket.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `key`: Object key name
///
/// # Notes
///
/// - Deleting a non-existent object does not cause an error
/// - Delete operation is irreversible
pub async fn delete_object(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<(), AppS3Error> {
    client
        .inner
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| AppS3Error::AwsError(format!("Failed to delete object: {}", e)))?;
    Ok(())
}

/// Batch delete objects
///
/// Delete multiple objects at once, more efficient than individual deletes.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `keys`: Array of object key names
///
/// # Returns
///
/// Returns delete results, including success and failure lists
///
/// # Performance
///
/// Single request can delete up to 1000 objects,
/// more than 1000 must be processed in batches
pub async fn delete_objects(
    client: &S3Client,
    bucket: &str,
    keys: &[&str],
) -> Result<DeleteBatchResult, AppS3Error> {
    // Empty list fast return
    if keys.is_empty() {
        return Ok(DeleteBatchResult {
            deleted: Vec::new(),
            failed: Vec::new(),
        });
    }

    // Build list of objects for delete request
    let objects: Vec<aws_sdk_s3::types::ObjectIdentifier> = keys
        .iter()
        .map(|k| {
            aws_sdk_s3::types::ObjectIdentifier::builder()
                .key(*k)
                .build()
                .unwrap()
        })
        .collect();

    // Build delete request
    let delete = aws_sdk_s3::types::Delete::builder()
        .set_objects(Some(objects))
        .quiet(false)  // Return delete results
        .build()
        .unwrap();

    // Execute batch delete
    let result = client
        .inner
        .delete_objects()
        .bucket(bucket)
        .delete(delete)
        .send()
        .await
        .map_err(|e| AppS3Error::AwsError(format!("Failed to batch delete objects: {}", e)))?;

    // Extract successfully deleted objects
    let deleted = result
        .deleted()
        .iter()
        .filter_map(|obj: &DeletedObject| obj.key().map(|s: &str| s.to_string()))
        .collect::<Vec<String>>();

    // Extract failed objects
    let failed = result
        .errors()
        .iter()
        .map(|e: &S3DeleteError| DeleteError {
            key: e.key().unwrap_or_default().to_string(),
            code: e.code().unwrap_or_default().to_string(),
            message: e.message().unwrap_or_default().to_string(),
        })
        .collect::<Vec<DeleteError>>();

    Ok(DeleteBatchResult { deleted, failed })
}

/// Batch delete result
///
/// Contains successfully deleted and failed object information
#[derive(Debug, serde::Serialize)]
pub struct DeleteBatchResult {
    /// Successfully deleted object keys
    pub deleted: Vec<String>,

    /// Failed objects list
    pub failed: Vec<DeleteError>,
}

/// Delete error information
///
/// Detailed information for failed object deletions
#[derive(Debug, serde::Serialize)]
pub struct DeleteError {
    /// Object key name
    pub key: String,

    /// Error code
    pub code: String,

    /// Error message
    pub message: String,
}