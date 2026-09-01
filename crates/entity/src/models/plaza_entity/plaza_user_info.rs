use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlazaUserInfo {
    /// 用户主键
    pub uuid: Option<Uuid>,
    /// 是否允许被发现 (默认 false)
    pub allow_discover: Option<bool>,
    /// 交友宣言
    pub motto: Option<String>,
    /// 状态 (0: 正常, 1: 隐藏, 2: 封禁)
    pub status: Option<u8>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
    /// 更新时间 (Unix 时间戳，秒)
    pub updated_at: Option<i64>,
}

crud!(PlazaUserInfo {});

impl PlazaUserInfo {
    #[rbatis::py_sql("select * from plaza_user_info where uuid = #{uuid} limit 1")]
    async fn select_by_uuid(rb: &dyn Executor, uuid: &Uuid) -> Option<PlazaUserInfo> {}

    pub async fn update_by_uuid(
        rb: &dyn Executor,
        table: &PlazaUserInfo,
        uuid: &Uuid,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error> {
        PlazaUserInfo::update_by_map(rb, table, rbs::value! {"uuid": uuid}).await
    }
}
