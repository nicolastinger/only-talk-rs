//! Object download operations module
//!
//! Provides object download and Range download functionality.

use crate::client::S3Client;
use crate::error::S3Error;

/// Download object
///
/// Downloads the complete content of an object to memory.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
///
/// # Returns
///
/// Binary data of the object
///
/// # Notes
///
/// Large file downloads consume significant memory,
/// consider using download_object_range for batch downloading
pub async fn download_object(
    client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<Vec<u8>, S3Error> {
    let result = client
        .inner
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| {
            // Handle object not found error
            if e.as_service_error().map(|se| se.is_no_such_key()).unwrap_or(false) {
                S3Error::ObjectNotFound(key.to_string())
            } else {
                S3Error::AwsError(format!("Failed to download object: {}", e))
            }
        })?;

    // Collect response stream data
    let data = result
        .body
        .collect()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to read object data: {}", e)))?;

    Ok(data.into_bytes().to_vec())
}

/// Range download object
///
/// Downloads a specified byte range of an object, supports resumable downloads.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
/// - `start`: Start byte position (inclusive)
/// - `end`: End byte position (inclusive)
///
/// # Returns
///
/// Data within the specified range
///
/// # Use Cases
///
/// - Resumable downloads: Record downloaded position, continue downloading remaining
/// - Chunked downloads: Split large files into multiple chunks for downloading
/// - Preview: Only download file header information
///
/// # HTTP Range Format
///
/// Uses "bytes=start-end" format,
/// e.g., bytes=0-1023 means download the first 1024 bytes
pub async fn download_object_range(
    client: &S3Client,
    bucket: &str,
    key: &str,
    start: i64,
    end: i64,
) -> Result<Vec<u8>, S3Error> {
    // Build Range request header
    let range = format!("bytes={}-{}", start, end);
    let result = client
        .inner
        .get_object()
        .bucket(bucket)
        .key(key)
        .range(range)
        .send()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to Range download object: {}", e)))?;

    // Collect response stream data
    let data = result
        .body
        .collect()
        .await
        .map_err(|e| S3Error::AwsError(format!("Failed to read object data: {}", e)))?;

    Ok(data.into_bytes().to_vec())
}

/// Download object to local file
///
/// Downloads an object and writes it directly to a local file.
///
/// # Parameters
///
/// - `client`: S3 client instance
/// - `bucket`: Storage bucket name
/// - `key`: Object key name
/// - `file_path`: Local file save path
///
/// # Notes
///
/// Automatically creates the required directory structure
pub async fn download_object_to_file(
    client: &S3Client,
    bucket: &str,
    key: &str,
    file_path: &str,
) -> Result<(), S3Error> {
    // Download object data
    let data = download_object(client, bucket, key).await?;

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(file_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Write file
    tokio::fs::write(file_path, &data).await?;
    Ok(())
}