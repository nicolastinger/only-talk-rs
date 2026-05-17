use rbatis::rbdc::{Bytes, Uuid};
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

/// 群消息类型
pub const MSG_TYPE_TEXT: i16 = 1;
pub const MSG_TYPE_IMAGE: i16 = 2;
pub const MSG_TYPE_FILE: i16 = 3;

/// 群消息记录（读扩散，只存 1 份）
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct GroupMessageRecord {
    /// 主键 ID
    pub id: Option<i64>,
    /// 消息唯一标识 (nanoid)
    pub nano_id: Option<String>,
    /// 群 UUID
    pub group_uuid: Option<Uuid>,
    /// 发送者 UUID
    pub send_user: Option<Uuid>,
    /// 时间戳 (Unix 时间戳，单位：毫秒)
    pub timestamp: Option<i64>,
    /// 原始消息内容
    pub raw: Bytes,
    /// 消息类型 (1: 文本, 2: 图片, 3: 文件)
    pub msg_type: Option<i16>,
    /// 是否撤回
    pub recalled: Option<bool>,
}

crud!(GroupMessageRecord {});

impl_select!(GroupMessageRecord{select_by_nano_id(nano_id: &str) -> Option => "`where nano_id = #{nano_id} limit 1`"});

impl_select!(GroupMessageRecord{select_by_group(group_uuid: &Uuid, start: u32, size: u32) => "`where group_uuid = #{group_uuid} order by timestamp desc limit #{size} offset #{start}`"});

impl_select!(GroupMessageRecord{select_unread(group_uuid: &Uuid, last_read_msg_id: i64) => "`where group_uuid = #{group_uuid} and id > #{last_read_msg_id} order by timestamp asc limit 100`"});
