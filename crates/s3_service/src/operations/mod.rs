//! S3 Operations Module
//!
//! This module contains concrete implementations of all S3 operations,
//! including bucket management, object upload/download, multipart upload, pre-signed URLs, etc.
//!
//! # Module Structure
//!
//! - `bucket`: Bucket operations (create, delete, list, CORS configuration, etc.)
//! - `upload`: Object upload
//! - `download`: Object download
//! - `delete`: Object deletion
//! - `list`: Object listing
//! - `copy_move`: Object copy and move
//! - `metadata`: Object metadata management
//! - `multipart`: Multipart upload
//! - `presigned`: Pre-signed URL generation

/// Bucket operations module
pub mod bucket;

/// Copy/move operations module
pub mod copy_move;

/// Delete operations module
pub mod delete;

/// Download operations module
pub mod download;

/// List operations module
pub mod list;

/// Metadata operations module
pub mod metadata;

/// Multipart upload operations module
pub mod multipart;

/// Pre-signed URL operations module
pub mod presigned;

/// Upload operations module
pub mod upload;

// Re-export all public interfaces
pub use bucket::*;
pub use copy_move::*;
pub use delete::*;
pub use download::*;
pub use list::*;
pub use metadata::*;
pub use multipart::*;
pub use presigned::*;
pub use upload::*;
