use serde::{Deserialize, Serialize};

/// 转发文件前"重新发牌"的入参。
///
/// 客户端转发图片/文件/语音时不会重新上传文件，而是带着原始 `biz_id` 请求本接口，
/// 由服务端为**本次转发目标**签发一条新的 `chat_biz_record`（复用原文件关联）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForwardChatBizDTO {
    /// 来源业务 id 列表（被转发的图片/文件/语音消息 raw 里的 `biz_id`）
    pub biz_ids: Vec<String>,
    /// 目标接收者：单聊为目标好友 uuid，群聊为目标群 uuid
    pub receiver: String,
    /// 目标是否为群聊
    pub is_group: bool,
}
