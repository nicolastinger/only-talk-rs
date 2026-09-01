use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::crud;
use serde::{Deserialize, Serialize};

/// 登录事件类型
pub const LOGIN_EVENT_SUCCESS: &str = "success";
pub const LOGIN_EVENT_PASSWORD_FAIL: &str = "password_fail";
pub const LOGIN_EVENT_ACCOUNT_NOT_FOUND: &str = "account_not_found";
pub const LOGIN_EVENT_REFRESH: &str = "refresh";

/// 登录渠道
pub const LOGIN_TYPE_ACCOUNT: &str = "account";
pub const LOGIN_TYPE_EMAIL: &str = "email";
pub const LOGIN_TYPE_REFRESH: &str = "refresh";

/// 用户登录审计流水表（记录每次登录/刷新 token 事件，用于安全审计与风控）
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserLoginLog {
    /// 主键ID
    pub id: Option<i64>,
    /// 关联 basic_user.uuid（账号不存在时为空，用户删除后置空保留审计）
    pub uuid: Option<Uuid>,
    /// 登录时提交的账号（失败/不存在也保留，便于审计）
    pub account: Option<String>,
    /// 登录渠道: account / email / refresh
    pub login_type: Option<String>,
    /// 事件类型: success / password_fail / account_not_found / refresh
    pub event_type: Option<String>,
    /// 事件时间（Unix时间戳，毫秒）
    pub login_at: Option<i64>,
    /// 登录平台: PC / MOBILE
    pub platform: Option<String>,
    /// 客户端IPv4地址
    pub ipv4: Option<String>,
    /// 客户端IPv6地址
    pub ipv6: Option<String>,
    /// 客户端 User-Agent
    pub user_agent: Option<String>,
    /// 设备指纹/名称（预留）
    pub device: Option<String>,
    /// 结果补充（如刷新失败原因）
    pub result: Option<String>,
}

crud!(UserLoginLog {});

impl UserLoginLog {
    #[rbatis::py_sql("select * from user_login_log where uuid = #{uuid} order by login_at desc")]
    async fn select_by_uuid(rb: &dyn Executor, uuid: &Uuid) -> Vec<UserLoginLog> {}

    #[rbatis::py_sql("select * from user_login_log where uuid = #{uuid} order by login_at desc limit #{limit}")]
    async fn select_recent_by_uuid(rb: &dyn Executor, uuid: &Uuid, limit: i64) -> Vec<UserLoginLog> {}
}
