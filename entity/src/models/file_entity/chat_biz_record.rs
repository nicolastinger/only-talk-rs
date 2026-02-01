use rbatis::rbdc::Uuid;
use rbatis::{crud, impl_select};
use serde::{Deserialize, Serialize};

/// 聊天文件上传业务表
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatBizRecord {
    /// 主键ID
    pub id: Option<i64>,
    /// 业务唯一标识符
    pub uuid: Option<Uuid>,
    /// 业务名称
    pub biz_name: Option<String>,
    /// 业务描述
    pub description: Option<String>,
    /// 创建用户UUID
    pub created_by: Option<Uuid>,
    /// 接收用户UUID
    pub receiver: Option<Uuid>,
    /// 创建时间（Unix时间戳，毫秒）
    pub created_at: Option<i64>,
    /// 更新时间（Unix时间戳，毫秒）
    pub updated_at: Option<i64>,
    /// 业务状态（0-正常，1-已禁用，2-已删除）
    pub status: Option<i32>,
    /// 审批状态(0-待处理,1-已通过,2-已拒绝)
    pub approve_status: Option<i32>,
    /// 业务类型(私聊、群聊等)
    pub biz_type: Option<String>,
    /// 备注信息
    pub remark: Option<String>,
}

crud!(ChatBizRecord {});
impl_select!(ChatBizRecord{select_by_uuid(uuid:&Uuid) -> Option => "`where uuid = #{uuid} limit 1`"});
