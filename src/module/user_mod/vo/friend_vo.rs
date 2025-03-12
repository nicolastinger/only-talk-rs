use serde::{Serialize};

#[derive(Serialize, Debug)]
pub struct FriendVO <'a>{
    pub uuid:&'a str
}