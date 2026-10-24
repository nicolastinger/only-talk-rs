//! 用户手动状态(presence)。
//!
//! 语义: "手动状态/签名"(忙碌 / 离开 / 学习中 / 奋斗中 / 自定义文案), **与在线离线无关**。
//!
//! - 只存 Redis(`USER:PRESENCE:{UUID}` = JSON), 不落库;
//! - TTL 48 小时, 到期自动回到"无状态"; 客户端据 `expires_at` 展示剩余时间;
//! - 只对好友可见: 批量查询前先校验好友关系, 非好友 uuid 一律忽略;
//! - 无实时推送: 好友侧在拉取好友状态时读取最新值。

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use anyhow::anyhow;
use common::config_str::{REDIS_USER_PRESENCE, USER_PRESENCE_TTL_SECS};
use common::utils::time::get_now_time_stamp_as_millis;
use deadpool_redis::Pool;
use deadpool_redis::redis::{AsyncCommands, cmd};
use rbatis::RBatis;
use rbatis::rbdc::Uuid as RbdcUuid;
use rbs::value;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::http_service::user_service::vo::presence_vo::PresenceVO;
use crate::utils::http_response::CommonResponseRef;

/// 无状态(清除状态)对应的 mode
pub const PRESENCE_MODE_ONLINE: &str = "online";
/// 允许的状态枚举
pub const PRESENCE_MODES: [&str; 6] = ["online", "busy", "away", "studying", "working", "custom"];
/// 自定义状态最大字符数
pub const PRESENCE_MAX_TEXT_CHARS: usize = 32;
/// 单次批量查询的最大 uuid 数量
pub const PRESENCE_BATCH_LIMIT: usize = 200;

/// Redis 中的状态载荷(与 `PresenceVO` 的区别: 多存 `updated_at` 便于排查)
#[derive(Serialize, Deserialize, Debug)]
struct PresencePayload {
    mode: String,
    text: Option<String>,
    updated_at: i64,
    expires_at: i64,
}

/// 好友关系查询结果行
#[derive(Deserialize, Debug)]
struct FriendPeerRow {
    peer: RbdcUuid,
}

/// 状态在 Redis 中的键(统一大写, 与其它用户维度 key 保持一致)
fn presence_key(uuid: &str) -> String {
    format!("{}{}", REDIS_USER_PRESENCE, uuid.trim()).to_uppercase()
}

/// 校验并归一化状态枚举
fn normalize_mode(mode: &str) -> Result<String, anyhow::Error> {
    let mode = mode.trim().to_lowercase();
    if !PRESENCE_MODES.contains(&mode.as_str()) {
        return Err(anyhow!("不支持的状态类型: {}", mode));
    }
    Ok(mode)
}

/// 清洗自定义文案: 去掉控制字符(换行/制表等)、首尾空白, 并按字符截断到上限
fn normalize_text(text: Option<&str>) -> Option<String> {
    let cleaned: String = text
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_string();
    if cleaned.is_empty() {
        return None;
    }
    Some(cleaned.chars().take(PRESENCE_MAX_TEXT_CHARS).collect())
}

/// 读取并解析状态载荷; 非法 JSON / 已过期一律视为无状态
fn parse_payload(raw: &str, now: i64) -> Option<PresencePayload> {
    let payload: PresencePayload = serde_json::from_str(raw).ok()?;
    if payload.expires_at <= now {
        return None;
    }
    Some(payload)
}

fn to_vo(payload: PresencePayload) -> PresenceVO {
    PresenceVO { mode: payload.mode, text: payload.text, expires_at: payload.expires_at }
}

/// 无状态时的返回值(与"从未设置"、"已过期"统一)
fn empty_presence() -> PresenceVO {
    PresenceVO { mode: PRESENCE_MODE_ONLINE.to_string(), text: None, expires_at: 0 }
}

/// 读取指定用户的状态(内部用, 无状态返回 `None`)
async fn read_presence(redis: &Pool, uuid: &str) -> Result<Option<PresencePayload>, anyhow::Error> {
    let mut conn = redis.get().await?;
    let raw: Option<String> = conn.get(presence_key(uuid)).await?;
    let now = get_now_time_stamp_as_millis()?;
    Ok(raw.as_deref().and_then(|raw| parse_payload(raw, now)))
}

/// 设置当前用户状态; `mode=online` 视为清除状态。
pub async fn set_presence(
    redis: &Pool,
    uuid: &str,
    mode: &str,
    text: Option<&str>,
) -> Result<String, anyhow::Error> {
    let mode = normalize_mode(mode)?;
    let key = presence_key(uuid);

    if mode == PRESENCE_MODE_ONLINE {
        let mut conn = redis.get().await?;
        let _: () = conn.del(&key).await?;
        info!("用户清除手动状态: uuid={}", uuid);
        return Ok(CommonResponseRef::success_json(&empty_presence())?);
    }

    // 自定义状态必须有文案; 其它枚举不保存文案
    let text = normalize_text(text);
    if mode == "custom" && text.is_none() {
        return Err(anyhow!("自定义状态内容不能为空"));
    }
    let text = if mode == "custom" { text } else { None };

    let now = get_now_time_stamp_as_millis()?;
    let payload = PresencePayload {
        mode,
        text,
        updated_at: now,
        expires_at: now + (USER_PRESENCE_TTL_SECS as i64) * 1000,
    };
    let raw = serde_json::to_string(&payload)?;

    let mut conn = redis.get().await?;
    conn.set_ex::<_, _, ()>(&key, &raw, USER_PRESENCE_TTL_SECS).await?;
    info!(
        "用户更新手动状态: uuid={}, mode={}, ttl={}s",
        uuid, payload.mode, USER_PRESENCE_TTL_SECS
    );

    Ok(CommonResponseRef::success_json(&to_vo(payload))?)
}

/// 读取当前用户自己的状态(无状态时返回 `mode=online`)
pub async fn get_my_presence(redis: &Pool, uuid: &str) -> Result<String, anyhow::Error> {
    let payload = read_presence(redis, uuid).await?;
    let vo = match payload {
        Some(payload) => to_vo(payload),
        None => empty_presence(),
    };
    Ok(CommonResponseRef::success_json(&vo)?)
}

/// 批量查询好友状态: 只返回确为好友且当前有状态的用户
pub async fn get_friend_presence(
    redis: &Pool,
    rb: &RBatis,
    me: &str,
    uuids: &[String],
) -> Result<String, anyhow::Error> {
    let mut result: HashMap<String, PresenceVO> = HashMap::new();

    let me_uuid = RbdcUuid::from_str(me.trim())?;

    // 去重 + 过滤非法 uuid, 并限制单次数量
    let mut seen: HashSet<String> = HashSet::new();
    let mut candidates: Vec<RbdcUuid> = Vec::new();
    for raw in uuids.iter().take(PRESENCE_BATCH_LIMIT) {
        let trimmed = raw.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_lowercase()) {
            continue;
        }
        if let Ok(peer) = RbdcUuid::from_str(trimmed) {
            candidates.push(peer);
        }
    }
    if candidates.is_empty() {
        return Ok(CommonResponseRef::success_json(&result)?);
    }

    // 一次性筛出"确实是我的好友"的 uuid, 避免越权读取他人状态
    let placeholders =
        (2..=candidates.len() + 1).map(|index| format!("${}", index)).collect::<Vec<_>>().join(",");
    let sql = format!(
        "select case when request_user = $1 then accept_user else request_user end as peer \
         from friend_link \
         where coalesce(is_del, false) = false \
           and ((request_user = $1 and accept_user in ({placeholders})) \
             or (accept_user = $1 and request_user in ({placeholders})))"
    );
    let mut args = vec![value!(me_uuid)];
    for candidate in &candidates {
        args.push(value!(candidate));
    }
    let friends: Vec<FriendPeerRow> = rb.exec_decode(&sql, args).await?;
    if friends.is_empty() {
        return Ok(CommonResponseRef::success_json(&result)?);
    }

    // 批量取状态(MGET, 一次往返)
    let keys: Vec<String> = friends.iter().map(|row| presence_key(&row.peer.to_string())).collect();
    let mut conn = redis.get().await?;
    let raws: Vec<Option<String>> = cmd("MGET").arg(&keys).query_async(&mut conn).await?;

    let now = get_now_time_stamp_as_millis()?;
    for (row, raw) in friends.iter().zip(raws) {
        if let Some(vo) = raw.as_deref().and_then(|raw| parse_payload(raw, now)).map(to_vo) {
            result.insert(row.peer.to_string(), vo);
        }
    }

    Ok(CommonResponseRef::success_json(&result)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_key_is_upper_case() {
        assert_eq!(presence_key(" ab-cd "), "USER:PRESENCE:AB-CD");
    }

    #[test]
    fn normalize_mode_accepts_known_modes() {
        for mode in ["busy", "away", "studying", "working", "custom", "online"] {
            assert_eq!(normalize_mode(mode).expect("合法状态应被接受"), mode);
        }
        assert_eq!(normalize_mode(" BUSY ").expect("应容忍大小写与空白"), "busy");
    }

    #[test]
    fn normalize_mode_rejects_unknown() {
        assert!(normalize_mode("invisible").is_err());
        assert!(normalize_mode("").is_err());
    }

    #[test]
    fn normalize_text_strips_control_chars_and_truncates() {
        assert_eq!(normalize_text(Some(" 学习\n中 ")), Some("学习中".to_string()));
        assert_eq!(normalize_text(Some("   ")), None);
        assert_eq!(normalize_text(None), None);

        let long = "状".repeat(PRESENCE_MAX_TEXT_CHARS + 8);
        let cleaned = normalize_text(Some(&long)).expect("长文本应被截断而非丢弃");
        assert_eq!(cleaned.chars().count(), PRESENCE_MAX_TEXT_CHARS);
    }

    #[test]
    fn parse_payload_drops_expired() {
        let raw = r#"{"mode":"busy","text":null,"updated_at":1000,"expires_at":2000}"#;
        assert!(parse_payload(raw, 1500).is_some());
        assert!(parse_payload(raw, 2000).is_none());
        assert!(parse_payload("not-json", 0).is_none());
    }

    #[test]
    fn presence_ttl_is_48_hours() {
        assert_eq!(USER_PRESENCE_TTL_SECS, 48 * 60 * 60);
    }
}
