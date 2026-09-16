use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::{Bytes, Uuid};
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

    // 获取未读消息，最大9999
    #[rbatis::py_sql(
        "select * from chat_message_record where (send_user = #{uuid} or recv_user = #{uuid}) and timestamp > #{time} order by timestamp desc limit 9999"
    )]
    async fn select_unread_by_time(
        rb: &dyn Executor,
        uuid: &Uuid,
        time: i64,
    ) -> Vec<ChatMessageRecord> {
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
