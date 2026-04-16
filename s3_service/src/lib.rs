//! S3对象存储服务库
//!
//! 该库提供了对AWS S3及兼容存储服务(如MinIO、阿里云OSS)的完整封装,
//! 支持文件的上传、下载、删除、列举、分片上传、预签名URL等核心功能。
//!
//! # 主要特性
//!
//! - **多存储后端支持**: 支持S3、MinIO、阿里云OSS等多种存储服务
//! - **统一抽象接口**: 通过`StorageBackend` trait提供统一的存储接口
//! - **分片上传**: 自动处理大文件的分片上传
//! - **预签名URL**: 支持生成带过期时间的临时访问URL
//! - **本地存储**: 提供本地文件系统存储实现,方便开发测试
//!
//! # 模块结构
//!
//! - `client`: S3客户端封装,管理AWS SDK连接
//! - `config`: 配置管理,支持从配置文件加载
//! - `error`: 统一的错误类型定义
//! - `storage`: 存储后端抽象trait和实现(本地/S3)
//! - `operations`: 各种S3操作的具体实现
//!
//! # 快速开始
//!
//! ```rust,no_run
//! use s3_service::{S3Client, S3Config};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 创建配置
//!     let config = S3Config::default_minio();
//!     
//!     // 初始化客户端
//!     let client = S3Client::new(config).await?;
//!     
//!     // 使用客户端进行操作...
//!     Ok(())
//! }
//! ```

/// S3客户端模块
pub mod client;

/// 配置模块
pub mod config;

/// 错误类型模块
pub mod error;

/// S3操作模块
pub mod operations;

/// 存储后端模块
pub mod storage;

// 重新导出常用类型,方便使用
pub use client::S3Client;
pub use config::S3Config;
pub use error::S3Error;
pub use storage::{
    LocalStorage, ObjectInfo, ObjectMetadata, PresignedMethod, S3Storage, StorageBackend,
    StorageError, StorageInfo, StorageType,
};

#[cfg(test)]
mod tests;
