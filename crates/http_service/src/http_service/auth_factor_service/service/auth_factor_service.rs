use std::str::FromStr;

use anyhow::anyhow;
use common::config_str::{AUTH_FACTOR_EMAIL_VERIFY_CODE, AUTH_FACTOR_UNBIND_EMAIL_VERIFY_CODE};
use common::models::user_entity::user_auth_factor::{
    AUTH_FACTOR_STATUS_DISABLED, AUTH_FACTOR_STATUS_NORMAL, AUTH_FACTOR_STATUS_UNBOUND,
    AUTH_FACTOR_TYPE_EMAIL, AUTH_FACTOR_TYPE_OTHER, AUTH_FACTOR_TYPE_PHONE, UserAuthFactor,
};
use common::utils::time::get_now_time_stamp_as_millis;
use common::utils::validators::{EMAIL_REGEX, normalize_email};
use deadpool_redis::redis::AsyncCommands;
use email_service::manager::EmailManager;
use email_service::{Email, EmailAddress};
use rand::Rng;
use rbatis::RBatis;
use rbatis::rbdc::Uuid;
use rbs::value;

use crate::http_service::auth_factor_service::dto::auth_factor_dto::{
    CreateAuthFactorDTO, DeleteAuthFactorDTO, SendAuthFactorCodeDTO, UpdateAuthFactorDTO,
};
use crate::http_service::auth_factor_service::vo::auth_factor_vo::{
    AuthFactorListVO, AuthFactorVO,
};
use crate::utils::http_response::{CommonResponseNoDataRef, CommonResponseRef};

fn parse_uuid(v: Option<String>) -> Result<Option<Uuid>, anyhow::Error> {
    match v {
        Some(s) => Ok(Some(Uuid::from_str(&s)?)),
        None => Ok(None),
    }
}

fn to_vo(row: UserAuthFactor) -> AuthFactorVO {
    AuthFactorVO {
        id: row.id.unwrap_or_default(),
        user_id: row.user_id.map(|u| u.to_string()).unwrap_or_default(),
        factor_type: row.factor_type.unwrap_or(AUTH_FACTOR_TYPE_EMAIL),
        factor_value: row.factor_value.unwrap_or_default(),
        verified: row.verified.unwrap_or(false),
        enabled: row.enabled.unwrap_or(false),
        is_primary: row.is_primary.unwrap_or(false),
        status: row.status.unwrap_or(AUTH_FACTOR_STATUS_NORMAL),
        verified_at: row.verified_at,
        last_used_at: row.last_used_at,
        created_at: row.created_at.unwrap_or(0),
        updated_at: row.updated_at.unwrap_or(0),
    }
}

/// 当前仅开放 email 认证渠道
fn ensure_email_channel(factor_type: i16) -> Result<(), anyhow::Error> {
    if factor_type != AUTH_FACTOR_TYPE_EMAIL {
        return Err(anyhow!("暂未开放该认证渠道"));
    }
    Ok(())
}

/// 规范化因素值: email 走小写规范化 + 格式校验, 其余仅去空白/非空
fn normalize_factor_value(factor_type: i16, raw: &str) -> Result<String, anyhow::Error> {
    let v = raw.trim();
    if v.is_empty() {
        return Err(anyhow!("因素值不能为空"));
    }
    match factor_type {
        AUTH_FACTOR_TYPE_EMAIL => {
            if !EMAIL_REGEX.is_match(v) {
                return Err(anyhow!("邮箱格式不正确"));
            }
            Ok(normalize_email(v))
        }
        AUTH_FACTOR_TYPE_PHONE | AUTH_FACTOR_TYPE_OTHER => Ok(v.to_string()),
        _ => Err(anyhow!("不支持的认证因素类型")),
    }
}

fn ensure_owner(row: &UserAuthFactor, me: &Uuid) -> Result<(), anyhow::Error> {
    if row.user_id.as_ref() != Some(me) {
        return Err(anyhow!("无权操作该认证因素"));
    }
    Ok(())
}

/// 发码场景: 解绑(缺省为绑定)
const AUTH_FACTOR_SCENE_UNBIND: &str = "unbind";

/// 发送二次认证因素验证码(当前仅 email)
///
/// - `scene=bind`(缺省): 发往用户输入的 `factor_value`, 已绑定则拒绝;
/// - `scene=unbind`: 发往当前已绑定邮箱(取库中因素值, 不接受用户输入), 未绑定则拒绝。
///
/// 验证码写入 Redis 5 分钟有效(绑定/解绑使用相互隔离的 key)。
pub async fn send_auth_factor_code(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    email_manager: &EmailManager,
    my_uuid: Option<String>,
    dto: SendAuthFactorCodeDTO,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    ensure_email_channel(dto.factor_type)?;

    let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let account_name = common::config_manager::get_config("email.account_name").unwrap_or_default();
    let mut conn = redis.get().await?;

    // 解绑场景: 必须已绑定, 发往已绑定邮箱, key 以用户 uuid
    if dto.scene.as_deref() == Some(AUTH_FACTOR_SCENE_UNBIND) {
        let factor = UserAuthFactor::select_by_user_and_type(rb, &me, AUTH_FACTOR_TYPE_EMAIL)
            .await?
            .ok_or_else(|| anyhow!("未绑定邮箱二次认证, 无需解绑"))?;
        let email = factor.factor_value.ok_or_else(|| anyhow!("认证因素信息缺失"))?;

        let key = format!("{}{}", AUTH_FACTOR_UNBIND_EMAIL_VERIFY_CODE, me).to_uppercase();
        conn.set_ex::<&str, &str, ()>(&key, &code, 300).await?;

        let mail = Email::builder()
            .from(EmailAddress::new(&account_name).map_err(|e| anyhow!("发件人配置错误: {}", e))?)
            .to(EmailAddress::new(&email).map_err(|e| anyhow!("收件人邮箱格式错误: {}", e))?)
            .subject("OnlyTalk 二次认证邮箱解绑验证码")
            .text_body(format!(
                "您的二次认证邮箱解绑验证码是: {},5 分钟内有效,请勿泄露给他人。",
                code
            ))
            .build()
            .map_err(|e| anyhow!("构建邮件失败: {}", e))?;

        let result = email_manager.send(&mail).await.map_err(|e| anyhow!("邮件发送失败: {}", e))?;
        if !result.is_success() {
            let reason = result.error.as_ref().map(|e| e.message.clone()).unwrap_or_default();
            return Err(anyhow!("邮件发送失败: {}", reason));
        }
        return Ok(CommonResponseNoDataRef::success_empty());
    }

    // 绑定场景(缺省): 发往用户输入邮箱, 已绑定则拒绝, key 以邮箱
    let raw = dto.factor_value.as_deref().ok_or_else(|| anyhow!("因素值不能为空"))?;
    let normalized = normalize_factor_value(dto.factor_type, raw)?;

    if UserAuthFactor::select_by_user_and_type(rb, &me, AUTH_FACTOR_TYPE_EMAIL).await?.is_some() {
        return Err(anyhow!("该邮箱二次认证已绑定"));
    }

    let key = format!("{}{}", AUTH_FACTOR_EMAIL_VERIFY_CODE, normalized).to_uppercase();
    conn.set_ex::<&str, &str, ()>(&key, &code, 300).await?;

    let mail = Email::builder()
        .from(EmailAddress::new(&account_name).map_err(|e| anyhow!("发件人配置错误: {}", e))?)
        .to(EmailAddress::new(&normalized).map_err(|e| anyhow!("收件人邮箱格式错误: {}", e))?)
        .subject("OnlyTalk 二次认证邮箱绑定验证码")
        .text_body(format!("您的二次认证邮箱绑定验证码是: {},5 分钟内有效,请勿泄露给他人。", code))
        .build()
        .map_err(|e| anyhow!("构建邮件失败: {}", e))?;

    let result = email_manager.send(&mail).await.map_err(|e| anyhow!("邮件发送失败: {}", e))?;
    if !result.is_success() {
        let reason = result.error.as_ref().map(|e| e.message.clone()).unwrap_or_default();
        return Err(anyhow!("邮件发送失败: {}", reason));
    }

    Ok(CommonResponseNoDataRef::success_empty())
}

/// 新增二次认证因素(email 需邮箱验证码校验通过), 1 用户同类型至多一条
pub async fn create_auth_factor(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    my_uuid: Option<String>,
    dto: CreateAuthFactorDTO,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    ensure_email_channel(dto.factor_type)?;
    let normalized = normalize_factor_value(dto.factor_type, &dto.factor_value)?;

    if UserAuthFactor::select_by_user_and_type(rb, &me, AUTH_FACTOR_TYPE_EMAIL).await?.is_some() {
        return Err(anyhow!("该邮箱二次认证已绑定"));
    }

    // 校验并消费验证码
    let code = dto.verification_code.as_ref().ok_or_else(|| anyhow!("验证码为空"))?;
    let mut conn = redis.get().await?;
    let code_key = format!("{}{}", AUTH_FACTOR_EMAIL_VERIFY_CODE, normalized).to_uppercase();
    let stored: Option<String> = conn.get(&code_key).await?;
    match stored {
        Some(stored) if stored == *code => {
            let _: Result<(), _> = conn.del(&code_key).await;
        }
        _ => return Err(anyhow!("验证码错误或已过期")),
    }

    let now = get_now_time_stamp_as_millis()?;
    let is_first = UserAuthFactor::select_by_user_id(rb, &me).await?.is_empty();
    let factor = UserAuthFactor {
        id: None,
        user_id: Some(me.clone()),
        factor_type: Some(AUTH_FACTOR_TYPE_EMAIL),
        factor_value: Some(normalized),
        verified: Some(true),
        enabled: Some(true),
        is_primary: Some(is_first),
        status: Some(AUTH_FACTOR_STATUS_NORMAL),
        verified_at: Some(now),
        last_used_at: None,
        created_at: Some(now),
        updated_at: Some(now),
        deleted_at: None,
    };
    UserAuthFactor::insert(rb, &factor).await?;

    let inserted = UserAuthFactor::select_by_user_and_type(rb, &me, AUTH_FACTOR_TYPE_EMAIL)
        .await?
        .ok_or_else(|| anyhow!("创建认证因素失败"))?;
    Ok(CommonResponseRef::<AuthFactorVO>::success_json(&to_vo(inserted))?)
}

/// 当前用户的二次认证因素列表
pub async fn list_auth_factors(
    rb: &RBatis,
    my_uuid: Option<String>,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let rows = UserAuthFactor::select_by_user_id(rb, &me).await?;
    let list: Vec<AuthFactorVO> = rows.into_iter().map(to_vo).collect();
    let total = list.len() as u32;
    Ok(CommonResponseRef::<AuthFactorListVO>::success_json(&AuthFactorListVO { total, list })?)
}

/// 二次认证因素详情(仅本人)
pub async fn get_auth_factor_detail(
    rb: &RBatis,
    my_uuid: Option<String>,
    id: i64,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let row =
        UserAuthFactor::select_by_id(rb, id).await?.ok_or_else(|| anyhow!("认证因素不存在"))?;
    ensure_owner(&row, &me)?;
    Ok(CommonResponseRef::<AuthFactorVO>::success_json(&to_vo(row))?)
}

/// 更新二次认证因素(仅本人); 修改因素值会重置验证状态
pub async fn update_auth_factor(
    rb: &RBatis,
    my_uuid: Option<String>,
    dto: UpdateAuthFactorDTO,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    let existing =
        UserAuthFactor::select_by_id(rb, dto.id).await?.ok_or_else(|| anyhow!("认证因素不存在"))?;
    ensure_owner(&existing, &me)?;
    let factor_type = existing.factor_type.ok_or_else(|| anyhow!("认证因素类型缺失"))?;

    let mut updated = existing.clone();
    if let Some(v) = dto.factor_value.as_ref() {
        updated.factor_value = Some(normalize_factor_value(factor_type, v)?);
        updated.verified = Some(false);
        updated.verified_at = None;
    }
    if let Some(enabled) = dto.enabled {
        updated.enabled = Some(enabled);
    }
    if let Some(is_primary) = dto.is_primary {
        updated.is_primary = Some(is_primary);
    }
    if let Some(status) = dto.status {
        if !matches!(
            status,
            AUTH_FACTOR_STATUS_DISABLED | AUTH_FACTOR_STATUS_NORMAL | AUTH_FACTOR_STATUS_UNBOUND
        ) {
            return Err(anyhow!("不支持的认证因素状态"));
        }
        updated.status = Some(status);
    }
    updated.updated_at = Some(get_now_time_stamp_as_millis()?);

    UserAuthFactor::update_by_map(rb, &updated, value! {"id": dto.id}).await?;

    let refreshed =
        UserAuthFactor::select_by_id(rb, dto.id).await?.ok_or_else(|| anyhow!("认证因素不存在"))?;
    Ok(CommonResponseRef::<AuthFactorVO>::success_json(&to_vo(refreshed))?)
}

/// 删除(解绑)二次认证因素(email 需解绑验证码校验通过后物理删除, 仅本人)
pub async fn delete_auth_factor(
    rb: &RBatis,
    redis: &deadpool_redis::Pool,
    my_uuid: Option<String>,
    dto: DeleteAuthFactorDTO,
) -> Result<String, anyhow::Error> {
    let me = parse_uuid(my_uuid)?.ok_or_else(|| anyhow!("Failed to get account"))?;
    ensure_email_channel(dto.factor_type)?;
    let row =
        UserAuthFactor::select_by_id(rb, dto.id).await?.ok_or_else(|| anyhow!("认证因素不存在"))?;
    ensure_owner(&row, &me)?;

    // 校验并消费解绑验证码(键以用户 uuid, 与绑定/改密验证码隔离)
    let code = dto.verification_code.as_ref().ok_or_else(|| anyhow!("验证码为空"))?;
    let mut conn = redis.get().await?;
    let key = format!("{}{}", AUTH_FACTOR_UNBIND_EMAIL_VERIFY_CODE, me).to_uppercase();
    let stored: Option<String> = conn.get(&key).await?;
    match stored {
        Some(stored) if stored == *code => {
            let _: Result<(), _> = conn.del(&key).await;
        }
        _ => return Err(anyhow!("验证码错误或已过期")),
    }

    UserAuthFactor::delete_by_map(rb, value! {"id": dto.id}).await?;
    Ok(CommonResponseNoDataRef::success_empty())
}
