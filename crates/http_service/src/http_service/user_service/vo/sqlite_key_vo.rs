use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchSqliteKeyResponseVO {
    /// 本地 SQLCipher 数据库密钥（32字节的 hex，64位小写十六进制）
    pub db_key: String,
    /// 密钥版本
    pub key_version: i32,
    /// 是否为该用户首次签发的全新密钥（false 表示本设备沿用用户已有的密钥，
    /// 或该设备此前已签发过；true 仅当该用户第一次获取密钥时出现）
    pub provisioned: bool,
}
