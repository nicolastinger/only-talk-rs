use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlazaUserTag {
    /// 主键
    pub id: Option<Uuid>,
    /// 所属用户
    pub user_uuid: Option<Uuid>,
    /// 标签内容
    pub tag: Option<String>,
    /// 展示顺序
    pub sort: Option<i32>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
}

crud!(PlazaUserTag {});

impl PlazaUserTag {
    #[rbatis::py_sql("select * from plaza_user_tag where user_uuid = #{uuid} order by sort asc, created_at asc")]
    async fn select_by_user(rb: &dyn Executor, uuid: &Uuid) -> Vec<PlazaUserTag> {}

    pub async fn delete_by_user(
        rb: &dyn Executor,
        uuid: &Uuid,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error> {
        PlazaUserTag::delete_by_map(rb, rbs::value! {"user_uuid": uuid}).await
    }
}
