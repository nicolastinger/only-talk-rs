use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select, impl_update};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Announcement {
    /// 公告主键 (uuid)
    pub uuid: Option<Uuid>,
    /// 公告标题
    pub title: Option<String>,
    /// 公告内容 (markdown/html)
    pub content: Option<String>,
    /// 内容类型: 0-markdown, 1-html
    pub content_type: Option<i16>,
    /// 展示开始时间 (Unix 时间戳，秒)
    pub start_at: Option<i64>,
    /// 展示结束时间 (Unix 时间戳，秒)
    pub end_at: Option<i64>,
    /// 是否启用
    pub is_active: Option<bool>,
    /// 排序权重(越小越靠前)
    pub sort_order: Option<i16>,
    /// 是否删除
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
    /// 更新时间 (Unix 时间戳，秒)
    pub updated_at: Option<i64>,
}

crud!(Announcement {});

impl_select!(Announcement {select_by_uuid(uuid:&Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
impl_update!(Announcement {update_by_uuid(uuid:&Uuid) => "`where uuid = #{uuid}`"});
