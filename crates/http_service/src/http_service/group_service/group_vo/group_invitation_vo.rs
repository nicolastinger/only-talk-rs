use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupInvitationVO {
    pub id: i64,
    pub group_uuid: String,
    pub group_name: String,
    pub group_avatar: Option<String>,
    pub inviter_uuid: String,
    pub invitee_uuid: String,
    pub status: i16,
    pub created_at: i64,
}
