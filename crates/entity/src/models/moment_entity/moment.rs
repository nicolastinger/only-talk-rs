use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select, impl_update};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Moment {
    /// 动态主键(同时也是图片分组 biz_id)
    pub uuid: Option<Uuid>,
    /// 作者用户uuid
    pub author_uuid: Option<Uuid>,
    /// 文案内容
    pub content: Option<String>,
    /// 可见性: 0-公开, 1-仅自己
    pub visibility: Option<i16>,
    /// 是否删除
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
    /// 更新时间 (Unix 时间戳，秒)
    pub updated_at: Option<i64>,
}

crud!(Moment {});

impl_select!(Moment {select_by_uuid(uuid:&Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
impl_update!(Moment {update_by_uuid(uuid:&Uuid) => "`where uuid = #{uuid}`"});
