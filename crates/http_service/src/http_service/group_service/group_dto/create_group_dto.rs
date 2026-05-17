use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateGroupDTO {
    #[validate(length(min = 1, max = 100, message = "群名称长度必须在1-100之间"))]
    pub group_name: String,
    
    #[validate(length(max = 500, message = "群头像URL长度不能超过500"))]
    pub avatar: Option<String>,
    
    #[validate(length(max = 500, message = "群描述长度不能超过500"))]
    pub description: Option<String>,
    
    pub max_members: Option<i32>,
}
