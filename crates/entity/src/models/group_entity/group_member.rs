use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select, impl_update};
use serde::{Deserialize, Serialize};

/// 群成员角色
pub const ROLE_MEMBER: i16 = 0;
pub const ROLE_ADMIN: i16 = 1;
pub const ROLE_OWNER: i16 = 2;

/// 群成员状态
pub const STATUS_NORMAL: i16 = 1;
pub const STATUS_QUIT: i16 = 2;
pub const STATUS_KICKED: i16 = 3;

/// 群成员信息
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct GroupMember {
    /// 主键 ID
    pub id: Option<i64>,
    /// 群 UUID
    pub group_uuid: Option<Uuid>,
    /// 用户 UUID
    pub user_uuid: Option<Uuid>,
    /// 角色 (0: 成员, 1: 管理员, 2: 群主)
    pub role: Option<i16>,
    /// 群内昵称
    pub nickname: Option<String>,
    /// 入群时间 (Unix 时间戳，单位：毫秒)
    pub join_time: Option<i64>,
    /// 已读消息 ID
    pub last_read_msg_id: Option<i64>,
    /// 是否禁言
    pub muted: Option<bool>,
    /// 成员状态 (1: 正常, 2: 已退出, 3: 被踢出)
    pub status: Option<i16>,
}

crud!(GroupMember {});

impl_select!(GroupMember{select_by_group_and_user(group_uuid: &Uuid, user_uuid: &Uuid) -> Option => "`where group_uuid = #{group_uuid} and user_uuid = #{user_uuid} limit 1`"});
impl_update!(GroupMember{update_by_group_and_user(group_uuid: &Uuid, user_uuid: &Uuid) => "`where group_uuid = #{group_uuid} and user_uuid = #{user_uuid}`"});

impl_select!(GroupMember{select_members_by_group(group_uuid: &Uuid) => "`where group_uuid = #{group_uuid} and status = 1`"});

impl_select!(GroupMember{select_groups_by_user(user_uuid: &Uuid) => "`where user_uuid = #{user_uuid} and status = 1`"});
