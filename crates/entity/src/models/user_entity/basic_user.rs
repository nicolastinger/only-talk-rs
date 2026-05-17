use rbatis::rbdc::Uuid;
use rbatis::{RBatis, crud, impl_delete, impl_select, impl_select_page, impl_update};
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
}

crud!(BasicUser {}); //crud = insert+select_by_column+update_by_column+delete_by_column
impl_select!(BasicUser{select_all_by_id(username:&str,account:&str) => "`where icon = #{username} and info = #{account}`"});
impl_select!(BasicUser{select_by_account(account:&str) -> Option => "`where account = #{account} limit 1`"});
impl_select!(BasicUser{select_by_uuid(uuid:&Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
impl_update!(BasicUser{update_by_name(name:&str) => "`where id = 1`"});
impl_update!(BasicUser{update_by_icon(icon:&str) => "`where id = 1`"});
impl_delete!(BasicUser {delete_by_name(name:&str) => "`where name= '2'`"});
impl_select_page!(BasicUser{select_page(name:&str) => "`where name != #{name}`"});

/// 是否存在某用户
pub async fn is_exist_user_by_uuid(rb: &RBatis, uuid: &Uuid) -> Result<bool, anyhow::Error> {
    let user = BasicUser::select_by_uuid(rb, uuid).await?;
    if user.is_none() {
        return Ok(false);
    }
    Ok(true)
}
