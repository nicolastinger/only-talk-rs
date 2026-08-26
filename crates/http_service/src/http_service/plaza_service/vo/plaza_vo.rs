use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PlazaUserVO {
    pub uuid: String,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub info: Option<String>,
    pub gender: Option<i32>,
    pub age: Option<i32>,
    pub address: Option<String>,
    pub motto: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct PlazaListVO {
    pub total: u32,
    pub list: Vec<PlazaUserVO>,
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
    pub gender: Option<i32>,
    pub age: Option<i32>,
    pub address: Option<String>,
    pub motto: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CountRow {
    pub count: i64,
}
