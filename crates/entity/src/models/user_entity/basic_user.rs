use rbatis::rbdc::Uuid;
use rbatis::{RBatis, crud, impl_select, impl_update};
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
impl_select!(BasicUser{select_by_account(account:&str) -> Option => "`where account = #{account} limit 1`"});
impl_select!(BasicUser{select_by_uuid(uuid:&Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
impl_update!(BasicUser{update_by_uuid(uuid:&Uuid) => "`where uuid = #{uuid}`"});

/// 是否存在某用户
pub async fn is_exist_user_by_uuid(rb: &RBatis, uuid: &Uuid) -> Result<bool, anyhow::Error> {
    let user = BasicUser::select_by_uuid(rb, uuid).await?;
    if user.is_none() {
        return Ok(false);
    }
    Ok(true)
}
