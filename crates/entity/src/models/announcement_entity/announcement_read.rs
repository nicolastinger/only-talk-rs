use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnnouncementRead {
    /// 主键ID
    pub id: Option<Uuid>,
    /// 公告key
    pub announcement_uuid: Option<Uuid>,
    /// 已读用户uuid
    pub user_uuid: Option<Uuid>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
}

crud!(AnnouncementRead {});

impl AnnouncementRead {
    #[rbatis::py_sql("select * from announcement_read where announcement_uuid = #{announcement_uuid} and user_uuid = #{user_uuid} limit 1")]
    async fn select_by_announcement_and_user(rb: &dyn Executor, announcement_uuid: &Uuid, user_uuid: &Uuid) -> Option<AnnouncementRead> {}
}
