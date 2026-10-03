use std::time::Duration;

use actix_web::HttpRequest;
use anyhow::anyhow;
use common::config_str::{GITHUB_OAUTH_STATE, PC_PLATFORM, REFRESH_TOKEN};
use common::models::user_entity::basic_user::{BasicUser, USER_TYPE_NORMAL};
use common::models::user_entity::github_sso::GithubSso;
use common::models::user_entity::user_info::UserInfo;
use common::models::user_entity::user_login_log::{
    LOGIN_EVENT_SUCCESS, LOGIN_TYPE_GITHUB, UserLoginLog,
};
use common::utils::jwt_util::{generate_access_token, generate_token_with_expiry};
use common::utils::rsa_util::hash_password;
use common::utils::time::get_now_time_stamp_as_millis;
use deadpool_redis::redis::{AsyncCommands, cmd};
use rbatis::{RBatis, rbdc};
use rbs::value;
use serde::Deserialize;
use tracing::{error, info};
use uuid::Uuid;

use crate::http_service::user_service::dto::github_oauth_dto::{
    GithubAuthorizeDTO, GithubOAuthCallbackDTO,
};
use crate::http_service::user_service::service::user_service::{
    extract_client_info, write_login_log,
};
use crate::http_service::user_service::vo::github_oauth_vo::GithubAuthorizeVO;
use crate::http_service::user_service::vo::sign_in_vo::SignInResponseVO;
use crate::utils::http_response::CommonResponseRef;

/// GitHub OAuth 授权地址默认回环回调（与配置 github_oauth.redirect_uri 保持一致）
const GITHUB_AUTHORIZE_ENDPOINT: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_ENDPOINT: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_ENDPOINT: &str = "https://api.github.com/user";
/// GitHub OAuth state 有效时间（秒）
const GITHUB_STATE_TTL_SECS: u64 = 600;

/// GitHub OAuth 换 token 的响应
#[derive(Debug, Deserialize)]
struct GithubTokenResponse {
    access_token: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

/// GitHub 用户信息（api.github.com/user）
#[derive(Debug, Deserialize)]
struct GithubUser {
    id: i64,
    login: String,
    name: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
    #[serde(default)]
    email: Option<String>,
}

fn github_oauth_config() -> Result<(String, String, String), anyhow::Error> {
    let client_id = common::config_manager::get_config("github_oauth.client_id")
        .ok_or(anyhow!("未配置 github_oauth.client_id"))?;
    let client_secret = common::config_manager::get_config("github_oauth.client_secret")
        .ok_or(anyhow!("未配置 github_oauth.client_secret"))?;
    let redirect_uri = common::config_manager::get_config("github_oauth.redirect_uri")
        .ok_or(anyhow!("未配置 github_oauth.redirect_uri"))?;
    Ok((client_id, client_secret, redirect_uri))
}

/// GitHub OAuth 第一步: 生成 state 并返回授权地址
pub async fn github_authorize_url_service(
    redis: &deadpool_redis::Pool,
    dto: GithubAuthorizeDTO,
) -> Result<String, anyhow::Error> {
    let platform = dto.platform.as_ref().ok_or(anyhow!("平台为空"))?;
    if platform != PC_PLATFORM {
        return Err(anyhow!("GitHub 登录仅支持 PC 端"));
    }
    let device_fingerprint = dto.device_fingerprint.as_ref().ok_or(anyhow!("设备指纹为空"))?;

    let (client_id, _, redirect_uri) = github_oauth_config()?;

    // CSRF state: uuid v7, Redis 记录发起方设备指纹(10 分钟有效)
    let state = Uuid::now_v7().to_string();
    let mut conn = redis.get().await?;
    let state_key = format!("{}{}", GITHUB_OAUTH_STATE, state).to_uppercase();
    conn.set_ex::<&str, &str, ()>(&state_key, device_fingerprint, GITHUB_STATE_TTL_SECS).await?;

    let authorize_url = format!(
        "{}?client_id={}&redirect_uri={}&scope={}&state={}&response_type=code",
        GITHUB_AUTHORIZE_ENDPOINT,
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        "read:user",
        urlencoding::encode(&state)
    );

    let vo = GithubAuthorizeVO { authorize_url, state };
    Ok(CommonResponseRef::<GithubAuthorizeVO>::success_json(&vo)?)
}

/// GitHub OAuth 第二步: 校验 state, 换 token, 拉用户信息, 免密自动注册/登录
pub async fn github_oauth_callback_service(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    dto: GithubOAuthCallbackDTO,
    req: &HttpRequest,
) -> Result<String, anyhow::Error> {
    let platform = dto.platform.as_ref().ok_or(anyhow!("平台为空"))?;
    if platform != PC_PLATFORM {
        return Err(anyhow!("GitHub 登录仅支持 PC 端"));
    }
    let device_fingerprint = dto.device_fingerprint.as_ref().ok_or(anyhow!("设备指纹为空"))?;
    let code = dto.code.as_ref().ok_or(anyhow!("授权码为空"))?;
    let state = dto.state.as_ref().ok_or(anyhow!("state为空"))?;

    let (ipv4, ipv6, user_agent) = extract_client_info(req);

    // 1. 校验 state 并单次消费（防 CSRF / 防重放）
    let mut conn = redis.get().await?;
    let state_key = format!("{}{}", GITHUB_OAUTH_STATE, state).to_uppercase();
    let stored_fp: Option<String> = conn.get(&state_key).await?;
    let _: Result<(), _> = conn.del(&state_key).await;
    if stored_fp.as_deref() != Some(device_fingerprint.as_str()) {
        return Err(anyhow!("GitHub 授权校验失败，请重新发起登录"));
    }

    // 2. 用 code 换 access_token
    let (client_id, client_secret, redirect_uri) = github_oauth_config()?;
    let client = reqwest::Client::new();
    let token_resp: GithubTokenResponse = client
        .post(GITHUB_TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| anyhow!("GitHub 授权换取失败: {}", e))?
        .json()
        .await
        .map_err(|e| anyhow!("GitHub 授权响应解析失败: {}", e))?;
    let gh_token = token_resp.access_token.ok_or_else(|| {
        anyhow!(
            "GitHub 授权失败: {}",
            token_resp.error_description.unwrap_or_else(|| "未知错误".to_string())
        )
    })?;

    // 3. 拉取 GitHub 用户信息
    let github_user: GithubUser = client
        .get(GITHUB_USER_ENDPOINT)
        .bearer_auth(&gh_token)
        .header("User-Agent", "only-talk-rs")
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| anyhow!("获取 GitHub 用户信息失败: {}", e))?
        .json()
        .await
        .map_err(|e| anyhow!("解析 GitHub 用户信息失败: {}", e))?;
    info!("GitHub 用户登录: id={} login={}", github_user.id, github_user.login);

    let now = get_now_time_stamp_as_millis()?;

    // 4. 查/建 github_sso（免密自动注册）
    let uuid = match GithubSso::select_by_github_id(rb, github_user.id).await? {
        Some(sso) => sso.uuid.ok_or(anyhow!("GitHub 渠道缺少 uuid"))?,
        None => create_github_user(rb, &github_user, now).await?,
    };

    // 5. 更新渠道登录信息
    let mut sso =
        GithubSso::select_by_uuid(rb, &uuid).await?.ok_or(anyhow!("GitHub 渠道不存在"))?;
    sso.last_login_at = Some(now);
    sso.last_login_ip = ipv4.clone().or_else(|| ipv6.clone());
    sso.login_count = Some(sso.login_count.unwrap_or(0) + 1);
    sso.updated_at = Some(now);
    sso.login = Some(github_user.login.clone());
    sso.name = github_user.name.clone();
    sso.avatar_url = github_user.avatar_url.clone();
    sso.email = github_user.email.clone();
    GithubSso::update_by_map(rb, &sso, value! { "uuid": &uuid }).await?;

    // 6. 签发 token（与账号密码登录一致的逻辑）
    let uuid_str = uuid.to_string();
    let access_token = generate_access_token(uuid_str.clone(), platform.clone())?;
    let refresh_token =
        generate_token_with_expiry(uuid_str.clone(), platform.clone(), 3600 * 24 * 30)?;
    let rt_key = format!("{}{}", REFRESH_TOKEN, refresh_token).to_uppercase();
    let _: () = cmd("SET")
        .arg(&rt_key)
        .arg(device_fingerprint)
        .arg("EX")
        .arg(3600 * 24 * 30)
        .query_async(&mut conn)
        .await?;

    // 7. 登录成功审计
    let account_str = format!("gh_{}", github_user.id);
    write_login_log(
        rb,
        UserLoginLog {
            id: None,
            uuid: Some(uuid),
            account: Some(account_str),
            login_type: Some(LOGIN_TYPE_GITHUB.to_string()),
            event_type: Some(LOGIN_EVENT_SUCCESS.to_string()),
            login_at: Some(now),
            platform: Some(platform.clone()),
            ipv4,
            ipv6,
            user_agent,
            device: Some(device_fingerprint.clone()),
            result: None,
        },
    )
    .await;

    let sign_in_vo = SignInResponseVO { access_token, refresh_token };
    Ok(CommonResponseRef::<SignInResponseVO>::success_json(&sign_in_vo)?)
}

/// 首次 GitHub 登录: 事务创建 basic_user + user_info + github_sso（免密，注册即完成）
async fn create_github_user(
    rb: &RBatis,
    github_user: &GithubUser,
    now: i64,
) -> Result<rbdc::Uuid, anyhow::Error> {
    let uuid: rbdc::Uuid = Uuid::now_v7().to_string().parse()?;
    let account = format!("gh_{}", github_user.id);
    // 密码列 NOT NULL，仅存随机占位哈希，OAuth 免密登录不校验
    let placeholder_password = hash_password(&Uuid::now_v7().to_string())?;

    let tx = rb.acquire_begin().await?;
    let result: Result<(), anyhow::Error> = async {
        let basic_user = BasicUser {
            uuid: Some(uuid.clone()),
            username: Some(github_user.login.clone()),
            account: Some(account),
            icon: github_user.avatar_url.clone(),
            info: Some("".to_string()),
            password: Some(placeholder_password),
            registration_status: Some(1),
            user_type: Some(USER_TYPE_NORMAL),
        };

        let user_info = UserInfo {
            uuid: Some(uuid.clone()),
            gender: None,
            birthday: Some(0),
            note: Some("这个人很勤快，但什么都没写".to_string()),
            created_at: Some(now),
            updated_at: Some(now),
            phone: None,
            email: github_user.email.clone(),
            address: None,
            status: None,
        };

        let github_sso = GithubSso {
            uuid: Some(uuid.clone()),
            github_id: Some(github_user.id),
            login: Some(github_user.login.clone()),
            name: github_user.name.clone(),
            avatar_url: github_user.avatar_url.clone(),
            email: github_user.email.clone(),
            verified: Some(true),
            status: Some(1),
            last_login_at: Some(now),
            last_login_ip: None,
            login_count: Some(1),
            created_at: Some(now),
            updated_at: Some(now),
            deleted_at: None,
        };

        BasicUser::insert(&tx, &basic_user).await?;
        UserInfo::insert(&tx, &user_info).await?;
        GithubSso::insert(&tx, &github_sso).await?;

        tx.commit().await?;
        Ok(())
    }
    .await;

    if result.is_err() {
        let _ = tx.rollback().await;
        error!("创建 GitHub 用户失败: {:?}", result.err());
        return Err(anyhow!("创建 GitHub 用户失败"));
    }
    Ok(uuid)
}
