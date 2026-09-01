//! S3 对象存储服务库
//!
//! 本库提供对 AWS S3 及兼容存储服务（如 MinIO、阿里云 OSS）的完整封装，
//! 支持文件上传、下载、删除、列表、分片上传和预签名 URL 等核心功能。
//!
//! # 主要特性
//!
//! - **多后端支持**: 支持 S3、MinIO、阿里云 OSS 等存储服务
//! - **统一抽象接口**: 通过 `StorageBackend` trait 提供统一的存储接口
//! - **分片上传**: 自动处理大文件的分片上传
//! - **预签名 URL**: 支持生成带过期时间的临时访问 URL
//! - **本地存储**: 提供本地文件系统存储实现，用于开发和测试
//!
//! # 模块结构
//!
//! - `client`: S3 客户端封装，管理 AWS SDK 连接
//! - `config`: 配置管理，支持从配置文件加载
//! - `error`: 统一的错误类型定义
//! - `storage`: 存储后端抽象 trait 及实现（本地/S3）
//! - `operations`: 各种 S3 操作的具体实现
//!
//! # 快速开始
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

/// S3 客户端模块
pub mod client;

/// 配置模块
pub mod config;

/// 错误类型模块
pub mod error;

/// S3 操作模块
pub mod operations;

/// 存储后端模块
pub mod storage;

// 重新导出常用类型，方便使用
pub use client::{GlobalS3Client, S3Client};
pub use config::S3Config;
pub use error::S3Error;
pub use storage::{
    ObjectInfo, ObjectMetadata, PresignedMethod, S3Storage, StorageBackend, StorageError,
    StorageInfo, StorageType,
};

#[cfg(test)]
mod tests;
