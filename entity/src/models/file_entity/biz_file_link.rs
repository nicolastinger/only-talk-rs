use rbatis::crud;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 公开文件业务表关联
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BizFileLink {
    /// 主键ID
    pub id: Option<i64>,
    /// 业务唯一标识符
    pub biz_id: Option<Uuid>,
    /// 文件唯一标识符
    pub file_id: Option<Uuid>,
    /// 预览文件唯一标识符
    pub preview_file_id: Option<Uuid>,
    /// 是否删除
    pub is_del: Option<bool>
}

crud!(BizFileLink {});