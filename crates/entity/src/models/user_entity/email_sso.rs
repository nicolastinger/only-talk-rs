use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
use serde::{Deserialize, Serialize};

/// 邮箱登录渠道（1:1 关联 basic_user）
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct EmailSso {
    /// 主键，关联 basic_user.uuid
    pub uuid: Option<Uuid>,
    /// 邮箱原始值（展示/发件）
    pub email: Option<String>,
    /// 规范化邮箱（小写），唯一
    pub email_normalized: Option<String>,
    /// 是否已验证
    pub verified: Option<bool>,
    /// 验证通过时间
    pub verified_at: Option<i64>,
    /// 最近一次验证码下发时间
    pub verify_code_issued_at: Option<i64>,
    /// 是否主邮箱
    pub is_primary: Option<bool>,
    /// 渠道状态: 0=未激活/禁用 1=正常 2=已解绑
    pub status: Option<i16>,
    /// 最近一次该邮箱登录时间
    pub last_login_at: Option<i64>,
    /// 最近登录IP
    pub last_login_ip: Option<String>,
    /// 累计登录次数
    pub login_count: Option<i64>,
    /// 连续验证失败次数
    pub fail_count: Option<i64>,
    /// 锁定截止时间
    pub locked_until: Option<i64>,
    /// 创建时间
    pub created_at: Option<i64>,
    /// 更新时间
    pub updated_at: Option<i64>,
    /// 软删除时间
    pub deleted_at: Option<i64>,
}

crud!(EmailSso {});

impl EmailSso {
    #[rbatis::py_sql("select * from email_sso where email_normalized = #{email_normalized} limit 1")]
    async fn select_by_email_normalized(rb: &dyn Executor, email_normalized: &str) -> Option<EmailSso> {}

    #[rbatis::py_sql("select * from email_sso where uuid = #{uuid} limit 1")]
    async fn select_by_uuid(rb: &dyn Executor, uuid: &Uuid) -> Option<EmailSso> {}
}
