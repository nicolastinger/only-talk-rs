use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 二次认证因素类型
pub const AUTH_FACTOR_TYPE_EMAIL: i16 = 0;
pub const AUTH_FACTOR_TYPE_PHONE: i16 = 1;
pub const AUTH_FACTOR_TYPE_OTHER: i16 = 2;

/// 二次认证因素状态
pub const AUTH_FACTOR_STATUS_DISABLED: i16 = 0;
pub const AUTH_FACTOR_STATUS_NORMAL: i16 = 1;
pub const AUTH_FACTOR_STATUS_UNBOUND: i16 = 2;

/// 二次认证因素表(1 用户多因素, 逻辑关联 basic_user.uuid, 同类型至多一条)。
///
/// 当前仅 email 渠道接入(绑定需邮箱验证码), phone/other 预留。
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserAuthFactor {
    /// 主键ID
    pub id: Option<i64>,
    /// 关联 basic_user.uuid
    pub user_id: Option<Uuid>,
    /// 因素类型: 0=email 1=phone 2=other
    pub factor_type: Option<i16>,
    /// 因素值(邮箱/手机号/其他标识)
    pub factor_value: Option<String>,
    /// 是否已验证
    pub verified: Option<bool>,
    /// 是否启用
    pub enabled: Option<bool>,
    /// 是否主因素
    pub is_primary: Option<bool>,
    /// 因素状态: 0=禁用 1=正常 2=已解绑
    pub status: Option<i16>,
    /// 验证通过时间(Unix毫秒)
    pub verified_at: Option<i64>,
    /// 最近使用时间(Unix毫秒)
    pub last_used_at: Option<i64>,
    /// 创建时间(Unix毫秒)
    pub created_at: Option<i64>,
    /// 更新时间(Unix毫秒)
    pub updated_at: Option<i64>,
    /// 软删除时间(预留)
    pub deleted_at: Option<i64>,
}

crud!(UserAuthFactor {});

impl UserAuthFactor {
    #[rbatis::py_sql("select * from user_auth_factor where id = #{id} limit 1")]
    async fn select_by_id_inner(rb: &dyn Executor, id: i64) -> Vec<UserAuthFactor> {}

    pub async fn select_by_id(
        rb: &dyn Executor,
        id: i64,
    ) -> rbatis::Result<Option<UserAuthFactor>> {
        Ok(Self::select_by_id_inner(rb, id).await?.into_iter().next())
    }

    #[rbatis::py_sql(
        "select * from user_auth_factor where user_id = #{user_id} and factor_type = #{factor_type} limit 1"
    )]
    async fn select_by_user_and_type_inner(
        rb: &dyn Executor,
        user_id: &Uuid,
        factor_type: i16,
    ) -> Vec<UserAuthFactor> {
    }

    pub async fn select_by_user_and_type(
        rb: &dyn Executor,
        user_id: &Uuid,
        factor_type: i16,
    ) -> rbatis::Result<Option<UserAuthFactor>> {
        Ok(Self::select_by_user_and_type_inner(rb, user_id, factor_type).await?.into_iter().next())
    }

    #[rbatis::py_sql(
        "select * from user_auth_factor where user_id = #{user_id} order by factor_type asc, id asc"
    )]
    async fn select_by_user_id_inner(rb: &dyn Executor, user_id: &Uuid) -> Vec<UserAuthFactor> {}

    pub async fn select_by_user_id(
        rb: &dyn Executor,
        user_id: &Uuid,
    ) -> rbatis::Result<Vec<UserAuthFactor>> {
        Self::select_by_user_id_inner(rb, user_id).await
    }
}
