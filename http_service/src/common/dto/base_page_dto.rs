use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BasePageDTO {
    pub page_num: Option<u32>,
    pub page_size: Option<u32>,
    pub total: Option<u32>,
}
