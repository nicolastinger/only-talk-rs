use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};
use entity::models::file_entity::biz_file_link::BizFileLink;

/// 公开文件业务表关联
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BizFileLinkVO {
    /// 业务唯一标识符
    pub biz_id: Option<Uuid>,
    /// 原文件唯一标识符
    pub origin_file_id: Option<Uuid>,
    /// 预览文件唯一标识符
    pub file_id: Option<Uuid>,
}

impl BizFileLinkVO {
    pub fn from_biz_file_link(biz_file_link: BizFileLink) -> Self {
        BizFileLinkVO {
            biz_id: biz_file_link.biz_id,
            origin_file_id: biz_file_link.origin_file_id,
            file_id: biz_file_link.file_id,
        }
    }
}