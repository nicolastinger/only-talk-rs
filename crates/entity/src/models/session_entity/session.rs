use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbs::value;
use serde::{Deserialize, Serialize};

/// 会话类型常量与 common::config_str 保持一致(避免 entity 反向依赖 common)。
/// 注意依赖方向: common → entity, 因此常量定义在 entity, common 侧 re-export 或直接引用。
pub const SESSION_TYPE_SINGLE: i16 = 1;
pub const SESSION_TYPE_GROUP: i16 = 2;
pub const SESSION_TYPE_SYSTEM: i16 = 3;
pub const SESSION_TYPE_OFFICIAL_ACCOUNT: i16 = 4;

/// 会话本体: 单聊=用户对, 群聊=群; 双方/全员共享一行。
///
/// `last_message_*` 由聚合维护(读路径惰性触发: `/session/list` 首页 / 控制操作), 非实时更新。
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Session {
    pub session_uuid: Uuid,
    /// 1-单聊 2-群聊 3-系统 4-公众号
    pub session_type: Option<i16>,
    /// 最后一条消息 id(消息表 id, 只前进不回退)
    pub last_message_id: Option<i64>,
    /// 最后一条消息时间(毫秒)
    pub last_message_at: Option<i64>,
    /// 列表摘要(已截断至 256 字符)
    pub last_preview: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

crud!(Session {});

/// 会话列表行(entity 层形态, http_service 转 VO)。
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct SessionListRow {
    pub session_uuid: Uuid,
    pub session_type: Option<i16>,
    pub peer_uuid: Option<Uuid>,
    pub last_message_id: Option<i64>,
    pub last_message_at: Option<i64>,
    pub last_preview: Option<String>,
    pub pinned: Option<i16>,
    pub muted: Option<i16>,
    pub unread: Option<i64>,
}

impl Session {
    /// 懒创建会话行(幂等): 已存在时不覆盖任何字段。
    ///
    /// 用于聚合任务与控制信息变更前确保行存在。`created_at` / `updated_at`
    /// 取 DB 端时钟(`clock_timestamp()`), 避免多节点时钟差异(决策 A)。
    pub async fn upsert(rb: &dyn Executor, s: &Session) -> Result<(), rbatis::Error> {
        rb.exec(
            "INSERT INTO session (session_uuid, session_type, created_at, updated_at)
             VALUES ($1, $2,
                     (extract(epoch from clock_timestamp()) * 1000)::bigint,
                     (extract(epoch from clock_timestamp()) * 1000)::bigint)
             ON CONFLICT (session_uuid) DO NOTHING",
            vec![value!(&s.session_uuid), value!(&s.session_type)],
        )
        .await?;
        Ok(())
    }

    /// 单调更新最后一条消息: 仅当新 id 更大时生效(§6.2)。
    ///
    /// `last_preview` 可为 `None`: NULL 不覆盖已有值(见任务书 §3.4)。
    /// 返回受影响行数 —— 0 表示会话行不存在(调用方应先 `upsert`)或 id 未前进。
    pub async fn update_last_message(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        last_message_id: i64,
        last_message_at: i64,
        last_preview: Option<&str>,
    ) -> Result<u64, rbatis::Error> {
        let res = rb
            .exec(
                "UPDATE session
                 SET last_message_id = $1,
                     last_message_at = $2,
                     last_preview    = COALESCE($3, last_preview),
                     updated_at      = (extract(epoch from clock_timestamp()) * 1000)::bigint
                 WHERE session_uuid = $4
                   AND (last_message_id IS NULL OR last_message_id < $1)",
                vec![
                    value!(last_message_id),
                    value!(last_message_at),
                    value!(last_preview),
                    value!(session_uuid),
                ],
            )
            .await?;
        Ok(res.rows_affected)
    }

    /// 会话列表(任务06 §9.1): 未读现算 + 软删复活 + keyset 分页。
    ///
    /// `cursor` 为 `None`(首页)时跳过分页条件。
    /// ⚠️ 游标占位符显式转型(`::int2` / `::int8` / `::uuid`): 首页传 NULL 会把参数类型
    /// 定为 UNKNOWN, 后续页传具体值复用同连接语句缓存时会 `22P03`(任务05 缺陷 T 同源)。
    ///
    /// 已知局限(主方案 §4.3 注): 排序键在 join 的 session 表上, 索引只帮过滤不帮排序,
    /// 实际为该用户会话行的内存排序 —— 单用户几百会话可接受, 列入任务 09 实测项。
    #[rbatis::py_sql(
        "select s.session_uuid, s.session_type, s.peer_uuid,
                c.last_message_id, c.last_message_at, c.last_preview, s.pinned, s.muted,
                (case s.session_type
                   when 1 then (select count(*) from chat_message_record m
                                where m.session_uuid = s.session_uuid and m.id > s.last_read_id
                                  and m.recv_user = #{me})
                   else (select count(*) from group_message_record m
                         where m.group_uuid = s.session_uuid and m.id > s.last_read_id
                           and m.send_user <> #{me})
                 end) as unread
         from user_session s
         join session c on c.session_uuid = s.session_uuid
         where s.user_uuid = #{me}
           and (s.deleted_at is null or c.last_message_at > s.deleted_at)
           and (#{cur_pinned}::int2 is null
                or (s.pinned, coalesce(c.last_message_at, 0), s.session_uuid)
                   < (#{cur_pinned}::int2, #{cur_at}::int8, #{cur_uuid}::uuid))
         order by s.pinned desc, c.last_message_at desc nulls last, s.session_uuid desc
         limit #{size}"
    )]
    async fn select_list_inner(
        rb: &dyn Executor,
        me: &Uuid,
        cur_pinned: Option<i16>,
        cur_at: Option<i64>,
        cur_uuid: Option<Uuid>,
        size: u32,
    ) -> Vec<SessionListRow> {
    }

    pub async fn select_list(
        rb: &dyn Executor,
        me: &Uuid,
        cursor: Option<(i16, i64, Uuid)>,
        size: u32,
    ) -> rbatis::Result<Vec<SessionListRow>> {
        let (p, a, u) =
            cursor.map(|(p, a, u)| (Some(p), Some(a), Some(u))).unwrap_or((None, None, None));
        Self::select_list_inner(rb, me, p, a, u, size).await
    }
}
