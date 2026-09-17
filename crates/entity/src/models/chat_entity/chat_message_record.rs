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

    /// 窗口内向旧翻页(任务12): `id < before` 且窗口内的最新 limit 条, 返回 id **降序**。
    ///
    /// 调用方取前 limit 条后自行反转为升序。命中任务11 PK
    /// `(session_uuid, id) INCLUDE (recv_user, "timestamp")` → Index Only Scan。
    /// `before` 缺省传 `i64::MAX`(等价无上界)。
    #[rbatis::py_sql(
        "select * from chat_message_record
         where session_uuid = #{session_uuid} and id < #{before} and \"timestamp\" > #{boundary}
         order by id desc limit #{size}"
    )]
    async fn select_window_before_inner(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        before: i64,
        boundary: i64,
        size: u32,
    ) -> Vec<ChatMessageRecord> {
    }

    pub async fn select_window_before(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        before: i64,
        boundary: i64,
        size: u32,
    ) -> rbatis::Result<Vec<ChatMessageRecord>> {
        Self::select_window_before_inner(rb, session_uuid, before, boundary, size).await
    }

    /// 是否存在比 `id` 更旧的消息(任务12 截断探测): 纯 id 存在性, 不带窗口条件。
    ///
    /// 调用方仅在窗口内取尽(`has_more=false`)时调用 → 任何更旧行必然已出窗口。
    ///
    /// 标量查询不走 py_sql(任务04 经验: py_sql 对非结构体标量解码不可靠), 用 `rb.query` 判空。
    pub async fn exists_older_than(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        id: i64,
    ) -> rbatis::Result<bool> {
        let sql = "select id from chat_message_record
                   where session_uuid = $1 and id < $2 limit 1";
        let result = rb.query(sql, vec![value!(session_uuid), value!(id)]).await?;
        Ok(result.as_array().map(|rows| !rows.is_empty()).unwrap_or(false))
    }

    /// 每个相关会话的最新一条消息(distinct on 按会话取 id 最大行)。
    ///
    /// ⚠️ 不含 `session_uuid` 等值条件, 分区表上跨全分区扫描 —— 开发期可接受,
    /// 数据量上来后由任务 09 评估(见任务书 §8 性能注记)。
    #[rbatis::py_sql(
        "select distinct on (session_uuid) * from chat_message_record
         where recv_user = #{me} or send_user = #{me}
         order by session_uuid, id desc"
    )]
    async fn select_latest_per_session_for_user(
        rb: &dyn Executor,
        me: &Uuid,
    ) -> Vec<ChatMessageRecord> {
    }

    pub async fn latest_per_session_for_user(
        rb: &dyn Executor,
        me: &Uuid,
    ) -> rbatis::Result<Vec<ChatMessageRecord>> {
        Self::select_latest_per_session_for_user(rb, me).await
    }
}
