use serde::{Deserialize, Serialize};

/// 通知中心列表查询: keyset 分页 + 分类过滤。
#[derive(Debug, Deserialize, Serialize)]
pub struct NotificationListDTO {
    /// keyset 游标(首页整体省略); 客户端原样回传 `next_cursor`
    pub cursor: Option<NotificationCursor>,
    /// 每页条数, 服务端 clamp [1, 100], 默认 50
    pub size: Option<u32>,
    /// 已读过滤: None=全部, Some(false)=未读, Some(true)=已读
    pub is_read: Option<bool>,
    /// 分类过滤: 一级功能大类
    pub level1: Option<i32>,
    /// 分类过滤: 二级子模块
    pub level2: Option<i32>,
}

/// keyset 游标(排序键 `created_at desc, id desc`)
#[derive(Debug, Deserialize, Serialize)]
pub struct NotificationCursor {
    pub created_at: i64,
    pub id: String,
}

/// 通知列表响应
#[derive(Debug, Serialize)]
pub struct NotificationListVO {
    pub notifications: Vec<SystemNotificationVO>,
    pub has_more: bool,
    /// 下一页游标(客户端原样回传); 无更多时为 None
    pub next_cursor: Option<NotificationCursor>,
    /// 当前过滤条件下(不含分页)的未读总数
    pub unread_total: i64,
}

/// 通知 VO(与客户端 SQLite `system_notification` 表字段对齐)
#[derive(Debug, Serialize)]
pub struct SystemNotificationVO {
    pub id: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub created_at: Option<i64>,
    pub content_type: Option<u16>,
    pub user_id: Option<String>,
    pub is_read: Option<bool>,
    pub biz_id: Option<String>,
    pub level1: Option<i32>,
    pub level2: Option<i32>,
    pub level3: Option<i32>,
    pub level4: Option<i32>,
    pub unread_count: Option<i32>,
    pub priority: Option<i32>,
}

/// 未读数汇总(供角标与分类 tab)
#[derive(Debug, Serialize)]
pub struct NotificationUnreadSummaryVO {
    /// 全部未读总数
    pub total: i64,
    /// 按 (level1, level2) 分组
    pub groups: Vec<NotificationLevelCountVO>,
}

/// 单分类未读数
#[derive(Debug, Serialize)]
pub struct NotificationLevelCountVO {
    pub level1: Option<i32>,
    pub level2: Option<i32>,
    pub count: i64,
}

/// 批量已读请求体
#[derive(Debug, Deserialize, Serialize)]
pub struct MarkReadDTO {
    /// 指定要标记的通知 id; 为空表示按过滤条件全部已读
    #[serde(default)]
    pub ids: Vec<String>,
    /// 全部已读时的分类过滤: 一级功能大类
    pub level1: Option<i32>,
    /// 全部已读时的分类过滤: 二级子模块
    pub level2: Option<i32>,
}
