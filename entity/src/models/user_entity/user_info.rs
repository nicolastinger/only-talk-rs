use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select, impl_update};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserInfo {
    /// 用户唯一标识符 (UUID)
    pub uuid: Option<Uuid>,
    /// 用户性别 (0: 未知, 1: 男, 2: 女)
    pub gender: Option<u8>,
    /// 用户年龄
    pub age: Option<u8>,
    /// 用户生日 (Unix 时间戳，单位：秒)
    pub birthday: Option<i64>,
    /// 用户备注信息
    pub note: Option<String>,
    /// 记录创建时间 (Unix 时间戳，单位：秒)
    pub created_at: Option<i64>,
    /// 记录最后更新时间 (Unix 时间戳，单位：秒)
    pub updated_at: Option<i64>,
    /// 用户手机号码
    pub phone: Option<String>,
    /// 用户电子邮箱
    pub email: Option<String>,
    /// 用户地址信息
    pub address: Option<String>,
    /// 用户状态 (0: 正常, 1: 禁用, 2: 注销等)
    pub status: Option<u8>,
}

crud!(UserInfo {});

impl_select!(UserInfo{select_by_uuid(uuid: &Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
impl_update!(UserInfo{update_by_uuid(uuid: &Uuid) => "`where uuid = #{uuid}`"});
