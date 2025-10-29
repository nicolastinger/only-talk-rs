use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct ChatListLinkVo {}

crud!(ChatListLinkVo {});
impl_select!(ChatListLinkVo {select_chat_link_by_uuid() => r#"`where uuid = #{uuid}`"#});
