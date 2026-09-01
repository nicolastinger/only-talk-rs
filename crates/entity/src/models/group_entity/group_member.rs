use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
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

impl GroupMember {
    #[rbatis::py_sql("select * from group_member where group_uuid = #{group_uuid} and user_uuid = #{user_uuid} limit 1")]
    async fn select_by_group_and_user(rb: &dyn Executor, group_uuid: &Uuid, user_uuid: &Uuid) -> Option<GroupMember> {}

    pub async fn update_by_group_and_user(
        rb: &dyn Executor,
        table: &GroupMember,
        group_uuid: &Uuid,
        user_uuid: &Uuid,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error> {
        GroupMember::update_by_map(
            rb,
            table,
            rbs::value! {"group_uuid": group_uuid, "user_uuid": user_uuid},
        )
        .await
    }

    #[rbatis::py_sql("select * from group_member where group_uuid = #{group_uuid} and status = 1")]
    async fn select_members_by_group(rb: &dyn Executor, group_uuid: &Uuid) -> Vec<GroupMember> {}

    #[rbatis::py_sql("select * from group_member where user_uuid = #{user_uuid} and status = 1")]
    async fn select_groups_by_user(rb: &dyn Executor, user_uuid: &Uuid) -> Vec<GroupMember> {}
}
