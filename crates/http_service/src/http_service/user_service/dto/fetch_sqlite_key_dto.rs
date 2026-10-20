use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct FetchSqliteKeyDTO {
    #[validate(length(min = 16, message = "设备指纹长度必须大于16"))]
    pub device_fingerprint: String,
}
