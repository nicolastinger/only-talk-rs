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
    #[rbatis::py_sql(
        "select * from chat_message_record where ((send_user = #{send_user} and recv_user = #{recv_user}) or (send_user = #{recv_user} and recv_user = #{send_user})) order by id limit #{size} offset #{start}"
    )]
    async fn select_chat_by_limit(
        rb: &dyn Executor,
        send_user: Uuid,
        recv_user: Uuid,
        start: u32,
        size: u32,
    ) -> Vec<ChatMessageRecord> {
    }

    // 获取最新一条消息
    #[rbatis::py_sql(
        "select * from chat_message_record where recv_user = #{uuid} or send_user = #{uuid} order by timestamp desc limit 1"
    )]
    async fn select_last_by_column_inner(rb: &dyn Executor, uuid: &Uuid) -> Vec<ChatMessageRecord> {
    }

    pub async fn select_last_by_column(
        rb: &dyn Executor,
        uuid: &Uuid,
    ) -> rbatis::Result<Option<ChatMessageRecord>> {
        Ok(Self::select_last_by_column_inner(rb, uuid).await?.into_iter().next())
    }

    /// 按会话游标拉取"我收到的"未读消息(缺陷A修复的核心查询: 游标与排序同列 id)。
    #[rbatis::py_sql(
        "select * from chat_message_record
         where session_uuid = #{session_uuid} and id > #{cursor} and recv_user = #{me}
         order by id asc limit #{size}"
    )]
    async fn select_unread_by_cursor(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        me: &Uuid,
        cursor: i64,
        size: u32,
    ) -> Vec<ChatMessageRecord> {
    }

    /// 会话当前最大消息 id(桥接"读到底"用; 单分区索引扫描, 便宜)。
    pub async fn max_id_by_session(rb: &dyn Executor, session_uuid: &Uuid) -> rbatis::Result<i64> {
        let sql =
            "select COALESCE(max(id), 0) AS v from chat_message_record where session_uuid = $1";
        let result = rb.query(sql, vec![value!(session_uuid)]).await?;
        Ok(crate::models::scalar_i64(&result))
    }

    /// 同步拉取(任务05): 游标 + 7 天窗口 + 升序。
    ///
    /// 命中 `idx_chat_msg_pull (session_uuid, id) INCLUDE ("timestamp")` —— 窗口过滤免回表。
    #[rbatis::py_sql(
        "select * from chat_message_record
         where session_uuid = #{session_uuid} and id > #{cursor} and \"timestamp\" > #{boundary}
         order by id asc limit #{size}"
    )]
    async fn select_sync_inner(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        cursor: i64,
        boundary: i64,
        size: u32,
    ) -> Vec<ChatMessageRecord> {
    }

    pub async fn select_sync(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        cursor: i64,
        boundary: i64,
        size: u32,
    ) -> rbatis::Result<Vec<ChatMessageRecord>> {
        Self::select_sync_inner(rb, session_uuid, cursor, boundary, size).await
    }

    /// initial 模式(任务05): 会话窗口内最新 N 条, 取回后反转为升序。
    #[rbatis::py_sql(
        "select * from chat_message_record
         where session_uuid = #{session_uuid} and \"timestamp\" > #{boundary}
         order by id desc limit #{size}"
    )]
    async fn select_latest_by_session_inner(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        boundary: i64,
        size: u32,
    ) -> Vec<ChatMessageRecord> {
    }

    pub async fn select_latest_by_session(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        boundary: i64,
        size: u32,
    ) -> rbatis::Result<Vec<ChatMessageRecord>> {
        let mut v = Self::select_latest_by_session_inner(rb, session_uuid, boundary, size).await?;
        v.reverse();
        Ok(v)
    }

    /// 窗口截断探测(任务05 §8.2): 游标之后是否存在窗口外的消息。
    ///
    /// 标量查询不走 py_sql(任务04 经验: py_sql 对非结构体标量解码不可靠), 用 `rb.query` 判空。
    pub async fn exists_beyond_window(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        cursor: i64,
        boundary: i64,
    ) -> rbatis::Result<bool> {
        let sql = "select id from chat_message_record
                   where session_uuid = $1 and id > $2 and \"timestamp\" <= $3 limit 1";
        let result =
            rb.query(sql, vec![value!(session_uuid), value!(cursor), value!(boundary)]).await?;
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
