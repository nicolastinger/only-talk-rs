use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select, impl_update};
use serde::{Deserialize, Serialize};

/// 群组信息
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct GroupInfo {
    /// 主键 ID
    pub id: Option<i64>,
    /// 群 UUID
    pub group_uuid: Option<Uuid>,
    /// 群名称
    pub group_name: Option<String>,
    /// 群头像
    pub avatar: Option<String>,
    /// 群主 UUID
    pub owner_uuid: Option<Uuid>,
    /// 群描述
    pub description: Option<String>,
    /// 最大成员数
    pub max_members: Option<i32>,
    /// 创建时间 (Unix 时间戳，单位：毫秒)
    pub created_at: Option<i64>,
    /// 更新时间 (Unix 时间戳，单位：毫秒)
    pub updated_at: Option<i64>,
    /// 群状态 (1: 正常, 2: 解散)
    pub status: Option<i16>,
}

crud!(GroupInfo {});

impl_select!(GroupInfo{select_by_group_uuid(group_uuid: &Uuid) -> Option => "`where group_uuid = #{group_uuid} limit 1`"});
impl_update!(GroupInfo{update_by_group_uuid(group_uuid: &Uuid) => "`where group_uuid = #{group_uuid}`"});

impl_select!(GroupInfo{select_by_owner_uuid(owner_uuid: &Uuid) => "`where owner_uuid = #{owner_uuid} and status = 1`"});
