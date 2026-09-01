//! S3 操作模块
//!
//! 本模块包含所有 S3 操作的具体实现，
//! 包括桶管理、对象上传/下载、分片上传、预签名 URL 等。
//!
//! # 模块结构
//!
//! - `bucket`: 桶操作（创建、删除、列表、CORS 配置等）
//! - `upload`: 对象上传
//! - `download`: 对象下载
//! - `delete`: 对象删除
//! - `list`: 对象列表
//! - `copy_move`: 对象复制和移动
//! - `metadata`: 对象元数据管理
//! - `multipart`: 分片上传
//! - `presigned`: 预签名 URL 生成

/// 桶操作模块
pub mod bucket;

/// 复制/移动操作模块
pub mod copy_move;

/// 删除操作模块
pub mod delete;

/// 下载操作模块
pub mod download;

/// 列表操作模块
pub mod list;

/// 元数据操作模块
pub mod metadata;

/// 分片上传操作模块
pub mod multipart;

/// 预签名 URL 操作模块
pub mod presigned;

/// 上传操作模块
pub mod upload;

// 重新导出所有公共接口
pub use bucket::*;
pub use copy_move::*;
pub use delete::*;
pub use download::*;
pub use list::*;
pub use metadata::*;
pub use multipart::*;
pub use presigned::*;
pub use upload::*;
