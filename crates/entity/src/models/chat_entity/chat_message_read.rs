use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

/// 会话类型: 单聊
pub const CHAT_TYPE_SINGLE: u32 = 1;
/// 会话类型: 群聊
pub const CHAT_TYPE_GROUP: u32 = 2;

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ChatMessageRecordRead {
    pub id: Option<i64>,
    pub nano_id: Option<String>,
    pub timestamp: Option<i64>,
    pub send_user: Uuid,
    pub recv_user: Uuid,
}

crud!(ChatMessageRecordRead {});

// 获取已读消息
impl_select!(ChatMessageRecordRead {select_all_read_by_column(uuid: &Uuid, size: i32)  => r#"where recv_user = #{uuid} order by timestamp desc limit #{size}"#});
