use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::{Bytes, Uuid};
use rbs::value;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ChatMessageRecord {
    pub id: Option<i64>,
    pub session_uuid: Uuid,
    pub nano_id: Option<String>,
    pub timestamp: Option<i64>,
    pub raw: Bytes,
    pub text_type: Option<u32>,
    pub send_user: Uuid,
    pub recv_user: Uuid,
}

crud!(ChatMessageRecord {});

impl ChatMessageRecord {
    /// 按会话分页翻历史(任务08): 分区剪枝 + 索引扫描, 取代双向 OR 全分区扫描。
    #[rbatis::py_sql(
        "select * from chat_message_record
         where session_uuid = #{session_uuid}
         order by id limit #{size} offset #{start}"
    )]
    async fn select_by_session_paged_inner(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        start: u32,
        size: u32,
    ) -> Vec<ChatMessageRecord> {
    }

    pub async fn select_by_session_paged(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        start: u32,
        size: u32,
    ) -> rbatis::Result<Vec<ChatMessageRecord>> {
        Self::select_by_session_paged_inner(rb, session_uuid, start, size).await
    }

    /// 会话当前最大消息 id(桥接"读到底"用; 单分区索引扫描, 便宜)。
    pub async fn max_id_by_session(rb: &dyn Executor, session_uuid: &Uuid) -> rbatis::Result<i64> {
        let sql =
            "select COALESCE(max(id), 0) AS v from chat_message_record where session_uuid = $1";
        let result = rb.query(sql, vec![value!(session_uuid)]).await?;
        Ok(crate::models::scalar_i64(&result))
    }

    /// 会话最新一条消息(聚合用; 单分区索引反向取首行, 便宜)。
    ///
    /// 取代旧 `latest_per_session_for_user` 的跨 16 分区 `distinct on` 全扫:
    /// 会话的发现职责已由任务 04b 建行钩子承接, 聚合按 `user_session` 行逐会话点查。
    #[rbatis::py_sql(
        "select * from chat_message_record
         where session_uuid = #{session_uuid}
         order by id desc limit 1"
    )]
    async fn latest_by_session_inner(
        rb: &dyn Executor,
        session_uuid: &Uuid,
    ) -> Vec<ChatMessageRecord> {
    }

    pub async fn latest_by_session(
        rb: &dyn Executor,
        session_uuid: &Uuid,
    ) -> rbatis::Result<Option<ChatMessageRecord>> {
        Ok(Self::latest_by_session_inner(rb, session_uuid).await?.into_iter().next())
    }

    /// 窗口内正向翻页(任务12 正向追平): `id > after` 且窗口内的 limit 条, 返回 id **升序**。
    ///
    /// 命中任务11 PK `(session_uuid, id) INCLUDE (recv_user, "timestamp")` → Index Only Scan。
    /// `after` 缺省传 0(从窗口内最早起); `has_more` 由调用方用 limit+1 探测。
    #[rbatis::py_sql(
        "select * from chat_message_record
         where session_uuid = #{session_uuid} and id > #{after} and \"timestamp\" > #{boundary}
         order by id asc limit #{size}"
    )]
    async fn select_window_after_inner(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        after: i64,
        boundary: i64,
        size: u32,
    ) -> Vec<ChatMessageRecord> {
    }

    pub async fn select_window_after(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        after: i64,
        boundary: i64,
        size: u32,
    ) -> rbatis::Result<Vec<ChatMessageRecord>> {
        Self::select_window_after_inner(rb, session_uuid, after, boundary, size).await
    }
}
