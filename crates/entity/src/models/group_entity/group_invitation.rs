use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
use serde::{Deserialize, Serialize};

/// 群邀请状态
pub const INVITATION_PENDING: i16 = 1;
pub const INVITATION_ACCEPTED: i16 = 2;
pub const INVITATION_DECLINED: i16 = 3;

/// 群邀请记录
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct GroupInvitation {
    pub id: Option<i64>,
    pub group_uuid: Option<Uuid>,
    pub inviter_uuid: Option<Uuid>,
    pub invitee_uuid: Option<Uuid>,
    pub status: Option<i16>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

crud!(GroupInvitation {});

impl GroupInvitation {
    #[rbatis::py_sql("select * from group_invitation where invitee_uuid = #{invitee_uuid} and status = 1 order by created_at desc")]
    async fn select_pending_by_invitee(rb: &dyn Executor, invitee_uuid: &Uuid) -> Vec<GroupInvitation> {}

    #[rbatis::py_sql("select * from group_invitation where inviter_uuid = #{inviter_uuid} order by created_at desc")]
    async fn select_by_inviter(rb: &dyn Executor, inviter_uuid: &Uuid) -> Vec<GroupInvitation> {}

    #[rbatis::py_sql("select * from group_invitation where group_uuid = #{group_uuid} and status = 1 order by created_at desc")]
    async fn select_pending_by_group(rb: &dyn Executor, group_uuid: &Uuid) -> Vec<GroupInvitation> {}

    #[rbatis::py_sql("select * from group_invitation where group_uuid = #{group_uuid} and invitee_uuid = #{invitee_uuid} order by created_at desc limit 1")]
    async fn select_by_group_and_invitee_inner(rb: &dyn Executor, group_uuid: &Uuid, invitee_uuid: &Uuid) -> Vec<GroupInvitation> {}

    pub async fn select_by_group_and_invitee(
        rb: &dyn Executor,
        group_uuid: &Uuid,
        invitee_uuid: &Uuid,
    ) -> rbatis::Result<Option<GroupInvitation>> {
        Ok(Self::select_by_group_and_invitee_inner(rb, group_uuid, invitee_uuid).await?.into_iter().next())
    }

    pub async fn update_by_id(
        rb: &dyn Executor,
        table: &GroupInvitation,
        id: &i64,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error> {
        GroupInvitation::update_by_map(rb, table, rbs::value! {"id": id}).await
    }
}
