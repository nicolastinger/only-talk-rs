use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlazaLike {
    /// 主键ID
    pub id: Option<Uuid>,
    /// 被心动用户uuid
    pub target_uuid: Option<Uuid>,
    /// 心动用户uuid
    pub user_uuid: Option<Uuid>,
    /// 是否删除(取消心动=软删)
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
}

crud!(PlazaLike {});

impl PlazaLike {
    #[rbatis::py_sql("select * from plaza_like where target_uuid = #{target_uuid} and user_uuid = #{user_uuid} limit 1")]
    async fn select_by_target_and_user_inner(rb: &dyn Executor, target_uuid: &Uuid, user_uuid: &Uuid) -> Vec<PlazaLike> {}

    pub async fn select_by_target_and_user(
        rb: &dyn Executor,
        target_uuid: &Uuid,
        user_uuid: &Uuid,
    ) -> rbatis::Result<Option<PlazaLike>> {
        Ok(Self::select_by_target_and_user_inner(rb, target_uuid, user_uuid).await?.into_iter().next())
    }

    #[rbatis::py_sql("select * from plaza_like where target_uuid = #{target_uuid}")]
    async fn select_by_target(rb: &dyn Executor, target_uuid: &Uuid) -> Vec<PlazaLike> {}
}
