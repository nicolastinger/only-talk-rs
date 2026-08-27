use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateMomentDTO {
    /// 文案内容
    pub content: String,
    /// 可见性: 0-公开, 1-仅自己
    pub visibility: i16,
    /// 已上传的图片 file_id 列表(FileUploadRecord.uuid)
    pub file_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MomentListQuery {
    pub author_uuid: Option<String>,
    /// 动态流类型: plaza-广场, following-关注, mine-我的
    pub feed: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LikeToggleDTO {
    pub moment_uuid: String,
}

/// 关注切换
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FollowToggleDTO {
    pub target_user_uuid: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddCommentDTO {
    pub moment_uuid: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommentListQuery {
    pub moment_uuid: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LikeListQuery {
    pub moment_uuid: String,
}
