use serde::{Deserialize, Serialize};

/// 用户手动状态(对外展示)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresenceVO {
    /// 状态枚举: busy / away / studying / working / custom; 无状态时为 online
    pub mode: String,
    /// 自定义文案(仅 mode=custom)
    pub text: Option<String>,
    /// 过期时间(毫秒时间戳), 0 表示无状态
    pub expires_at: i64,
}
