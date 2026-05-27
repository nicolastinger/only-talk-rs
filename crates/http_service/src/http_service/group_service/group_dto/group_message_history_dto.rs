use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct GroupMessageHistoryDTO {
    pub group_uuid: String,
    
    #[validate(range(min = 0, message = "起始位置不能为负数"))]
    pub start: Option<u32>,
    
    #[validate(range(min = 1, max = 100, message = "每页数量必须在1-100之间"))]
    pub size: Option<u32>,
}
