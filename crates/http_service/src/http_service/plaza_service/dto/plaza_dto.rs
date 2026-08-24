use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlazaUpdateProfileDTO {
    pub allow_discover: Option<bool>,
    pub motto: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlazaUpdateTagsDTO {
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct PlazaListQuery {
    pub gender: Option<u8>,
    pub age_min: Option<u32>,
    pub age_max: Option<u32>,
}
