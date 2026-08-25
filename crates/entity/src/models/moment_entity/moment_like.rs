use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MomentLike {
    /// 主键ID
    pub id: Option<Uuid>,
    /// 动态key
    pub moment_uuid: Option<Uuid>,
    /// 点赞用户uuid
    pub user_uuid: Option<Uuid>,
    /// 是否删除(取消点赞=软删)
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
}

crud!(MomentLike {});

impl_select!(MomentLike {select_by_moment_and_user(moment_uuid:&Uuid, user_uuid:&Uuid) -> Option => "`where moment_uuid = #{moment_uuid} and user_uuid = #{user_uuid} limit 1`"});
impl_select!(MomentLike {select_by_moment(moment_uuid:&Uuid) => "`where moment_uuid = #{moment_uuid}`"});
