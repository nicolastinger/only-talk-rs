use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

/// 文件上传业务表
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BizRecord {
    /// 主键ID
    pub id: Option<i64>,
    /// 业务唯一标识符
    pub uuid: Option<Uuid>,
    /// 业务名称
    pub biz_name: Option<String>,
    /// 业务描述
    pub description: Option<String>,
    /// 关联的文件UUID
    pub file_uuid: Option<Uuid>,
    /// 创建用户UUID
    pub creator_uuid: Option<Uuid>,
    /// 创建时间（Unix时间戳，毫秒）
    pub create_time: Option<i64>,
    /// 更新时间（Unix时间戳，毫秒）
    pub update_time: Option<i64>,
    /// 业务状态（0-正常，1-已禁用，2-已删除）
    pub status: Option<i32>,
    /// 业务类型
    pub biz_type: Option<String>,
    /// 备注信息
    pub remark: Option<String>,
}

crud!(BizRecord {});
impl_select!(BizRecord{select_by_uuid(uuid:&Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});