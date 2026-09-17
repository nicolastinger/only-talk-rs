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
