use rbatis::{crud, rbdc::Uuid};
use serde::{Deserialize, Serialize};

/// 系统通知
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemNotification {
    pub id: Option<Uuid>,
    /// 通知标题
    pub title: Option<String>,
    /// 详细内容
    pub content: Option<String>,
    /// 创建时间
    pub created_at: Option<i64>,
    /// 内容类型，0-纯文本，1-json, 2-xml
    pub content_type: Option<u16>,
    /// 接收人
    pub user_id: Option<Uuid>,
    /// 是否已读
    pub is_read: Option<bool>,

    /// 第一层级，用于定位功能大类
    pub level1: Option<i32>,
    /// 第二层级，用于定位子功能模块
    pub level2: Option<i32>,
    /// 第三层级，用于定位具体功能组
    pub level3: Option<i32>,
    /// 第四层级，用于定位详细功能项
    pub level4: Option<i32>,
    /// 未读数量
    pub unread_count: Option<i32>,
}

crud!(SystemNotification {});
