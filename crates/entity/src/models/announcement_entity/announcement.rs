use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Announcement {
    /// 公告主键 (uuid)
    pub uuid: Option<Uuid>,
    /// 公告标题
    pub title: Option<String>,
    /// 公告内容 (markdown/html)
    pub content: Option<String>,
    /// 内容类型: 0-markdown, 1-html
    pub content_type: Option<i16>,
    /// 展示开始时间 (Unix 时间戳，秒)
    pub start_at: Option<i64>,
    /// 展示结束时间 (Unix 时间戳，秒)
    pub end_at: Option<i64>,
    /// 是否启用
    pub is_active: Option<bool>,
    /// 排序权重(越小越靠前)
    pub sort_order: Option<i16>,
    /// 是否删除
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
    /// 更新时间 (Unix 时间戳，秒)
    pub updated_at: Option<i64>,
}

crud!(Announcement {});

impl Announcement {
    #[rbatis::py_sql("select * from announcement where uuid = #{uuid} limit 1")]
    async fn select_by_uuid_inner(rb: &dyn Executor, uuid: &Uuid) -> Vec<Announcement> {}

    pub async fn select_by_uuid(
        rb: &dyn Executor,
        uuid: &Uuid,
    ) -> rbatis::Result<Option<Announcement>> {
        Ok(Self::select_by_uuid_inner(rb, uuid).await?.into_iter().next())
    }

    pub async fn update_by_uuid(
        rb: &dyn Executor,
        table: &Announcement,
        uuid: &Uuid,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error> {
        Announcement::update_by_map(rb, table, rbs::value! {"uuid": uuid}).await
    }
}
