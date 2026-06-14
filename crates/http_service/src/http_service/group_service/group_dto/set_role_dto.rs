use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct SetRoleDTO {
    pub group_uuid: String,
    pub user_uuid: String,

    #[validate(range(min = 0, max = 2, message = "角色值必须在0-2之间"))]
    pub role: i16,
}
