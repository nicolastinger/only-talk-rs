use rbatis::{crud, impl_select};
use rbatis::rbdc::{Uuid};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ChatMessageRead{
    pub id: Option<i64>,
    pub nano_id: Option<String>,
    pub timestamp: Option<i64>,
    pub send_user: Uuid,
    pub recv_user: Uuid
}

crud!(ChatMessageRead {});

// 获取已读消息
impl_select!(ChatMessageRead {select_all_read_by_column(uuid: &Uuid, size: i32)  => "`where recv_user = ?1 order by timestamp desc limit ?2`"});