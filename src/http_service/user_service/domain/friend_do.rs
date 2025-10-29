pub struct FriendDO {
    pub name: String,
    pub age: i32,
}

impl FriendDO {
    pub fn new(name: String, age: i32) -> Self {
        FriendDO { name, age }
    }
}
