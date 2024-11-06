use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct BasicUser {
    pub username: String,
    pub account: String
}

#[derive(Deserialize, Serialize, Debug)]
struct UserInfo {
    username: String,
    account: String,
    icon: Option<String>,
    password: String,
    gender: u8,  //0-m,1-s,2-男,3-女,4-机器人,5-other,etc
    age: u8,
    note: Option<String>,
    create_time: i64,
    update_time: i64,
    last_login_time: i64,
    last_login_equipment: Option<String>,  //最后登录的设备
    last_login_ipv4: Option<String>,
    last_login_ipv6: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    is_active: bool
}