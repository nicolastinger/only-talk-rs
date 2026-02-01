use rbatis::{crud, impl_select};
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 公开文件业务表关联
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BizFileLink {
    /// 主键ID
    pub id: Option<i64>,
    /// 业务唯一标识符
    pub biz_id: Option<Uuid>,
    /// 原文件唯一标识符
    pub origin_file_id: Option<Uuid>,
    /// 预览文件唯一标识符
    pub file_id: Option<Uuid>,
    /// 是否删除
    pub is_del: Option<bool>
}

crud!(BizFileLink {});
impl_select!(BizFileLink{select_by_biz_and_file(biz_id:&Uuid,file_id:&Uuid) -> Option => "`where biz_id = #{biz_id} and (file_id = #{file_id} or origin_file_id = #{file_id})`"});
impl_select!(BizFileLink{select_by_biz(biz_id:&Uuid) => "`where biz_id = #{biz_id} limit 100`"});