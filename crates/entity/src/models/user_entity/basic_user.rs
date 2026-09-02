use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::{RBatis, crud};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 作为登录注册的基本用户信息
#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct BasicUser {
    pub uuid: Option<Uuid>,
    pub username: Option<String>,
    pub account: Option<String>,
    pub icon: Option<String>,
    pub info: Option<String>,
    pub password: Option<String>,
    /// 注册状态: 0=占位未完成(邮箱已验证但资料未补全,不可登录) 1=已完成
    pub registration_status: Option<i16>,
}

crud!(BasicUser {}); //crud = 插入+按列查询+按列更新+按列删除

impl BasicUser {
    #[rbatis::py_sql("select * from basic_user where account = #{account} limit 1")]
    async fn select_by_account_inner(rb: &dyn Executor, account: &str) -> Vec<BasicUser> {}

    pub async fn select_by_account(
        rb: &dyn Executor,
        account: &str,
    ) -> rbatis::Result<Option<BasicUser>> {
        Ok(Self::select_by_account_inner(rb, account).await?.into_iter().next())
    }

    #[rbatis::py_sql("select * from basic_user where uuid = #{uuid} limit 1")]
    async fn select_by_uuid_inner(rb: &dyn Executor, uuid: &Uuid) -> Vec<BasicUser> {}

    pub async fn select_by_uuid(
        rb: &dyn Executor,
        uuid: &Uuid,
    ) -> rbatis::Result<Option<BasicUser>> {
        Ok(Self::select_by_uuid_inner(rb, uuid).await?.into_iter().next())
    }

    pub async fn update_by_uuid(
        rb: &dyn Executor,
        table: &BasicUser,
        uuid: &Uuid,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error> {
        BasicUser::update_by_map(rb, table, rbs::value! {"uuid": uuid}).await
    }
}

/// 是否存在某用户
pub async fn is_exist_user_by_uuid(rb: &RBatis, uuid: &Uuid) -> Result<bool, anyhow::Error> {
    let user = BasicUser::select_by_uuid(rb, uuid).await?;
    if user.is_none() {
        return Ok(false);
    }
    Ok(true)
}
