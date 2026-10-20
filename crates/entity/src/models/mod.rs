pub mod announcement_entity;
pub mod chat_entity;
pub mod file_entity;
pub mod group_entity;
pub mod moment_entity;
pub mod notify_entity;
pub mod plaza_entity;
pub mod report_entity;
pub mod session_entity;
pub mod user_entity;

/// 从单行单列查询结果(列别名 `AS v`)提取 i64, 缺失/非法返回 0。
///
/// 供 `max(id)` 这类标量查询使用(rbatis 的 py_sql 对非结构体标量解码不可靠)。
pub(crate) fn scalar_i64(result: &rbs::Value) -> i64 {
    result
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.as_map())
        .map(|map| map.get(&rbs::Value::from("v")))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}
