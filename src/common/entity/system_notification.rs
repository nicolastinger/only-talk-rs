use rbatis::crud;
use serde::{Deserialize, Serialize};

/// 系统通知, 定时清理
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemNotification {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub content_type: u16, // 内容类型，0-纯文本，1-json, 2-xml
    pub is_read: bool,  // 是否已读
    
    /// 第一层级，用于定位功能大类
    pub level1: i32,
    /// 第二层级，用于定位子功能模块
    pub level2: i32,
    /// 第三层级，用于定位具体功能组
    pub level3: i32,
    /// 第四层级，用于定位详细功能项
    pub level4: i32,
    /// 未读数量
    pub unread_count: i32,
}


crud!(SystemNotification {});