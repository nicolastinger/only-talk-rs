//! Object listing operations module
//!
//! Provides object listing and pagination functionality for buckets.

use aws_sdk_s3::types::{Object, ObjectStorageClass, CommonPrefix};
use aws_sdk_s3::primitives::DateTime;

use crate::client::S3Client;
use crate::error::S3Error;
use crate::storage::ObjectInfo;

/// List objects in bucket
///
/// List all objects in a bucket, supports prefix filtering and count limits.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `prefix`: Optional object key prefix filter
/// - `max_keys`: Maximum number of objects to return
///
/// # Returns
///
/// Returns object list, common prefixes, and pagination information
///
/// # Example
///
/// ```rust,no_run
/// async fn list_example(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     let result = s3_service::operations::list_objects(
///         client,
///         "my-bucket",
///         Some("photos/"),
///         Some(100)
///     ).await?;
///     
///     for obj in result.objects {
///         println!("Object: {} ({} bytes)", obj.key, obj.size);
///     }
///     Ok(())
/// }
/// ```
pub async fn list_objects(
    client: &S3Client,
    bucket: &str,
    prefix: Option<&str>,
    max_keys: Option<i32>,
) -> Result<ListObjectsResult, S3Error> {
    // Build list request
    let mut builder = client.inner.list_objects_v2().bucket(bucket);

    // Set prefix filter
    if let Some(p) = prefix {
        builder = builder.prefix(p);
    }

    // Set max keys count
    if let Some(mk) = max_keys {
        builder = builder.max_keys(mk);
    }

    // Execute list
    let result = builder
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to list objects: {}", e)))?;

    // Convert results
    let objects = convert_objects(result.contents());
    let common_prefixes = extract_common_prefixes(result.common_prefixes());

    Ok(ListObjectsResult {
        objects,
        common_prefixes,
        is_truncated: result.is_truncated().unwrap_or(false),
        next_continuation_token: result
            .next_continuation_token()
            .map(|s: &str| s.to_string()),
    })
}

/// Paginated list objects
///
/// Supports pagination token for listing operations, used for handling large numbers of objects.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `prefix`: Optional prefix filter
/// - `max_keys`: Maximum count per page
/// - `continuation_token`: Pagination token (returned from previous page)
///
/// # Use Cases
///
/// Used when object count exceeds 1000 or pagination display is needed
pub async fn list_objects_paginated(
    client: &S3Client,
    bucket: &str,
    prefix: Option<&str>,
    max_keys: Option<i32>,
    continuation_token: Option<&str>,
) -> Result<ListObjectsResult, S3Error> {
    // Build list request
    let mut builder = client.inner.list_objects_v2().bucket(bucket);

    // Set prefix
    if let Some(p) = prefix {
        builder = builder.prefix(p);
    }

    // Set max keys per page
    if let Some(mk) = max_keys {
        builder = builder.max_keys(mk);
    }

    // Set pagination token
    if let Some(token) = continuation_token {
        builder = builder.continuation_token(token);
    }

    // Execute list
    let result = builder
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to paginated list objects: {}", e)))?;

    // Convert results
    let objects = convert_objects(result.contents());
    let common_prefixes = extract_common_prefixes(result.common_prefixes());

    Ok(ListObjectsResult {
        objects,
        common_prefixes,
        is_truncated: result.is_truncated().unwrap_or(false),
        next_continuation_token: result
            .next_continuation_token()
            .map(|s: &str| s.to_string()),
    })
}

/// Convert S3 Object list to ObjectInfo list
///
/// Internal function, converts AWS SDK object structure to custom structure
pub(crate) fn convert_objects(contents: &[Object]) -> Vec<ObjectInfo> {
    contents
        .iter()
        .map(|obj| ObjectInfo {
            key: obj.key().unwrap_or_default().to_string(),
            size: obj.size().unwrap_or(0),
            last_modified: obj.last_modified().map(|t: &DateTime| t.to_string()),
            etag: obj.e_tag().map(|s: &str| s.to_string()),
            storage_class: obj.storage_class().map(|sc: &ObjectStorageClass| sc.as_str().to_string()),
        })
        .collect()
}

/// Extract common prefixes
///
/// Used to simulate directory structure, extracts common prefixes (e.g., directory names)
fn extract_common_prefixes(common_prefixes: &[CommonPrefix]) -> Vec<String> {
    common_prefixes
        .iter()
        .filter_map(|p| p.prefix().map(|s: &str| s.to_string()))
        .collect()
}

/// List objects result
///
/// Contains object list, common prefixes, and pagination information
#[derive(Debug, serde::Serialize)]
pub struct ListObjectsResult {
    /// Object list
    pub objects: Vec<ObjectInfo>,

    /// Common prefixes (used to simulate directory structure)
    pub common_prefixes: Vec<String>,

    /// Whether there are more results
    pub is_truncated: bool,

    /// Continuation token for next page
    /// Used to fetch the next page of data
    pub next_continuation_token: Option<String>,
}