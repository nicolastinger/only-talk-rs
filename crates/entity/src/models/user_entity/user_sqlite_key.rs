use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use serde::{Deserialize, Serialize};

/// 用户本地加密数据库密钥托管表。
///
/// 以 (user_id, device_fingerprint) 唯一确认。同一用户各设备/历史指纹记录**复用同一把
/// 底层密钥**(新设备沿用已签发密钥再加密落一行)，从而保证设备指纹漂移或换机后，
/// 客户端仍能取回可打开既有本地库的密钥。密钥经服务端主密钥 AES-256-GCM 加密后落库，
/// 服务端只在鉴权通过后向对应设备解密返回，客户端登录后据此创建/打开本地 private.db。
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserSqliteKey {
    /// 主键ID
    pub id: Option<i64>,
    /// 关联 basic_user.uuid
    pub user_id: Option<Uuid>,
    /// 设备指纹（客户端采集的 SHA-256 稳定指纹）
    pub device_fingerprint: Option<String>,
    /// 密钥版本（预留轮换/重签发）
    pub key_version: Option<i32>,
    /// AES-256-GCM 加密后的密钥（hex：nonce(12) + 密文含tag）
    pub encrypted_key: Option<String>,
    /// 创建时间（Unix毫秒）
    pub created_at: Option<i64>,
    /// 更新时间（Unix毫秒）
    pub updated_at: Option<i64>,
}

crud!(UserSqliteKey {});

impl UserSqliteKey {
    #[rbatis::py_sql(
        "select * from user_sqlite_key where user_id = #{uuid} and device_fingerprint = #{device_fingerprint} limit 1"
    )]
    async fn select_by_user_device_inner(
        rb: &dyn Executor,
        uuid: &Uuid,
        device_fingerprint: &str,
    ) -> Vec<UserSqliteKey> {
    }

    pub async fn select_by_user_device(
        rb: &dyn Executor,
        uuid: &Uuid,
        device_fingerprint: &str,
    ) -> rbatis::Result<Option<UserSqliteKey>> {
        Ok(Self::select_by_user_device_inner(rb, uuid, device_fingerprint)
            .await?
            .into_iter()
            .next())
    }

    #[rbatis::py_sql(
        "select * from user_sqlite_key where user_id = #{uuid} order by id asc limit 1"
    )]
    async fn select_one_by_user_inner(rb: &dyn Executor, uuid: &Uuid) -> Vec<UserSqliteKey> {}

    /// 取该用户最早签发的密钥记录(新设备/新指纹复用同一把 key 的依据)
    pub async fn select_one_by_user(
        rb: &dyn Executor,
        uuid: &Uuid,
    ) -> rbatis::Result<Option<UserSqliteKey>> {
        Ok(Self::select_one_by_user_inner(rb, uuid).await?.into_iter().next())
    }
}
