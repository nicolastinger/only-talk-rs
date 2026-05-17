use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct AddMemberDTO {
    pub group_uuid: String,
    
    #[validate(length(min = 1, message = "成员列表不能为空"))]
    pub user_uuids: Vec<String>,
}
