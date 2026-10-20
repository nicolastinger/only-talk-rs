use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Moment {
    /// 动态主键(同时也是图片分组 biz_id)
    pub uuid: Option<Uuid>,
    /// 作者用户uuid
    pub author_uuid: Option<Uuid>,
    /// 文案内容
    pub content: Option<String>,
    /// 可见性: 0-公开, 1-仅自己
    pub visibility: Option<i16>,
    /// 是否删除
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
    /// 更新时间 (Unix 时间戳，秒)
    pub updated_at: Option<i64>,
}

crud!(Moment {});

impl Moment {
    #[rbatis::py_sql("select * from moment where uuid = #{uuid} limit 1")]
    async fn select_by_uuid_inner(rb: &dyn Executor, uuid: &Uuid) -> Vec<Moment> {}

    pub async fn select_by_uuid(rb: &dyn Executor, uuid: &Uuid) -> rbatis::Result<Option<Moment>> {
        Ok(Self::select_by_uuid_inner(rb, uuid).await?.into_iter().next())
    }

    pub async fn update_by_uuid(
        rb: &dyn Executor,
        table: &Moment,
        uuid: &Uuid,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error> {
        Moment::update_by_map(rb, table, rbs::value! {"uuid": uuid}).await
    }
}
