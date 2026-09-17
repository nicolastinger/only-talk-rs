//! 窗口外消息清理(任务09 §3 / 任务11 §4.1): 分批删除 7 天前的单聊/群聊消息。

use rbatis::executor::Executor;
use rbs::value;

/// 分批删除窗口外消息, 返回本轮删除总行数(0 = 本轮清完)。
///
/// 任务11: PK 不再以 `id` 打头(`(session_uuid, id)`), 外层 `WHERE id IN (...)` 无法用 PK,
/// 改为**逐分区 `ctid` 批删**。
///
/// ⚠️ 必须**逐分区**执行: 子查询与删除目标同一分区, `ctid` 才安全。
/// 跨分区 `ctid` 是分区内局部值, 会误删(主方案 §15.6 陷阱) —— 不可合并为单条语句。
///
/// 清理是每天一次的后台任务, `timestamp` 无索引的分区内 seq scan + LIMIT 命中即止可接受;
/// 写路径扛不起多余索引, 这是本任务的取舍落点(任务11 §4.1)。
pub async fn delete_messages_before(
    rb: &dyn Executor,
    boundary: i64,
    batch: u32,
) -> Result<u64, rbatis::Error> {
    let mut total = 0u64;
    // 分区名由 DDL 固定(后缀 p00..p15), 直接枚举, 无需查 pg_inherits
    for table in ["chat_message_record", "group_message_record"] {
        for i in 0..16 {
            let partition = format!("{table}_p{i:02}");
            let sql = format!(
                "DELETE FROM {partition} WHERE ctid IN (
                     SELECT ctid FROM {partition}
                     WHERE \"timestamp\" < $1 LIMIT $2)"
            );
            let res = rb.exec(&sql, vec![value!(boundary), value!(batch)]).await?;
            total += res.rows_affected;
        }
    }
    Ok(total)
}
