use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MomentComment {
    /// 主键ID
    pub id: Option<Uuid>,
    /// 动态key
    pub moment_uuid: Option<Uuid>,
    /// 评论作者uuid
    pub author_uuid: Option<Uuid>,
    /// 评论内容
    pub content: Option<String>,
    /// 是否删除
    pub is_del: Option<bool>,
    /// 创建时间 (Unix 时间戳，秒)
    pub created_at: Option<i64>,
}

crud!(MomentComment {});

impl_select!(MomentComment {select_by_moment(moment_uuid:&Uuid) => "`where moment_uuid = #{moment_uuid} and is_del = false order by created_at asc`"});
