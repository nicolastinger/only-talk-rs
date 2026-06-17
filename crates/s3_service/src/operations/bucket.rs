//! Bucket operations module
//!
//! Provides bucket creation, deletion, listing, permission configuration, etc.

use aws_sdk_s3::primitives::DateTime;
use aws_sdk_s3::types::Bucket;

use crate::client::S3Client;
use crate::error::S3Error;

/// Create bucket
///
/// Create a new S3 bucket.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
///
/// # Notes
///
/// - Bucket name must be globally unique
/// - Bucket name must comply with S3 naming conventions
/// - Different regions may have different restrictions
///
/// # Example
///
/// ```rust,no_run
/// use s3_service::operations::create_bucket;
///
/// async fn example(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     create_bucket(client, "my-new-bucket").await
/// }
/// ```
pub async fn create_bucket(client: &S3Client, bucket: &str) -> Result<(), S3Error> {
    client
        .inner
        .create_bucket()
        .bucket(bucket)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to create bucket: {}", e)))?;
    Ok(())
}

/// Delete bucket
///
/// Delete an empty S3 bucket.
/// The bucket must be empty to be deleted, otherwise it will fail.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name to delete
///
/// # Errors
///
/// - Bucket not empty
/// - Bucket does not exist
/// - Insufficient permissions
pub async fn delete_bucket(client: &S3Client, bucket: &str) -> Result<(), S3Error> {
    client
        .inner
        .delete_bucket()
        .bucket(bucket)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to delete bucket: {}", e)))?;
    Ok(())
}

/// List all buckets
///
/// List all buckets under the current account.
///
/// # Parameters
///
/// - `client`: S3 client instance
///
/// # Returns
///
/// Returns a list of bucket information
pub async fn list_buckets(client: &S3Client) -> Result<Vec<BucketInfo>, S3Error> {
    let result = client
        .inner
        .list_buckets()
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to list buckets: {}", e)))?;

    // Convert to custom BucketInfo struct
    let buckets = result
        .buckets()
        .iter()
        .map(|b: &Bucket| BucketInfo {
            name: b.name().unwrap_or_default().to_string(),
            created: b.creation_date().map(|d: &DateTime| d.to_string()),
        })
        .collect();

    Ok(buckets)
}

/// Check if bucket exists
///
/// Use HEAD operation to check if a bucket exists.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
///
/// # Returns
///
/// - `Ok(true)`: Bucket exists
/// - `Ok(false)`: Bucket does not exist
pub async fn bucket_exists(client: &S3Client, bucket: &str) -> Result<bool, S3Error> {
    match client.inner.head_bucket().bucket(bucket).send().await {
        Ok(_) => Ok(true),
        Err(e) => {
            // 404 means bucket does not exist
            if e.as_service_error().map(|se| se.is_not_found()).unwrap_or(false) {
                Ok(false)
            } else {
                Err(S3Error::AwsError(format!("Failed to check bucket: {}", e)))
            }
        }
    }
}

/// Set bucket CORS configuration
///
/// Configure cross-origin resource sharing rules to allow browser cross-origin access.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `allowed_origins`: List of allowed origins (e.g., ["*"] or ["https://example.com"])
/// - `allowed_methods`: Allowed HTTP methods (e.g., ["GET", "PUT"])
/// - `allowed_headers`: Allowed request headers
/// - `max_age_seconds`: Preflight request cache time (seconds)
///
/// # Example
///
/// ```rust,no_run
/// async fn setup_cors(client: &s3_service::S3Client) -> Result<(), s3_service::S3Error> {
///     s3_service::operations::put_bucket_cors(
///         client,
///         "my-bucket",
///         vec!["*".to_string()],
///         vec!["GET".to_string(), "PUT".to_string()],
///         vec!["*".to_string()],
///         Some(3600),
///     ).await
/// }
/// ```
pub async fn put_bucket_cors(
    client: &S3Client,
    bucket: &str,
    allowed_origins: Vec<String>,
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>,
    max_age_seconds: Option<i32>,
) -> Result<(), S3Error> {
    // Build CORS rule
    let rule = aws_sdk_s3::types::CorsRule::builder()
        .set_allowed_origins(Some(allowed_origins))
        .set_allowed_methods(Some(allowed_methods))
        .set_allowed_headers(Some(allowed_headers))
        .max_age_seconds(max_age_seconds.unwrap_or(3600))
        .build()
        .map_err(|e| S3Error::AwsError(format!("Failed to build CORS rule: {}", e)))?;

    // Build CORS configuration
    let cors_config = aws_sdk_s3::types::CorsConfiguration::builder()
        .cors_rules(rule)
        .build()
        .map_err(|e| S3Error::AwsError(format!("Failed to build CORS configuration: {}", e)))?;

    // Apply configuration
    client
        .inner
        .put_bucket_cors()
        .bucket(bucket)
        .cors_configuration(cors_config)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to set CORS: {}", e)))?;

    Ok(())
}

/// Set bucket access policy
///
/// Set the IAM access policy for a bucket to control access permissions.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
/// - `policy`: JSON-formatted policy string
pub async fn put_bucket_policy(
    client: &S3Client,
    bucket: &str,
    policy: &str,
) -> Result<(), S3Error> {
    client
        .inner
        .put_bucket_policy()
        .bucket(bucket)
        .policy(policy)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to set bucket policy: {}", e)))?;
    Ok(())
}

/// Get bucket access policy
///
/// Get the current IAM access policy for a bucket.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Bucket name
///
/// # Returns
///
/// JSON-formatted policy string
pub async fn get_bucket_policy(client: &S3Client, bucket: &str) -> Result<String, S3Error> {
    let result = client
        .inner
        .get_bucket_policy()
        .bucket(bucket)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to get bucket policy: {}", e)))?;

    Ok(result.policy().unwrap_or_default().to_string())
}

/// Bucket information struct
///
/// Basic bucket information, returned when listing buckets.
#[derive(Debug, serde::Serialize)]
pub struct BucketInfo {
    /// Bucket name
    pub name: String,

    /// Creation time
    pub created: Option<String>,
}
