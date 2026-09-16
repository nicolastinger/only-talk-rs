use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbs::value;
use serde::{Deserialize, Serialize};

/// 用户视角的会话状态: 每用户每会话一行。
///
/// 不变式: `last_read_id <= synced_id`(服务端在上报处钳制)。
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserSession {
    pub id: Option<i64>,
    /// 谁的列表
    pub user_uuid: Uuid,
    /// 关联 session
    pub session_uuid: Uuid,
    /// 冗余会话类型, 免 join
    pub session_type: Option<i16>,
    /// 单聊: 对方; 群聊: NULL
    pub peer_uuid: Option<Uuid>,
    /// 已读游标(角标口径), 只前进
    pub last_read_id: Option<i64>,
    /// 同步游标(拉取起点), 只前进, 恒 >= last_read_id
    pub synced_id: Option<i64>,
    /// 置顶: 0-否 1-是
    pub pinned: Option<i16>,
    /// 免打扰: 0-否 1-是
    pub muted: Option<i16>,
    /// 软删时刻(毫秒); session.last_message_at > 此值即视为复活
    pub deleted_at: Option<i64>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

crud!(UserSession {});

impl UserSession {
    /// 懒创建用户会话行(幂等): 已存在时不覆盖任何字段(游标只前进, 禁止回写覆盖)。
    ///
    /// `last_read_id` / `synced_id` 为 `None` 时按 0 落库。
    pub async fn upsert(rb: &dyn Executor, u: &UserSession) -> Result<(), rbatis::Error> {
        rb.exec(
            "INSERT INTO user_session
                 (user_uuid, session_uuid, session_type, peer_uuid,
                  last_read_id, synced_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4,
                     COALESCE($5, 0), COALESCE($6, 0),
                     (extract(epoch from clock_timestamp()) * 1000)::bigint,
                     (extract(epoch from clock_timestamp()) * 1000)::bigint)
             ON CONFLICT (user_uuid, session_uuid) DO NOTHING",
            vec![
                value!(&u.user_uuid),
                value!(&u.session_uuid),
                value!(&u.session_type),
                value!(&u.peer_uuid),
                value!(&u.last_read_id),
                value!(&u.synced_id),
            ],
        )
        .await?;
        Ok(())
    }

    /// 已读游标推进: 只前进 + 钳制到 `synced_id`(§7.1 / §4.3 不变式 `last_read_id <= synced_id`)。
    ///
    /// 返回受影响行数(0 = 无需推进或行不存在)。
    pub async fn update_last_read_id(
        rb: &dyn Executor,
        user_uuid: &Uuid,
        session_uuid: &Uuid,
        reported: i64,
    ) -> Result<u64, rbatis::Error> {
        let res = rb
            .exec(
                "UPDATE user_session
                 SET last_read_id = LEAST($1, synced_id),
                     updated_at   = (extract(epoch from clock_timestamp()) * 1000)::bigint
                 WHERE user_uuid = $2
                   AND session_uuid = $3
                   AND last_read_id < $1",
                vec![value!(reported), value!(user_uuid), value!(session_uuid)],
            )
            .await?;
        Ok(res.rows_affected)
    }

    /// 同步游标推进: 只前进(§7.3)。
    pub async fn update_synced_id(
        rb: &dyn Executor,
        user_uuid: &Uuid,
        session_uuid: &Uuid,
        new_id: i64,
    ) -> Result<u64, rbatis::Error> {
        let res = rb
            .exec(
                "UPDATE user_session
                 SET synced_id   = $1,
                     updated_at  = (extract(epoch from clock_timestamp()) * 1000)::bigint
                 WHERE user_uuid = $2
                   AND session_uuid = $3
                   AND synced_id < $1",
                vec![value!(new_id), value!(user_uuid), value!(session_uuid)],
            )
            .await?;
        Ok(res.rows_affected)
    }

    /// 软删会话: `deleted_at` 置当前时刻, 同时把 `last_read_id` 推到 session 最后一条消息(§9.2)。
    ///
    /// 推到底是必须的 —— 否则复活时未读数会把删除前的旧消息一起算上。
    pub async fn soft_delete(
        rb: &dyn Executor,
        user_uuid: &Uuid,
        session_uuid: &Uuid,
    ) -> Result<u64, rbatis::Error> {
        let res = rb
            .exec(
                "UPDATE user_session us
                 SET deleted_at  = (extract(epoch from clock_timestamp()) * 1000)::bigint,
                     last_read_id = GREATEST(us.last_read_id, s.last_message_id),
                     updated_at  = (extract(epoch from clock_timestamp()) * 1000)::bigint
                 FROM session s
                 WHERE us.user_uuid = $1
                   AND us.session_uuid = $2
                   AND s.session_uuid = us.session_uuid",
                vec![value!(user_uuid), value!(session_uuid)],
            )
            .await?;
        Ok(res.rows_affected)
    }

    /// 入群初始化游标(§7.4): `last_read_id` / `synced_id` 初始化为群当前最新消息 id,
    /// 避免新成员把全部群历史视为未读。幂等(`ON CONFLICT DO NOTHING`) —— 老成员不重置游标。
    pub async fn init_for_group_join(
        rb: &dyn Executor,
        user_uuid: &Uuid,
        group_uuid: &Uuid,
    ) -> Result<(), rbatis::Error> {
        rb.exec(
            "INSERT INTO user_session
                 (user_uuid, session_uuid, session_type, peer_uuid,
                  last_read_id, synced_id, created_at, updated_at)
             VALUES ($1, $2, 2, NULL,
                     (SELECT COALESCE(MAX(id), 0) FROM group_message_record WHERE group_uuid = $2),
                     (SELECT COALESCE(MAX(id), 0) FROM group_message_record WHERE group_uuid = $2),
                     (extract(epoch from clock_timestamp()) * 1000)::bigint,
                     (extract(epoch from clock_timestamp()) * 1000)::bigint)
             ON CONFLICT (user_uuid, session_uuid) DO NOTHING",
            vec![value!(user_uuid), value!(group_uuid)],
        )
        .await?;
        Ok(())
    }
}
