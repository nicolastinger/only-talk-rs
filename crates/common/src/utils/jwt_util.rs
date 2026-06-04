use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::EncodePrivateKey;
use serde::{Deserialize, Serialize};
use crate::utils::rsa_util::get_rsa_keys;
use crate::utils::time::get_now_time_stamp_as_secs;

// Define JWT Claims struct
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,     // Extended info
    pub uuid: String, // User unique ID
    pub exp: i64,     // Expiry time (Unix timestamp)
}

fn load_jwt_keys() -> Result<(EncodingKey, DecodingKey), anyhow::Error> {
    let (private_key, public_key) = get_rsa_keys()?;
    let private_key_pem = private_key.to_pkcs8_pem(Default::default())?;
    let private_key_str = private_key_pem.to_string();
    // Convert public key to PEM format string
    let public_key_pem = public_key.to_pkcs1_pem(Default::default())?;
    let public_key_str = public_key_pem.to_string();

    // Create EncodingKey and DecodingKey
    let encoding_key = EncodingKey::from_rsa_pem(private_key_str.as_ref())?;
    let decoding_key = DecodingKey::from_rsa_pem(public_key_str.as_ref())?;

    Ok((encoding_key, decoding_key))
}

pub fn generate_access_token(uuid: String, platform: String) -> Result<String, anyhow::Error> {
    let (encoding_key, _) = load_jwt_keys()?;
    let claims =
        Claims { sub: platform, uuid, exp: get_now_time_stamp_as_secs()? + (3600 * 24) };
    let header = Header::new(jsonwebtoken::Algorithm::RS256);
    let token = encode(&header, &claims, &encoding_key)?;
    Ok(token)
}

pub fn generate_token_with_expiry(uuid: String, platform: String, expiry_secs: i64) -> Result<String, anyhow::Error> {
    let (encoding_key, _) = load_jwt_keys()?;
    let claims = Claims { sub: platform, uuid, exp: get_now_time_stamp_as_secs()? + expiry_secs };
    let header = Header::new(jsonwebtoken::Algorithm::RS256);
    let token = encode(&header, &claims, &encoding_key)?;
    Ok(token)
}

pub fn verify_token(token: &str) -> Result<Claims, anyhow::Error> {
    let (_, decoding_key) = load_jwt_keys()?;
    let validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    let decoded = decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(decoded.claims)
}
