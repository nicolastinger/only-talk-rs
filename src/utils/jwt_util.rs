use crate::utils::time::get_now_time_stamp_as_millis;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use log::info;
use rand::rngs::OsRng;
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;

// 定义 JWT 的 Claims 结构体
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,     // 主题 (subject)
    account: String, // 账号
    exp: i64,        // 过期时间 (Unix 时间戳)
}

fn generate_keys() -> Result<(EncodingKey, DecodingKey), Box<dyn Error>> {
    // 读取私钥和公钥文件
    info!("Generating keys...");
    let private_key_str = fs::read_to_string("config/jwt/private.key")?;
    let public_key_str = fs::read_to_string("config/jwt/public.key")?;

    if private_key_str.len() > 100 && public_key_str.len() > 50 {
        let encoding_key = EncodingKey::from_rsa_pem(private_key_str.as_ref())?;
        let decoding_key = DecodingKey::from_rsa_pem(public_key_str.as_ref())?;
        Ok((encoding_key, decoding_key))
    } else {
        // 如果没有现有的密钥文件，则生成新的 RSA 密钥对
        let mut rng = rand::thread_rng();
        let bits = 2048;
        let private_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
        // 将私钥转换为 PEM 格式的字符串
        let private_key_pem = private_key.to_pkcs8_pem(Default::default())?;
        let private_key_str = private_key_pem.to_string();
        // 从私钥派生出公钥
        let public_key = RsaPublicKey::from(&private_key);

        // 将公钥转换为 PEM 格式的字符串
        let public_key_pem = public_key.to_pkcs1_pem(Default::default())?;
        let public_key_str = public_key_pem.to_string();

        // 创建 EncodingKey 和 DecodingKey
        let encoding_key = EncodingKey::from_rsa_pem(private_key_str.as_ref())?;
        let decoding_key = DecodingKey::from_rsa_pem(public_key_str.as_ref())?;

        // 保存生成的密钥到文件（可选）
        fs::write("config/jwt/private.key", private_key_str)?;
        fs::write("config/jwt/public.key", public_key_str)?;
        Ok((encoding_key, decoding_key))
    }
}

pub fn get_jwt(account: String) -> Result<String, Box<dyn Error>> {
    let (encoding_key, _) = generate_keys()?;
    let claims = Claims {
        sub: "123123".to_string(),
        account,
        exp: get_now_time_stamp_as_millis().unwrap() + (3600000 * 24),
    };
    // 使用 RSA 算法生成 JWT
    let header = Header::new(jsonwebtoken::Algorithm::RS256);
    let token = encode(&header, &claims, &encoding_key)?;
    Ok(token)
}

pub fn decode_jwt(token: String) -> Result<String, Box<dyn Error>> {
    let (_, decoding_key) = generate_keys()?;
    // 使用 RSA 算法解码 JWT
    let validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    let decoded = decode::<Claims>(&token, &decoding_key, &validation)?;
    info!("Decoded claims: {:?}", decoded.claims);
    let now = get_now_time_stamp_as_millis()?;
    match now < decoded.claims.exp {
        true => Ok(decoded.claims.account),
        false => Err("token超时".into())
    }
}
