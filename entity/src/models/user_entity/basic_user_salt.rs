use anyhow::anyhow;
use rbatis::{crud, impl_select, RBatis};
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct BasicUserSalt {
    pub uuid: Option<Uuid>,
    pub sign_up_salt: Option<String>,
}

crud!(BasicUserSalt {});

impl_select!(BasicUserSalt{select_by_uuid(uuid:&Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
/// 查询用户盐
pub async fn get_user_salt(rb: &RBatis, uuid: &Option<Uuid>) -> Result<BasicUserSalt, anyhow::Error> {
    // 查询盐值信息
    let uuid = uuid.as_ref().ok_or(anyhow!("用户不存在!".to_string()))?;
    let salt = BasicUserSalt::select_by_uuid(rb, uuid).await?;
    // 检查盐值是否存在
    salt.ok_or(anyhow!("用户不存在!".to_string()))

}