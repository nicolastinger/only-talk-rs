use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

/// 私密文件上传业务表
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivateBizRecord {
    /// 主键ID
    pub id: Option<i64>,
    /// 业务唯一标识符
    pub uuid: Option<Uuid>,
    /// 鉴权策略id
    pub policy_id: Option<String>,
    /// 业务名称
    pub biz_name: Option<String>,
    /// 业务描述
    pub description: Option<String>,
    /// 创建用户UUID
    pub created_by: Option<Uuid>,
    /// 创建时间（Unix时间戳，毫秒）
    pub created_at: Option<i64>,
    /// 更新时间（Unix时间戳，毫秒）
    pub updated_at: Option<i64>,
    /// 业务状态（0-正常，1-已禁用，2-已删除）
    pub status: Option<i32>,
    /// 审批状态(0-待处理,1-已通过,2-已拒绝)
    pub approve_status: Option<i32>,
    /// 业务类型(朋友圈、收藏、私密相册等)
    pub biz_type: Option<String>,
    /// 备注信息
    pub remark: Option<String>,
}

crud!(PrivateBizRecord {});
impl_select!(PrivateBizRecord{select_by_uuid(uuid:&Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
