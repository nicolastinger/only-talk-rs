use serde::{Deserialize, Serialize};

/// 用户登录记录表
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserLoginLog {
    pub id: Option<i64>,
    pub last_login_at: Option<i64>,
    // 最后登录的设备
    pub last_login_equipment: Option<String>,
    pub last_login_ipv4: Option<String>,
    pub last_login_ipv6: Option<String>,
}
