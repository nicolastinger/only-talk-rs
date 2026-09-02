use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendLink {
    pub uuid: Option<Uuid>,
    pub request_user: Option<Uuid>,
    pub accept_user: Option<Uuid>,
    pub is_del: Option<bool>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub version: Option<i32>,
}

crud!(FriendLink {});

impl FriendLink {
    #[rbatis::py_sql("select * from friend_link where (accept_user = #{uuid} and request_user = #{last_uuid}) or (accept_user = #{last_uuid} and request_user = #{uuid}) limit 1")]
    async fn select_by_last_uuid_inner(rb: &dyn Executor, uuid: &Uuid, last_uuid: &Uuid) -> Vec<FriendLink> {}

    pub async fn select_by_last_uuid(
        rb: &dyn Executor,
        uuid: &Uuid,
        last_uuid: &Uuid,
    ) -> rbatis::Result<Option<FriendLink>> {
        Ok(Self::select_by_last_uuid_inner(rb, uuid, last_uuid).await?.into_iter().next())
    }

    pub async fn update_is_del_by_users(
        rb: &dyn Executor,
        table: &FriendLink,
        request_user: &Uuid,
        accept_user: &Uuid,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error> {
        FriendLink::update_by_map(
            rb,
            table,
            rbs::value! {"request_user": request_user, "accept_user": accept_user},
        )
        .await
    }
}
