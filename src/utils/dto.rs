use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct ReqList <T> {
    pub page_num: Option<u32>,
    pub page_size: Option<u32>,
    pub data: Option<T>,
}

///请求的account
#[derive(Clone, Debug)]
pub struct AuthAccount(pub String);