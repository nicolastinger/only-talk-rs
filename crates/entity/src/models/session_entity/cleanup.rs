//! 窗口外消息清理(任务09 §3): 分批删除 7 天前的单聊/群聊消息。

use rbatis::executor::Executor;
use rbs::value;

/// 分批删除窗口外消息, 返回本批删除行数 `(单聊, 群聊)`(0 = 本轮清完)。
///
/// ⚠️ 定位必须用 `id` —— `ctid` 是分区内局部值, 跨分区会误删(主方案 §15.6 陷阱)。
/// 幂等: 重复执行结果一致, 单批失败由调用方下轮重试。
pub async fn delete_messages_before(
    rb: &dyn Executor,
    boundary: i64,
    batch: u32,
) -> Result<(u64, u64), rbatis::Error> {
    let s = rb
        .exec(
            "DELETE FROM chat_message_record WHERE id IN (
                 SELECT id FROM chat_message_record WHERE \"timestamp\" < $1 LIMIT $2)",
            vec![value!(boundary), value!(batch)],
        )
        .await?;
    let g = rb
        .exec(
            "DELETE FROM group_message_record WHERE id IN (
                 SELECT id FROM group_message_record WHERE \"timestamp\" < $1 LIMIT $2)",
            vec![value!(boundary), value!(batch)],
        )
        .await?;
    Ok((s.rows_affected, g.rows_affected))
}
