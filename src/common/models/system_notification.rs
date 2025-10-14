use serde::{Deserialize, Serialize};

/// 系统通知, 不做持久化
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemNotification {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    
    /// 第一层级，用于定位功能大类
    pub level1: i32,
    /// 第二层级，用于定位子功能模块
    pub level2: i32,
    /// 第三层级，用于定位具体功能组
    pub level3: i32,
    /// 第四层级，用于定位详细功能项
    pub level4: i32,
    /// 第五层级，用于定位最细粒度的功能点
    pub level5: i32,
    /// 未读数量
    pub unread_count: i32,
}