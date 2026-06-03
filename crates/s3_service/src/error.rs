use std::fmt;

/// Unified S3 service error types
///
/// Defines all possible error types that the S3 service may return,
/// providing a unified error handling interface.
///
/// # Error Types
///
/// - `AwsError`: AWS SDK底层-level errors
/// - `ConfigError`: Configuration-related errors
/// - `BucketNotFound`: Bucket does not exist
/// - `ObjectNotFound`: Object does not exist
/// - `PermissionDenied`: Insufficient permissions
/// - `PresignError`: Pre-signed URL generation failed
/// - `MultipartError`: Multipart upload errors
/// - `IoError`: IO operation errors
/// - `Other`: Other uncategorized errors
///
/// # Error Handling Example
///
/// ```rust,no_run
/// use s3_service::S3Error;
///
/// fn handle_error(e: S3Error) {
///     match e {
///         S3Error::BucketNotFound(bucket) => {
///             eprintln!("Bucket {} does not exist", bucket);
///         }
///         S3Error::ObjectNotFound(key) => {
///             eprintln!("Object {} does not exist", key);
///         }
///         _ => eprintln!("Other error: {}", e),
///     }
/// }
/// ```
#[derive(Debug)]
pub enum S3Error {
    /// AWS SDK error
    /// Returned when underlying SDK call fails
    AwsError(String),

    /// Configuration error
    /// Missing, malformed, or invalid configuration
    ConfigError(String),

    /// Bucket not found
    /// Returned when accessing non-existent bucket
    BucketNotFound(String),

    /// Object not found
    /// Returned when accessing non-existent object
    ObjectNotFound(String),

    /// Permission denied
    /// Insufficient permissions to perform operation
    PermissionDenied(String),

    /// Pre-signed URL error
    /// Failed to create temporary access URL
    PresignError(String),

    /// Multipart upload error
    /// Errors during multipart upload process
    MultipartError(String),

    /// IO error
    /// Failed file read/write or other IO operation
    IoError(String),

    /// Other error
    /// Uncategorized error type
    Other(String),
}

/// Display trait implementation, provides friendly error messages
impl fmt::Display for S3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            S3Error::AwsError(msg) => write!(f, "AWS SDK error: {}", msg),
            S3Error::ConfigError(msg) => write!(f, "S3 config error: {}", msg),
            S3Error::BucketNotFound(bucket) => write!(f, "Bucket not found: {}", bucket),
            S3Error::ObjectNotFound(key) => write!(f, "Object not found: {}", key),
            S3Error::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            S3Error::PresignError(msg) => write!(f, "Pre-signed URL generation failed: {}", msg),
            S3Error::MultipartError(msg) => write!(f, "Multipart upload error: {}", msg),
            S3Error::IoError(msg) => write!(f, "IO error: {}", msg),
            S3Error::Other(msg) => write!(f, "S3 error: {}", msg),
        }
    }
}

/// Standard Error trait implementation
impl std::error::Error for S3Error {}

/// Convert from std::io::Error
///
/// Allows direct use of ? operator to convert IO errors to S3Error
impl From<std::io::Error> for S3Error {
    fn from(err: std::io::Error) -> Self {
        S3Error::IoError(err.to_string())
    }
}

/// Convert from anyhow::Error
///
/// Allows conversion of anyhow errors to S3Error
impl From<anyhow::Error> for S3Error {
    fn from(err: anyhow::Error) -> Self {
        S3Error::Other(err.to_string())
    }
}
