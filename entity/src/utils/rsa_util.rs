use std::fs;
use anyhow::anyhow;
use rand::Rng;
use rand::distributions::Alphanumeric;
use rbatis::rbatis_codegen::ops::AsProxy;
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs1::EncodeRsaPublicKey;
use sha2::{Digest, Sha256};
use tracing::info;
use crate::config_manager::{get_config, set_config};

pub fn generate_rsa_keys() -> Result<(RsaPrivateKey, RsaPublicKey), anyhow::Error> {
    let private_key_config = get_config("jwt_private_key");
    let public_key_config = get_config("jwt_public_key");
    
    let (private_key_str, public_key_str) = if private_key_config.is_some() && public_key_config.is_some() {
        let private_key_str = private_key_config.ok_or(anyhow!("jwt_private_key 配置不存在"))?;
        let public_key_str = public_key_config.ok_or(anyhow!("jwt_public_key 配置不存在"))?;
        (private_key_str, public_key_str)
    } else {
        let private_key_text = fs::read_to_string("./config/jwt/private.key");
        let public_key_text = fs::read_to_string("./config/jwt/public.key");
        if private_key_text.is_err() || public_key_text.is_err() {
            let (private_key, public_key) = new_rsa_key()?;
            return Ok((private_key, public_key));
        }
        let private_key_str = private_key_text?;
        let public_key_str = public_key_text?;
        set_config("jwt_private_key".string(), private_key_str.clone());
        set_config("jwt_public_key".string(), public_key_str.clone());
        (private_key_str, public_key_str)
    };
    
    if private_key_str.len() > 100 && public_key_str.len() > 50 {
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_str.as_str())?;
        let public_key = RsaPublicKey::from(&private_key);
        return Ok((private_key, public_key));
    };
    let (private_key, public_key) = new_rsa_key()?;
    Ok((private_key, public_key))
}

fn new_rsa_key() -> Result<(RsaPrivateKey, RsaPublicKey), anyhow::Error> {
    // 如果没有现有的密钥文件，则生成新的 RSA 密钥对
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");

    // 从私钥派生出公钥
    let public_key = RsaPublicKey::from(&private_key);
    let private_key_pem = private_key.to_pkcs8_pem(Default::default())?;
    let private_key_str = private_key_pem.to_string();
    // 将公钥转换为 PEM 格式的字符串
    let public_key_pem = public_key.to_pkcs1_pem(Default::default())?;
    let public_key_str = public_key_pem.to_string();
    set_config("jwt_private_key".string(), private_key_str.clone());
    set_config("jwt_public_key".string(), public_key_str.clone());
    // 保存生成的密钥到文件
    fs::write("./config/jwt/private.key", private_key_str)?;
    fs::write("./config/jwt/public.key", public_key_str)?;
    Ok((private_key, public_key))
}

// 生成一个指定长度的随机字符串
pub fn generate_random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    std::iter::repeat(())
        .map(|_| rng.sample(Alphanumeric))
        .map(|num| num as char) // 将 u8 转换为 char
        .take(length)
        .collect::<String>()
}

pub fn hash_with_salt(data: &str, salt: &str) -> String {
    // 创建一个新的 SHA-256 哈希器
    let mut hasher = Sha256::new();

    // 将数据和盐一起传递给哈希器
    hasher.update(data);
    hasher.update(salt);

    // 获取最终的哈希值并转换为十六进制字符串
    let result = hasher.finalize();
    format!("{:x}", result)
}
