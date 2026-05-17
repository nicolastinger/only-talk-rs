use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAddressInfo {
    pub uuid: String,
    pub address: String,
    pub token: String,
    pub ip_type: u8,
    pub target_uuid: String,
    pub nat_type: u8,
    pub is_server: bool,
    pub lock_uuid: String,
    pub is_lock: bool,
}
