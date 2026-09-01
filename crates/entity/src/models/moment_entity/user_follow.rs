use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserFollow {
    /// 主键ID
    pub id: Option<Uuid>,
    /// 关注者uuid
    pub follow_user_uuid: Option<Uuid>,
    /// 被关注者uuid
    pub target_user_uuid: Option<Uuid>,
    /// 是否删除(取消关注=软删)
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
}

crud!(UserFollow {});

impl UserFollow {
    #[rbatis::py_sql("select * from user_follow where follow_user_uuid = #{follow_user_uuid} and target_user_uuid = #{target_user_uuid} limit 1")]
    async fn select_by_follow_and_target(rb: &dyn Executor, follow_user_uuid: &Uuid, target_user_uuid: &Uuid) -> Option<UserFollow> {}
}
