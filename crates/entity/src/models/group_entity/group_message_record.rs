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

    #[rbatis::py_sql(
        "select * from group_message_record where group_uuid = #{group_uuid} order by timestamp desc limit #{size} offset #{start}"
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

    /// 同步拉取(任务05): 游标 + 7 天窗口 + 升序。
    ///
    /// 命中 `idx_group_msg_pull (group_uuid, id) INCLUDE ("timestamp")`。
    #[rbatis::py_sql(
        "select * from group_message_record
         where group_uuid = #{group_uuid} and id > #{cursor} and \"timestamp\" > #{boundary}
         order by id asc limit #{size}"
    )]
    async fn select_sync_inner(
        rb: &dyn Executor,
        group_uuid: &Uuid,
        cursor: i64,
        boundary: i64,
        size: u32,
    ) -> Vec<GroupMessageRecord> {
    }

    pub async fn select_sync(
        rb: &dyn Executor,
        group_uuid: &Uuid,
        cursor: i64,
        boundary: i64,
        size: u32,
    ) -> rbatis::Result<Vec<GroupMessageRecord>> {
        Self::select_sync_inner(rb, group_uuid, cursor, boundary, size).await
    }

    /// initial 模式(任务05): 群窗口内最新 N 条, 取回后反转为升序。
    ///
    /// 与既有 `select_latest_by_group`(单条, 无窗口) 区分, 故另起名。
    #[rbatis::py_sql(
        "select * from group_message_record
         where group_uuid = #{group_uuid} and \"timestamp\" > #{boundary}
         order by id desc limit #{size}"
    )]
    async fn select_latest_in_window_by_group_inner(
        rb: &dyn Executor,
        group_uuid: &Uuid,
        boundary: i64,
        size: u32,
    ) -> Vec<GroupMessageRecord> {
    }

    pub async fn select_latest_in_window_by_group(
        rb: &dyn Executor,
        group_uuid: &Uuid,
        boundary: i64,
        size: u32,
    ) -> rbatis::Result<Vec<GroupMessageRecord>> {
        let mut v =
            Self::select_latest_in_window_by_group_inner(rb, group_uuid, boundary, size).await?;
        v.reverse();
        Ok(v)
    }

    /// 窗口截断探测(任务05 §8.2): 游标之后是否存在窗口外的消息。
    ///
    /// 标量查询不走 py_sql(任务04 经验), 用 `rb.query` 判空。
    pub async fn exists_beyond_window(
        rb: &dyn Executor,
        group_uuid: &Uuid,
        cursor: i64,
        boundary: i64,
    ) -> rbatis::Result<bool> {
        let sql = "select id from group_message_record
                   where group_uuid = $1 and id > $2 and \"timestamp\" <= $3 limit 1";
        let result =
            rb.query(sql, vec![value!(group_uuid), value!(cursor), value!(boundary)]).await?;
        Ok(result.as_array().map(|rows| !rows.is_empty()).unwrap_or(false))
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
