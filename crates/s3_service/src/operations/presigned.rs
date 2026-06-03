//! Pre-signed URL operations module
//!
//! Provides functionality for generating temporary access URLs.

use std::time::Duration;

use crate::client::S3Client;
use crate::error::S3Error;
use crate::storage::PresignedMethod;

/// Generate pre-signed URL
///
/// Generates a temporary, signed URL that allows direct access to an object.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
/// - `expires`: URL expiration time
/// - `method`: HTTP method (GET for download, PUT for upload)
///
/// # Returns
///
/// Signed temporary URL string
///
/// # Security
///
/// - URL contains signature, cannot be forged
/// - Automatically expires after expiration time
/// - Can restrict operation type (GET/PUT)
///
/// # Use Cases
///
/// - Browser direct download/upload
/// - Share temporary access links
/// - Reduce server load (direct transfer)
///
/// # Example
///
/// ```rust,no_run
/// use std::time::Duration;
/// use s3_service::storage::PresignedMethod;
///
/// async fn example(client: &s3_service::S3Client) -> Result<String, s3_service::S3Error> {
///     // Generate download URL, expires after 1 hour
///     let url = s3_service::operations::generate_presigned_url(
///         client,
///         "my-bucket",
///         "file.pdf",
///         Duration::from_secs(3600),
///         PresignedMethod::Get
///     ).await?;
///     Ok(url)
/// }
/// ```
pub async fn generate_presigned_url(
    client: &S3Client,
    bucket: &str,
    key: &str,
    expires: Duration,
    method: PresignedMethod,
) -> Result<String, S3Error> {
    let expires_secs = expires.as_secs() as u32;

    match method {
        PresignedMethod::Get => {
            // Generate download pre-signed URL
            let builder = client
                .inner
                .get_object()
                .bucket(bucket)
                .key(key);
            
            let presigned_request = builder
                .presigned(aws_sdk_s3::presigning::PresigningConfig::expires_in(
                    Duration::from_secs(expires_secs as u64),
                ).map_err(|e| S3Error::PresignError(format!("Failed to configure presigned URL: {}", e)))?)
                .await
                .map_err(|e| S3Error::PresignError(format!("Failed to generate download presigned URL: {}", e)))?;

            Ok(presigned_request.uri().to_string())
        }
        PresignedMethod::Put => {
            // Generate upload pre-signed URL
            let builder = client
                .inner
                .put_object()
                .bucket(bucket)
                .key(key);
            
            let presigned_request = builder
                .presigned(aws_sdk_s3::presigning::PresigningConfig::expires_in(
                    Duration::from_secs(expires_secs as u64),
                ).map_err(|e| S3Error::PresignError(format!("Failed to configure presigned URL: {}", e)))?)
                .await
                .map_err(|e| S3Error::PresignError(format!("Failed to generate upload presigned URL: {}", e)))?;

            Ok(presigned_request.uri().to_string())
        }
    }
}

/// Generate download pre-signed URL
///
/// Convenience function, specifically for generating download URLs.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
/// - `expires`: Expiration time
pub async fn generate_download_presigned_url(
    client: &S3Client,
    bucket: &str,
    key: &str,
    expires: Duration,
) -> Result<String, S3Error> {
    generate_presigned_url(client, bucket, key, expires, PresignedMethod::Get).await
}

/// Generate upload pre-signed URL
///
/// Convenience function, specifically for generating upload URLs.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
/// - `expires`: Expiration time
pub async fn generate_upload_presigned_url(
    client: &S3Client,
    bucket: &str,
    key: &str,
    expires: Duration,
) -> Result<String, S3Error> {
    generate_presigned_url(client, bucket, key, expires, PresignedMethod::Put).await
}

/// Generate download pre-signed URL with default expiration
///
/// Uses the default expiration time from configuration.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
pub async fn generate_download_url_default(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<String, S3Error> {
    // Use configured default expiration time
    let expires = Duration::from_secs(client.config.presign_expire_seconds);
    generate_download_presigned_url(client, bucket, key, expires).await
}

/// Generate upload pre-signed URL with default expiration
///
/// Uses the default expiration time from configuration.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
pub async fn generate_upload_url_default(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<String, S3Error> {
    // Use configured default expiration time
    let expires = Duration::from_secs(client.config.presign_expire_seconds);
    generate_upload_presigned_url(client, bucket, key, expires).await
}