//! S3 Object Storage Service Library
//!
//! This library provides a complete wrapper for AWS S3 and compatible storage services
//! (such as MinIO, Aliyun OSS), supporting core functionality including file upload,
//! download, deletion, listing, multipart upload, and pre-signed URLs.
//!
//! # Key Features
//!
//! - **Multi-Backend Support**: Supports S3, MinIO, Aliyun OSS, and other storage services
//! - **Unified Abstract Interface**: Provides unified storage interface via `StorageBackend` trait
//! - **Multipart Upload**: Automatically handles multipart upload for large files
//! - **Pre-signed URLs**: Supports generating temporary access URLs with expiration
//! - **Local Storage**: Provides local filesystem storage implementation for development and testing
//!
//! # Module Structure
//!
//! - `client`: S3 client wrapper, manages AWS SDK connection
//! - `config`: Configuration management, supports loading from config files
//! - `error`: Unified error type definitions
//! - `storage`: Storage backend abstract trait and implementations (local/S3)
//! - `operations`: Concrete implementations of various S3 operations
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use s3_service::{S3Client, S3Config};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create configuration
//!     let config = S3Config::default_minio();
//!
//!     // Initialize client
//!     let client = S3Client::new(config).await?;
//!
//!     // Use client for operations...
//!     Ok(())
//! }
//! ```

/// S3 client module
pub mod client;

/// Configuration module
pub mod config;

/// Error types module
pub mod error;

/// S3 operations module
pub mod operations;

/// Storage backend module
pub mod storage;

// Re-export commonly used types for convenience
pub use client::S3Client;
pub use config::S3Config;
pub use error::S3Error;
pub use storage::{
    LocalStorage, ObjectInfo, ObjectMetadata, PresignedMethod, S3Storage, StorageBackend,
    StorageError, StorageInfo, StorageType,
};

#[cfg(test)]
mod tests;
