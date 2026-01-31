use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

/// 文件上传记录表
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileUploadRecord {
    /// 主键ID
    pub id: Option<i64>,
    /// 文件唯一标识符
    pub uuid: Option<Uuid>,
    /// 原始文件名
    pub original_name: Option<String>,
    /// 存储文件名
    pub stored_name: Option<String>,
    /// 文件路径
    pub file_path: Option<String>,
    /// 文件大小（字节）
    pub file_size: Option<i64>,
    /// 文件MIME类型
    pub mime_type: Option<String>,
    /// 文件哈希值（用于去重）
    pub file_hash: Option<String>,
    /// 上传用户UUID
    pub upload_user_uuid: Option<Uuid>,
    /// 上传时间（Unix时间戳，毫秒）
    pub upload_time: Option<i64>,
    /// 文件状态（0-正常，1-已删除，2-临时文件）
    pub status: Option<i32>,
    /// 文件描述
    pub description: Option<String>,
    /// 下载次数
    pub download_count: Option<i32>,
    /// 最后下载时间
    pub last_download_time: Option<i64>,
    /// 是否为OSS存储（0-否，1-是）
    pub is_oss: Option<i32>,
    /// OSS类型（0-阿里云，1-腾讯云，2-亚马逊AWS，3-其他）
    pub oss_type: Option<i32>,
}

impl FileUploadRecord {
    /// 创建空的文件记录（用于非图片类型或压缩失败的情况）
    pub fn empty() -> Self {
        FileUploadRecord {
            id: None,
            uuid: None,
            original_name: None,
            stored_name: None,
            file_path: None,
            file_size: None,
            mime_type: None,
            file_hash: None,
            upload_user_uuid: None,
            upload_time: None,
            status: None,
            description: None,
            download_count: None,
            last_download_time: None,
            is_oss: None,
            oss_type: None,
        }
    }
}

crud!(FileUploadRecord {});
impl_select!(FileUploadRecord{select_by_uuid(uuid:&Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
