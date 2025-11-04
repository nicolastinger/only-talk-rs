use rand::distributions::Alphanumeric;
use rand::Rng;
use rsa::pkcs8::DecodePrivateKey;
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};
use std::fs;

pub fn generate_rsa_keys() -> Result<(RsaPrivateKey, RsaPublicKey), anyhow::Error> {
    let private_key_str = fs::read_to_string("./config/jwt/private.key")?;
    let public_key_str = fs::read_to_string("./config/jwt/public.key")?;

    if private_key_str.len() > 100 && public_key_str.len() > 50 {
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_str.as_str())?;
        let public_key = RsaPublicKey::from(&private_key);
        return Ok((private_key, public_key));
    };
    let (private_key, public_key) = new_rsa_key()?;
    Ok((private_key, public_key))
}

fn new_rsa_key() -> Result<(RsaPrivateKey, RsaPublicKey), rsa::errors::Error> {
    // 如果没有现有的密钥文件，则生成新的 RSA 密钥对
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");

    // 从私钥派生出公钥
    let public_key = RsaPublicKey::from(&private_key);
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
