use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MomentComment {
    /// 主键ID
    pub id: Option<Uuid>,
    /// 动态key
    pub moment_uuid: Option<Uuid>,
    /// 评论作者uuid
    pub author_uuid: Option<Uuid>,
    /// 评论内容
    pub content: Option<String>,
    /// 是否删除
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
}

crud!(MomentComment {});

impl MomentComment {
    #[rbatis::py_sql(
        "select * from moment_comment where moment_uuid = #{moment_uuid} and is_del = false order by created_at asc"
    )]
    async fn select_by_moment(rb: &dyn Executor, moment_uuid: &Uuid) -> Vec<MomentComment> {}
}
