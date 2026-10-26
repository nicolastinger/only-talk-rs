use serde::{Deserialize, Serialize};

/// 转发文件"重新发牌"结果：来源 biz -> 新目标 biz 的映射。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForwardChatBizVO {
    /// 原消息里的 biz_id
    pub source_biz_id: String,
    /// 为本次转发目标新签发的 biz_id
    pub biz_id: String,
}
