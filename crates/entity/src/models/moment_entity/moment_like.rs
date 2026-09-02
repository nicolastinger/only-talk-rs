use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MomentLike {
    /// 主键ID
    pub id: Option<Uuid>,
    /// 动态key
    pub moment_uuid: Option<Uuid>,
    /// 点赞用户uuid
    pub user_uuid: Option<Uuid>,
    /// 是否删除(取消点赞=软删)
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
}

crud!(MomentLike {});

impl MomentLike {
    #[rbatis::py_sql("select * from moment_like where moment_uuid = #{moment_uuid} and user_uuid = #{user_uuid} limit 1")]
    async fn select_by_moment_and_user_inner(rb: &dyn Executor, moment_uuid: &Uuid, user_uuid: &Uuid) -> Vec<MomentLike> {}

    pub async fn select_by_moment_and_user(
        rb: &dyn Executor,
        moment_uuid: &Uuid,
        user_uuid: &Uuid,
    ) -> rbatis::Result<Option<MomentLike>> {
        Ok(Self::select_by_moment_and_user_inner(rb, moment_uuid, user_uuid).await?.into_iter().next())
    }

    #[rbatis::py_sql("select * from moment_like where moment_uuid = #{moment_uuid}")]
    async fn select_by_moment(rb: &dyn Executor, moment_uuid: &Uuid) -> Vec<MomentLike> {}
}
