use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BasePageDto {
    pub page_index: Option<u32>,
    pub page_size: Option<u32>,
    pub total: Option<u32>,
}
