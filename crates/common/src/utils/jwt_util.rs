use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::EncodePrivateKey;
use serde::{Deserialize, Serialize};
use crate::utils::rsa_util::generate_rsa_keys;
use crate::utils::time::get_now_time_stamp_as_secs;

// 定义 JWT 的 Claims 结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,     // 拓展信息
    pub uuid: String, // 用户唯一id
    pub exp: i64,     // 过期时间 (Unix 时间戳)
}

fn generate_keys() -> Result<(EncodingKey, DecodingKey), anyhow::Error> {
    let (private_key, public_key) = generate_rsa_keys()?;
    let private_key_pem = private_key.to_pkcs8_pem(Default::default())?;
    let private_key_str = private_key_pem.to_string();
    // 将公钥转换为 PEM 格式的字符串
    let public_key_pem = public_key.to_pkcs1_pem(Default::default())?;
    let public_key_str = public_key_pem.to_string();

    // 创建 EncodingKey 和 DecodingKey
    let encoding_key = EncodingKey::from_rsa_pem(private_key_str.as_ref())?;
    let decoding_key = DecodingKey::from_rsa_pem(public_key_str.as_ref())?;

    Ok((encoding_key, decoding_key))
}

pub fn get_jwt(uuid: String, platform: String) -> Result<String, anyhow::Error> {
    let (encoding_key, _) = generate_keys()?;
    let claims =
        Claims { sub: platform, uuid, exp: get_now_time_stamp_as_secs()? + (3600 * 24) };
    let header = Header::new(jsonwebtoken::Algorithm::RS256);
    let token = encode(&header, &claims, &encoding_key)?;
    Ok(token)
}

pub fn get_jwt_with_expiry(uuid: String, platform: String, expiry_secs: i64) -> Result<String, anyhow::Error> {
    let (encoding_key, _) = generate_keys()?;
    let claims = Claims { sub: platform, uuid, exp: get_now_time_stamp_as_secs()? + expiry_secs };
    let header = Header::new(jsonwebtoken::Algorithm::RS256);
    let token = encode(&header, &claims, &encoding_key)?;
    Ok(token)
}

pub fn decode_jwt(token: &str) -> Result<Claims, anyhow::Error> {
    let (_, decoding_key) = generate_keys()?;
    let validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    let decoded = decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(decoded.claims)
}
