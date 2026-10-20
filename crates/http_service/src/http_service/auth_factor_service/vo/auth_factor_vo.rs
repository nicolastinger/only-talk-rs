use serde::{Deserialize, Serialize};

/// 二次认证因素视图对象
#[derive(Serialize, Deserialize, Debug)]
pub struct AuthFactorVO {
    pub id: i64,
    pub user_id: String,
    pub factor_type: i16,
    pub factor_value: String,
    pub verified: bool,
    pub enabled: bool,
    pub is_primary: bool,
    pub status: i16,
    pub verified_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 二次认证因素列表视图对象
#[derive(Serialize, Debug)]
pub struct AuthFactorListVO {
    pub total: u32,
    pub list: Vec<AuthFactorVO>,
}
