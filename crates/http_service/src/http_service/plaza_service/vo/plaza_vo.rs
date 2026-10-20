use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PlazaUserVO {
    pub uuid: String,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub info: Option<String>,
    /// 用户类型 (0: 普通用户, 1: 机器人, 2: 企业用户, 其他待补充)
    pub user_type: Option<i16>,
    pub gender: Option<i32>,
    pub age: Option<i32>,
    pub address: Option<String>,
    pub motto: Option<String>,
    pub tags: Vec<String>,
    pub liked_by_me: bool,
}

#[derive(Serialize, Debug)]
pub struct PlazaListVO {
    pub total: u32,
    pub list: Vec<PlazaUserVO>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlazaCrushResult {
    /// 是否达成互相心动(匹配)
    pub matched: bool,
}

#[derive(Serialize, Debug)]
pub struct PlazaProfileVO {
    pub allow_discover: bool,
    pub motto: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct PlazaUserRow {
    pub uuid: Option<String>,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub info: Option<String>,
    pub user_type: Option<i16>,
    pub gender: Option<i32>,
    pub age: Option<i32>,
    pub address: Option<String>,
    pub motto: Option<String>,
    pub liked_by_me: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct CountRow {
    pub count: i64,
}
