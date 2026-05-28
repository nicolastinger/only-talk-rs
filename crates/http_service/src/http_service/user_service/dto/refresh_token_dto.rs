use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct RefreshTokenDTO {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}
