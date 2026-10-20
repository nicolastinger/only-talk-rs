use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateReportDTO {
    /// 举报目标类型: 1=用户, 2=群组, 3=动态, 4=卡片匹配(交友广场用户), 5=动态评论
    pub target_type: i16,
    /// 举报目标主键 (用户/群/动态/广场用户 uuid 或 评论 id)
    pub target_uuid: String,
    /// 举报原因/描述
    pub reason: String,
}
