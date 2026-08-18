use rbatis::rbdc::Uuid;
use rbatis::{RBatis, crud, impl_select};
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
    pub email: Option<String>,
    pub password: Option<String>,
}

crud!(BasicUser {}); //crud = 插入+按列查询+按列更新+按列删除
impl_select!(BasicUser{select_by_account(account:&str) -> Option => "`where account = #{account} limit 1`"});
impl_select!(BasicUser{select_by_email(email:&str) -> Option => "`where email = #{email} limit 1`"});
impl_select!(BasicUser{select_by_uuid(uuid:&Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});

/// 是否存在某用户
pub async fn is_exist_user_by_uuid(rb: &RBatis, uuid: &Uuid) -> Result<bool, anyhow::Error> {
    let user = BasicUser::select_by_uuid(rb, uuid).await?;
    if user.is_none() {
        return Ok(false);
    }
    Ok(true)
}
