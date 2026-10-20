//! 验证码写入/校验的通用防爆破逻辑。
//!
//! 验证码本身存于 Redis(默认 5 分钟有效); 校验失败时累计错误次数,
//! 达到 [`MAX_VERIFY_CODE_ATTEMPTS`] 后直接删除验证码(key 作废), 需重新获取。

use anyhow::anyhow;
use deadpool_redis::redis::AsyncCommands;

/// 验证码有效期(秒), 与各发码处保持一致。
pub const VERIFY_CODE_TTL_SECS: u64 = 300;

/// 单个验证码允许的最大错误尝试次数, 达到后验证码作废(需重新获取)。
pub const MAX_VERIFY_CODE_ATTEMPTS: u32 = 5;

/// 错误次数计数 key 的后缀(与验证码 key 同 TTL)。
const ATTEMPTS_SUFFIX: &str = ":ATTEMPTS";

/// 错误次数计数 key。
fn attempts_key(code_key: &str) -> String {
    format!("{}{}", code_key, ATTEMPTS_SUFFIX)
}

/// 写入验证码并重置其失败计数(发码时调用)。
pub async fn set_verify_code(
    conn: &mut deadpool_redis::Connection,
    code_key: &str,
    code: &str,
) -> Result<(), anyhow::Error> {
    conn.set_ex::<_, _, ()>(code_key, code, VERIFY_CODE_TTL_SECS).await?;
    let _: Result<(), _> = conn.del(attempts_key(code_key)).await;
    Ok(())
}

/// 校验验证码(带防爆破):
/// - 命中: 删除验证码与失败计数, 返回 `Ok`;
/// - 未命中: 失败计数 +1; 达到 [`MAX_VERIFY_CODE_ATTEMPTS`] 时删除验证码与计数并返回明确错误。
pub async fn verify_code(
    conn: &mut deadpool_redis::Connection,
    code_key: &str,
    input_code: &str,
) -> Result<(), anyhow::Error> {
    let stored: Option<String> = conn.get(code_key).await?;
    if matches!(stored.as_deref(), Some(stored) if stored == input_code) {
        let _: Result<(), _> = conn.del(code_key).await;
        let _: Result<(), _> = conn.del(attempts_key(code_key)).await;
        return Ok(());
    }

    let ak = attempts_key(code_key);
    let attempts: u32 = conn.incr(ak.as_str(), 1).await?;
    if attempts == 1 {
        // 首次失败时给计数 key 设定与验证码一致的有效期
        let _: Result<(), _> = conn.expire(ak.as_str(), VERIFY_CODE_TTL_SECS as i64).await;
    }
    if attempts >= MAX_VERIFY_CODE_ATTEMPTS {
        // 超过上限: 验证码作废
        let _: Result<(), _> = conn.del(code_key).await;
        let _: Result<(), _> = conn.del(ak.as_str()).await;
        return Err(anyhow!("验证码错误次数过多, 请重新获取验证码"));
    }
    Err(anyhow!("验证码错误或已过期"))
}
