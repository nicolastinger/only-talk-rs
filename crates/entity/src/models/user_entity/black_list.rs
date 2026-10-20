use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlackList {
    pub uuid: Option<Uuid>,
    pub me_user: Option<Uuid>,
    pub block_user: Option<Uuid>,
    pub created_at: Option<i64>,
    pub version: Option<i32>,
}

crud!(BlackList {});

impl BlackList {
    #[rbatis::py_sql(
        "select * from black_list where me_user = #{me_user} and block_user = #{block_user} limit 1"
    )]
    async fn select_by_pair_inner(
        rb: &dyn Executor,
        me_user: &Uuid,
        block_user: &Uuid,
    ) -> Vec<BlackList> {
    }

    pub async fn select_by_pair(
        rb: &dyn Executor,
        me_user: &Uuid,
        block_user: &Uuid,
    ) -> rbatis::Result<Option<BlackList>> {
        Ok(Self::select_by_pair_inner(rb, me_user, block_user).await?.into_iter().next())
    }

    #[rbatis::py_sql("select * from black_list where me_user = #{me_user}")]
    async fn select_by_me_user(rb: &dyn Executor, me_user: &Uuid) -> Vec<BlackList> {}
}
