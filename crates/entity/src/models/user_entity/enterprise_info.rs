use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 企业用户扩展信息表 (逻辑关联 basic_user.uuid, user_type=2)
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct EnterpriseInfo {
    /// 企业用户唯一标识符 (UUID, 逻辑关联 basic_user.uuid)
    pub uuid: Option<Uuid>,
    /// 企业名称
    pub company_name: Option<String>,
    /// 统一社会信用代码
    pub credit_code: Option<String>,
    /// 法人代表
    pub legal_person: Option<String>,
    /// 联系人
    pub contact_name: Option<String>,
    /// 联系电话
    pub contact_phone: Option<String>,
    /// 联系邮箱
    pub contact_email: Option<String>,
    /// 企业地址
    pub address: Option<String>,
    /// 是否认证 (0: 未认证, 1: 已认证)
    pub verified: Option<i16>,
    /// 状态 (0: 正常, 1: 禁用, 2: 注销等)
    pub status: Option<i16>,
    /// 备注
    pub note: Option<String>,
    /// 记录创建时间 (Unix 时间戳，单位：秒)
    pub created_at: Option<i64>,
    /// 记录最后更新时间 (Unix 时间戳，单位：秒)
    pub updated_at: Option<i64>,
}

crud!(EnterpriseInfo {});

impl EnterpriseInfo {
    #[rbatis::py_sql("select * from enterprise_info where uuid = #{uuid} limit 1")]
    async fn select_by_uuid_inner(rb: &dyn Executor, uuid: &Uuid) -> Vec<EnterpriseInfo> {}

    pub async fn select_by_uuid(
        rb: &dyn Executor,
        uuid: &Uuid,
    ) -> rbatis::Result<Option<EnterpriseInfo>> {
        Ok(Self::select_by_uuid_inner(rb, uuid).await?.into_iter().next())
    }

    pub async fn update_by_uuid(
        rb: &dyn Executor,
        table: &EnterpriseInfo,
        uuid: &Uuid,
    ) -> Result<rbatis::rbdc::db::ExecResult, rbatis::rbdc::Error> {
        EnterpriseInfo::update_by_map(rb, table, rbs::value! {"uuid": uuid}).await
    }
}
