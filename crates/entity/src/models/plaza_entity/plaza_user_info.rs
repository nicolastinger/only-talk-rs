use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select, impl_update};
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

impl_select!(PlazaUserInfo {select_by_uuid(uuid:&Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
impl_update!(PlazaUserInfo {update_by_uuid(uuid:&Uuid) => "`where uuid = #{uuid}`"});
