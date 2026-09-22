use rbatis::crud;
use rbatis::executor::Executor;
use rbatis::rbdc::{Bytes, Uuid};
use rbs::value;
use serde::{Deserialize, Serialize};

/// 群消息类型
pub const MSG_TYPE_TEXT: i16 = 1;
pub const MSG_TYPE_IMAGE: i16 = 2;
pub const MSG_TYPE_FILE: i16 = 3;

/// 群消息记录（读扩散，只存 1 份）
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct GroupMessageRecord {
    /// 主键 ID
    pub id: Option<i64>,
    /// 消息唯一标识 (nanoid)
    pub nano_id: Option<String>,
    /// 群 UUID
    pub group_uuid: Option<Uuid>,
    /// 发送者 UUID
    pub send_user: Option<Uuid>,
    /// 时间戳 (Unix 时间戳，单位：毫秒)
    pub timestamp: Option<i64>,
    /// 原始消息内容
    pub raw: Bytes,
    /// 消息类型 (1: 文本, 2: 图片, 3: 文件)
    pub msg_type: Option<i16>,
    /// 是否撤回
    pub recalled: Option<bool>,
}

crud!(GroupMessageRecord {});

impl GroupMessageRecord {
    #[rbatis::py_sql("select * from group_message_record where nano_id = #{nano_id} limit 1")]
    async fn select_by_nano_id_inner(rb: &dyn Executor, nano_id: &str) -> Vec<GroupMessageRecord> {}

    pub async fn select_by_nano_id(
        rb: &dyn Executor,
        nano_id: &str,
    ) -> rbatis::Result<Option<GroupMessageRecord>> {
        Ok(Self::select_by_nano_id_inner(rb, nano_id).await?.into_iter().next())
    }

    /// 群聊翻历史(任务11: 排序统一到 `id`, 与同步/未读一致, 由 PK 支持, 不再依赖 timestamp 索引)。
    #[rbatis::py_sql(
        "select * from group_message_record where group_uuid = #{group_uuid} order by id desc limit #{size} offset #{start}"
    )]
    async fn select_by_group(
        rb: &dyn Executor,
        group_uuid: &Uuid,
        start: u32,
        size: u32,
    ) -> Vec<GroupMessageRecord> {
    }

    #[rbatis::py_sql(
        "select * from group_message_record where group_uuid = #{group_uuid} and id > #{cursor} order by id asc limit #{size}"
    )]
    async fn select_unread(
        rb: &dyn Executor,
        group_uuid: &Uuid,
        cursor: i64,
        size: u32,
    ) -> Vec<GroupMessageRecord> {
    }

    /// 群当前最大消息 id(桥接"读到底"用; 单分区索引扫描, 便宜)。
    pub async fn max_id_by_group(rb: &dyn Executor, group_uuid: &Uuid) -> rbatis::Result<i64> {
        let sql =
            "select COALESCE(max(id), 0) AS v from group_message_record where group_uuid = $1";
        let result = rb.query(sql, vec![value!(group_uuid)]).await?;
        Ok(crate::models::scalar_i64(&result))
    }

    /// 窗口内正向翻页(任务12 正向追平): `id > after` 且窗口内的 limit 条, 返回 id **升序**。
    ///
    /// 命中任务11 PK `(group_uuid, id) INCLUDE ("timestamp", send_user)` → Index Only Scan。
    /// `after` 缺省传 0(从窗口内最早起); `has_more` 由调用方用 limit+1 探测。
    #[rbatis::py_sql(
        "select * from group_message_record
         where group_uuid = #{group_uuid} and id > #{after} and \"timestamp\" > #{boundary}
         order by id asc limit #{size}"
    )]
    async fn select_window_after_inner(
        rb: &dyn Executor,
        group_uuid: &Uuid,
        after: i64,
        boundary: i64,
        size: u32,
    ) -> Vec<GroupMessageRecord> {
    }

    pub async fn select_window_after(
        rb: &dyn Executor,
        group_uuid: &Uuid,
        after: i64,
        boundary: i64,
        size: u32,
    ) -> rbatis::Result<Vec<GroupMessageRecord>> {
        Self::select_window_after_inner(rb, group_uuid, after, boundary, size).await
    }

    /// 指定群的最新一条消息。
    #[rbatis::py_sql(
        "select * from group_message_record where group_uuid = #{group_uuid} order by id desc limit 1"
    )]
    async fn select_latest_by_group_inner(
        rb: &dyn Executor,
        group_uuid: &Uuid,
    ) -> Vec<GroupMessageRecord> {
    }

    pub async fn select_latest_by_group(
        rb: &dyn Executor,
        group_uuid: &Uuid,
    ) -> rbatis::Result<Option<GroupMessageRecord>> {
        Ok(Self::select_latest_by_group_inner(rb, group_uuid).await?.into_iter().next())
    }
}
