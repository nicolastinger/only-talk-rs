use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlazaLike {
    /// 主键ID
    pub id: Option<Uuid>,
    /// 被心动用户uuid
    pub target_uuid: Option<Uuid>,
    /// 心动用户uuid
    pub user_uuid: Option<Uuid>,
    /// 是否删除(取消心动=软删)
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
}

crud!(PlazaLike {});

impl_select!(PlazaLike {select_by_target_and_user(target_uuid:&Uuid, user_uuid:&Uuid) -> Option => "`where target_uuid = #{target_uuid} and user_uuid = #{user_uuid} limit 1`"});
impl_select!(PlazaLike {select_by_target(target_uuid:&Uuid) => "`where target_uuid = #{target_uuid}`"});
