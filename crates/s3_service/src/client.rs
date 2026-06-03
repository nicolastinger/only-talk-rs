use std::sync::Arc;

use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::BehaviorVersion;
use aws_sdk_s3::Client as AwsS3Client;
use tracing::info;

use crate::config::S3Config;
use crate::error::S3Error;

/// S3 client wrapper struct
///
/// This struct wraps the AWS SDK's S3 client, providing a unified interface for S3 service access.
/// It holds the underlying AWS SDK client instance and configuration information.
///
/// # Fields
///
/// - `inner`: Raw AWS SDK S3 client instance
/// - `config`: S3 service configuration
///
/// # Example
///
/// ```rust,no_run
/// use s3_service::{S3Client, S3Config};
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let config = S3Config::default_minio();
///     let client = S3Client::new(config).await?;
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct S3Client {
    /// AWS S3 SDK client instance
    /// Used for actual S3 API calls
    pub inner: AwsS3Client,

    /// S3 configuration information
    /// Contains endpoint, authentication, default bucket, etc.
    pub config: S3Config,
}

impl S3Client {
    /// Create a new S3 client instance
    ///
    /// Initializes the AWS SDK S3 client based on the provided configuration.
    /// Supports MinIO, Aliyun OSS, and AWS S3 storage services.
    ///
    /// # Parameters
    ///
    /// - `config`: S3 configuration, including authentication, endpoint, region, etc.
    ///
    /// # Returns
    ///
    /// Returns the initialized S3 client instance, or error on failure
    ///
    /// # Initialization Flow
    ///
    /// 1. Set credential provider name based on provider type
    /// 2. Create AWS credentials object
    /// 3. Build S3 SDK configuration (region, credentials, path style, etc.)
    /// 4. If custom endpoint is configured, set the endpoint URL
    /// 5. Create SDK client and log the initialization
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use s3_service::{S3Client, S3Config};
    ///
    /// async fn create_client() -> Result<S3Client, s3_service::S3Error> {
    ///     let config = S3Config::default_minio();
    ///     S3Client::new(config).await
    /// }
    /// ```
    pub async fn new(config: S3Config) -> Result<Self, S3Error> {
        // Set provider name identifier based on storage provider type
        let provider_name: &'static str = match config.provider {
            crate::config::S3Provider::MinIO => "minio",
            crate::config::S3Provider::AliyunOSS => "aliyun_oss",
            crate::config::S3Provider::AwsS3 => "aws_s3",
        };

        // Create AWS credentials object
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,  // No session token
            None,  // No expiration
            provider_name,
        );

        // Build S3 SDK configuration
        let mut s3_config_builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())  // Use latest behavior version
            .region(Region::new(config.region.clone()))   // Set region
            .credentials_provider(credentials)              // Set credentials
            .force_path_style(config.force_path_style);    // Path-style access (required for MinIO)

        // If custom endpoint is configured, set the endpoint URL
        // Used for non-AWS S3 services like MinIO, Aliyun OSS
        if !config.endpoint_url.is_empty() {
            s3_config_builder = s3_config_builder
                .endpoint_url(&config.endpoint_url);
        }

        // Build configuration and create client
        let s3_config = s3_config_builder.build();
        let inner = AwsS3Client::from_conf(s3_config);

        // Log successful initialization
        info!(
            "S3 client initialized - Provider: {}, Endpoint: {}, Bucket: {}",
            config.provider, config.endpoint_url, config.default_bucket
        );

        Ok(S3Client { inner, config })
    }

    /// Get default bucket name
    ///
    /// Returns the default bucket name from configuration.
    /// The default bucket is used for operations when no bucket is specified.
    ///
    /// # Returns
    ///
    /// String reference to the default bucket name
    pub fn default_bucket(&self) -> &str {
        &self.config.default_bucket
    }

    /// S3 service health check
    ///
    /// Checks S3 service availability by attempting to list all buckets.
    /// This is a lightweight health check method.
    ///
    /// # Returns
    ///
    /// - `Ok(true)`: Service is available
    /// - `Ok(false)`: Service is unavailable
    /// - `Err`: Error during the check
    ///
    /// # Notes
    ///
    /// Even if the health check fails, the service may still be partially available (e.g., permission issues)
    pub async fn health_check(&self) -> Result<bool, S3Error> {
        match self.inner.list_buckets().send().await {
            Ok(_) => {
                info!("S3 health check passed");
                Ok(true)
            }
            Err(e) => {
                tracing::error!("S3 health check failed: {:?}", e);
                Ok(false)
            }
        }
    }

    /// Ensure default bucket exists
    ///
    /// Checks if the default bucket exists, creates it if not.
    /// This is an idempotent operation, safe to call multiple times.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, error on failure
    ///
    /// # Workflow
    ///
    /// 1. Use `head_bucket` to check if bucket exists
    /// 2. If bucket does not exist, call `create_bucket` to create it
    /// 3. Log the operation
    ///
    /// # Error Conditions
    ///
    /// - Insufficient permissions
    /// - Bucket name already in use by another account
    /// - Network connection failure
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use s3_service::S3Client;
    ///
    /// async fn setup_bucket(client: &S3Client) -> Result<(), s3_service::S3Error> {
    ///     client.ensure_default_bucket().await
    /// }
    /// ```
    pub async fn ensure_default_bucket(&self) -> Result<(), S3Error> {
        let bucket = &self.config.default_bucket;

        // Check if bucket exists
        // head_bucket success means bucket exists and we have access
        let exists = self
            .inner
            .head_bucket()
            .bucket(bucket)
            .send()
            .await
            .is_ok();

        if !exists {
            info!("default bucket {} does not exist, creating...", bucket);
            // Create bucket
            self.inner
                .create_bucket()
                .bucket(bucket)
                .send()
                .await
                .map_err(|e| S3Error::AwsError(format!("Failed to create bucket: {}", e)))?;
            info!("default bucket {} created successfully", bucket);
        } else {
            info!("default bucket {} already exists", bucket);
        }

        Ok(())
    }
}

/// Global S3 client singleton manager
///
/// Provides global singleton S3 client initialization and management functionality.
/// Uses Arc for thread-safe shared access.
///
/// # Design Pattern
///
/// Uses singleton pattern to ensure the entire application uses the same S3 client instance,
/// avoiding duplicate connection creation and improving resource utilization.
pub struct GlobalS3Client;

impl GlobalS3Client {
    /// Initialize global S3 client
    ///
    /// Creates S3 client and initializes default bucket (if enabled).
    /// Returns Arc-wrapped client instance, sharable across multiple threads.
    ///
    /// # Parameters
    ///
    /// - `config`: S3 configuration
    ///
    /// # Returns
    ///
    /// Returns Arc-wrapped S3 client instance
    ///
    /// # Initialization Flow
    ///
    /// 1. Create S3 client based on configuration
    /// 2. If S3 is enabled in config, ensure default bucket exists
    /// 3. Return Arc-wrapped client instance
    ///
    /// # Error Handling
    ///
    /// - Client creation failure returns error
    /// - Default bucket creation failure only logs warning, does not affect service startup
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use s3_service::{GlobalS3Client, S3Config};
    /// use std::sync::Arc;
    ///
    /// async fn init_s3() -> Result<Arc<s3_service::S3Client>, s3_service::S3Error> {
    ///     let config = S3Config::default_minio();
    ///     GlobalS3Client::init(config).await
    /// }
    /// ```
    pub async fn init(config: S3Config) -> Result<Arc<S3Client>, S3Error> {
        // Create S3 client instance
        let client = S3Client::new(config).await?;

        // If S3 service is enabled, ensure default bucket exists
        if client.config.enabled {
            if let Err(e) = client.ensure_default_bucket().await {
                // Bucket creation failure only logs warning, service still starts
                tracing::warn!("failed to ensure default bucket: {}, service still starts", e);
            }
        }

        // Return Arc-wrapped client, supports multi-threading sharing
        Ok(Arc::new(client))
    }
}
