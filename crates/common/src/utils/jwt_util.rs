use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use once_cell::sync::OnceCell;
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::EncodePrivateKey;
use serde::{Deserialize, Serialize};

use crate::utils::rsa_util::get_rsa_keys;
use crate::utils::time::get_now_time_stamp_as_secs;

// 定义 JWT Claims 结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // 扩展信息
    pub uuid: String, // 用户唯一 ID
    pub exp: i64,     // 过期时间(Unix 时间戳)
    /// 登录会话 ID：每次签发一个随机值。
    /// 同一设备断网重连会复用旧 token（jti 不变），可用它区分
    /// “同一会话顶替自己”和“另一台同平台设备抢登录”。
    /// `serde(default)` 兼容存量旧 token（无该字段时视为空会话）。
    #[serde(default)]
    pub jti: String,
}

/// 进程级缓存：只组装一次 EncodingKey/DecodingKey，避免每次签发/校验都解析 PEM 重建密钥结构
static JWT_KEYS: OnceCell<(EncodingKey, DecodingKey)> = OnceCell::new();

fn load_jwt_keys() -> Result<&'static (EncodingKey, DecodingKey), anyhow::Error> {
    JWT_KEYS.get_or_try_init(|| {
        let (private_key, public_key) = get_rsa_keys()?;
        let private_key_pem = private_key.to_pkcs8_pem(Default::default())?;
        let private_key_str = private_key_pem.to_string();
        // 将公钥转换为 PEM 格式字符串
        let public_key_pem = public_key.to_pkcs1_pem(Default::default())?;
        let public_key_str = public_key_pem.to_string();

        // 创建 EncodingKey 和 DecodingKey
        let encoding_key = EncodingKey::from_rsa_pem(private_key_str.as_ref())?;
        let decoding_key = DecodingKey::from_rsa_pem(public_key_str.as_ref())?;

        Ok((encoding_key, decoding_key))
    })
}

pub fn generate_access_token(uuid: String, platform: String) -> Result<String, anyhow::Error> {
    let (encoding_key, _) = load_jwt_keys()?;
    let claims = Claims {
        sub: platform,
        uuid,
        exp: get_now_time_stamp_as_secs()? + (3600 * 24),
        jti: uuid::Uuid::now_v7().to_string(),
    };
    let header = Header::new(jsonwebtoken::Algorithm::RS256);
    let token = encode(&header, &claims, encoding_key)?;
    Ok(token)
}

pub fn generate_token_with_expiry(
    uuid: String,
    platform: String,
    expiry_secs: i64,
) -> Result<String, anyhow::Error> {
    let (encoding_key, _) = load_jwt_keys()?;
    let claims = Claims {
        sub: platform,
        uuid,
        exp: get_now_time_stamp_as_secs()? + expiry_secs,
        jti: uuid::Uuid::now_v7().to_string(),
    };
    let header = Header::new(jsonwebtoken::Algorithm::RS256);
    let token = encode(&header, &claims, encoding_key)?;
    Ok(token)
}

pub fn verify_token(token: &str) -> Result<Claims, anyhow::Error> {
    let (_, decoding_key) = load_jwt_keys()?;
    let validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    let decoded = decode::<Claims>(token, decoding_key, &validation)?;
    Ok(decoded.claims)
}

/// 从会话 ID(jti)中提取签发毫秒时间戳。
/// 新签发的 jti 为 uuid v7(前 48 位为大端毫秒),可用于比较两次登录的先后。
/// 存量 token 的 v4 随机值或空串(旧 token 无 jti)无法比较,返回 None。
pub fn session_id_ms(session_id: &str) -> Option<u64> {
    if session_id.is_empty() {
        return None;
    }
    let uuid = uuid::Uuid::parse_str(session_id).ok()?;
    let bytes = uuid.as_bytes();
    // v7 版本 nibble 固定为 7(位于第 7 个字节的高 4 位)
    if bytes[6] >> 4 != 7 {
        return None;
    }
    let ms = (bytes[0] as u64) << 40
        | (bytes[1] as u64) << 32
        | (bytes[2] as u64) << 24
        | (bytes[3] as u64) << 16
        | (bytes[4] as u64) << 8
        | bytes[5] as u64;
    Some(ms)
}

#[cfg(test)]
mod tests {
    use super::session_id_ms;

    #[test]
    fn session_id_ms_extracts_v7_timestamp_and_orders() {
        let first = uuid::Uuid::now_v7().to_string();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = uuid::Uuid::now_v7().to_string();
        let first_ms = session_id_ms(&first).expect("v7 应能解析出毫秒");
        let second_ms = session_id_ms(&second).expect("v7 应能解析出毫秒");
        assert!(first_ms <= second_ms, "后签发的 jti 不应早于先签发的");
        assert!(second_ms - first_ms >= 1, "两次签发间隔至少 1ms");
    }

    #[test]
    fn session_id_ms_rejects_empty_and_v4() {
        assert_eq!(session_id_ms(""), None);
        assert_eq!(session_id_ms(&uuid::Uuid::new_v4().to_string()), None);
    }
}
