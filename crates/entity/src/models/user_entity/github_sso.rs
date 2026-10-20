use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// GitHub 登录渠道（1:1 关联 basic_user，免密 OAuth）
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct GithubSso {
    /// 主键，关联 basic_user.uuid
    pub uuid: Option<Uuid>,
    /// GitHub 用户 ID
    pub github_id: Option<i64>,
    /// GitHub 登录名
    pub login: Option<String>,
    /// GitHub 显示名
    pub name: Option<String>,
    /// GitHub 头像地址
    pub avatar_url: Option<String>,
    /// GitHub 公开邮箱
    pub email: Option<String>,
    /// GitHub 身份已验证
    pub verified: Option<bool>,
    /// 渠道状态: 0=未激活/禁用 1=正常 2=已解绑
    pub status: Option<i16>,
    /// 最近一次该渠道登录时间
    pub last_login_at: Option<i64>,
    /// 最近登录IP
    pub last_login_ip: Option<String>,
    /// 累计登录次数
    pub login_count: Option<i64>,
    /// 创建时间
    pub created_at: Option<i64>,
    /// 更新时间
    pub updated_at: Option<i64>,
    /// 软删除时间
    pub deleted_at: Option<i64>,
}

crud!(GithubSso {});

impl GithubSso {
    #[rbatis::py_sql("select * from github_sso where github_id = #{github_id} limit 1")]
    async fn select_by_github_id_inner(rb: &dyn Executor, github_id: i64) -> Vec<GithubSso> {}

    pub async fn select_by_github_id(
        rb: &dyn Executor,
        github_id: i64,
    ) -> rbatis::Result<Option<GithubSso>> {
        Ok(Self::select_by_github_id_inner(rb, github_id).await?.into_iter().next())
    }

    #[rbatis::py_sql("select * from github_sso where uuid = #{uuid} limit 1")]
    async fn select_by_uuid_inner(rb: &dyn Executor, uuid: &Uuid) -> Vec<GithubSso> {}

    pub async fn select_by_uuid(
        rb: &dyn Executor,
        uuid: &Uuid,
    ) -> rbatis::Result<Option<GithubSso>> {
        Ok(Self::select_by_uuid_inner(rb, uuid).await?.into_iter().next())
    }
}
