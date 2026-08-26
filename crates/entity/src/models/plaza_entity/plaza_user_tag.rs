use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_delete, impl_select};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlazaUserTag {
    /// 主键
    pub id: Option<Uuid>,
    /// 所属用户
    pub user_uuid: Option<Uuid>,
    /// 标签内容
    pub tag: Option<String>,
    /// 展示顺序
    pub sort: Option<i32>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
}

crud!(PlazaUserTag {});

impl_select!(PlazaUserTag {select_by_user(uuid:&Uuid) -> Vec => "`where user_uuid = #{uuid} order by sort asc, created_at asc`"});
impl_delete!(PlazaUserTag {delete_by_user(uuid:&Uuid) => "`where user_uuid = #{uuid}`"});
