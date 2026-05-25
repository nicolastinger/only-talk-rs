use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select, impl_update};
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

impl_select!(GroupInvitation{select_pending_by_invitee(invitee_uuid: &Uuid) => "`where invitee_uuid = #{invitee_uuid} and status = 1 order by created_at desc`"});
impl_select!(GroupInvitation{select_by_inviter(inviter_uuid: &Uuid) => "`where inviter_uuid = #{inviter_uuid} order by created_at desc`"});
impl_select!(GroupInvitation{select_pending_by_group(group_uuid: &Uuid) => "`where group_uuid = #{group_uuid} and status = 1 order by created_at desc`"});
impl_select!(GroupInvitation{select_by_group_and_invitee(group_uuid: &Uuid, invitee_uuid: &Uuid) -> Option => "`where group_uuid = #{group_uuid} and invitee_uuid = #{invitee_uuid} order by created_at desc limit 1`"});
impl_update!(GroupInvitation{update_by_id(id: &i64) => "`where id = #{id}`"});
