use std::fs;
use anyhow::anyhow;
use rand::Rng;
use rand::distributions::Alphanumeric;
use rbatis::rbatis_codegen::ops::AsProxy;
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs1::EncodeRsaPublicKey;
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::PasswordHash;
use crate::config_manager::{get_config, set_config};

pub fn generate_rsa_keys() -> Result<(RsaPrivateKey, RsaPublicKey), anyhow::Error> {
    let private_key_config = get_config("jwt_private_key");
    let public_key_config = get_config("jwt_public_key");

    let (private_key_str, public_key_str) = if private_key_config.is_some() && public_key_config.is_some() {
        let private_key_str = private_key_config.ok_or(anyhow!("jwt_private_key config not found"))?;
        let public_key_str = public_key_config.ok_or(anyhow!("jwt_public_key config not found"))?;
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
    // Generate a new RSA key pair if no existing key files
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits)?;

    // Derive public key from private key
    let public_key = RsaPublicKey::from(&private_key);
    let private_key_pem = private_key.to_pkcs8_pem(Default::default())?;
    let private_key_str = private_key_pem.to_string();
    // Convert public key to PEM format string
    let public_key_pem = public_key.to_pkcs1_pem(Default::default())?;
    let public_key_str = public_key_pem.to_string();
    set_config("jwt_private_key".string(), private_key_str.clone());
    set_config("jwt_public_key".string(), public_key_str.clone());
    // Save generated key to file
    fs::write("./config/jwt/private.key", private_key_str)?;
    fs::write("./config/jwt/public.key", public_key_str)?;
    Ok((private_key, public_key))
}

// Generate a random string of specified length
pub fn generate_random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    std::iter::repeat(())
        .map(|_| rng.sample(Alphanumeric))
        .map(|num| num as char) // Convert u8 to char
        .take(length)
        .collect::<String>()
}

pub fn hash_password(password: &str) -> Result<String, anyhow::Error> {
    let salt = argon2::password_hash::SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("{}", e))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
}
