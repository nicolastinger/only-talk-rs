use serde::{Serialize};

#[derive(Serialize, Debug)]
pub struct FriendVO <'a>{
    pub uuid:&'a str
}

#[derive(Serialize, Debug)]
pub struct FriendListVO {
    pub account: Option<String>,
    pub username: Option<String>,
    pub icon: Option<String>,
    pub info: Option<String>
}