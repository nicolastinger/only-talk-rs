use serde::{Deserialize, Serialize};
use validator::Validate;

/// 设置当前用户的手动状态
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct SetPresenceDTO {
    /// 状态: online(清除状态) / busy / away / studying / working / custom
    pub mode: String,
    /// 自定义文案, 仅 mode=custom 时有效(最长 32 个字符)
    pub text: Option<String>,
}

/// 批量查询好友手动状态
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct PresenceBatchDTO {
    /// 目标用户 uuid 列表; 仅"我的好友"会被返回, 其余忽略
    pub uuids: Vec<String>,
}
