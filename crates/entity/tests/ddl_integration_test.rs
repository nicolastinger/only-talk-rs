//! 集成测试：将 `entity/ddl` 下全部建表脚本写入独立的测试数据库。
//!
//! 复用仓库根目录 `.env` 中的 `DATABASE_URL`（管理员连接）作为连接信息，
//! 自动创建测试库（默认 `only_talk_test`，可用 `TEST_DATABASE_NAME` 覆盖）
//! 并在其中执行全部 DDL，不会触碰 `.env` 指向的开发库。
//! 测试结束后由 [`common::drop_test_database`] 删除测试库（见函数注释）。
//!
//! 运行方式：
//!   cargo test -p entity --test ddl_integration_test -- --ignored
//! 前提：本地 PostgreSQL 可用，且仓库根目录存在 `.env`。
//!
//! 建库/连接/删库等共享设施见 `tests/common/mod.rs`。

use anyhow::{Context, Result, anyhow};
use tracing::info;

mod common;

use common::{
    admin_database_url, build_pool, drop_test_database, init_tracing, recreate_database,
    should_drop_test_database, test_database_name, test_database_url,
};

/// 关键表清单：DDL 应用后逐一校验存在性
const TABLES: &[&str] = &[
    "basic_user",
    "user_info",
    "friend_link",
    "friend_list",
    "black_list",
    "chat_message_record",
    "file_upload_record",
    "group_info",
    "group_member",
    "system_notification",
    "user_login_log",
    "robot_info",
    "enterprise_info",
    "session",
    "user_session",
];

/// 哈希分区表与其期望的分区数（任务02：两张消息表各 16 个分区）
const PARTITIONED_TABLES: &[(&str, i64)] =
    &[("chat_message_record", 16), ("group_message_record", 16)];

#[tokio::test]
#[ignore = "需要本地 PostgreSQL 与仓库根目录 .env"]
async fn apply_all_ddl_to_test_database() -> Result<()> {
    init_tracing();

    let admin_url = admin_database_url()?;
    info!("已读取测试数据库管理员连接（库名将替换为测试库）");
    let test_db_name = test_database_name();
    info!("测试库名: {}", test_db_name);

    let admin = build_pool(&admin_url).await?;
    info!("已连接管理员数据库");
    recreate_database(&admin, &test_db_name).await?;
    info!("测试库 {} 已重建为空库", test_db_name);

    // 执行 DDL 并校验关键表，结果暂存，最后统一交给删除清理
    let result = async {
        let test_url = test_database_url(&admin_url, &test_db_name)?;
        let test_rb = build_pool(&test_url).await?;
        info!("已连接测试库 {}", test_db_name);

        entity::ddl::apply_all_ddl(&test_rb).await.context("应用 DDL 失败")?;
        info!("DDL 应用完成，开始校验关键表");

        for table in TABLES {
            if !table_exists(&test_rb, table).await? {
                return Err(anyhow!("表 {} 未创建", table));
            }
            info!("表 {} 校验通过", table);
        }
        info!("全部 {} 张关键表校验通过", TABLES.len());

        for (table, expected) in PARTITIONED_TABLES {
            let actual = partition_count(&test_rb, table).await?;
            if actual != *expected {
                return Err(anyhow!("分区表 {} 期望 {} 个分区, 实际 {}", table, expected, actual));
            }
            info!("分区表 {} 校验通过: {} 个分区", table, actual);
        }

        // 任务09 §2.3: 分区表唯一约束必须含分区键(session_uuid / group_uuid)
        for (table, key_col) in
            [("chat_message_record", "session_uuid"), ("group_message_record", "group_uuid")]
        {
            if !unique_constraint_has_column(&test_rb, table, key_col).await? {
                return Err(anyhow!("{} 的唯一约束应含分区键 {}", table, key_col));
            }
            info!("{} 唯一约束含分区键 {} 校验通过", table, key_col);
        }

        // 任务11 §7: PK 必须含分区键, 且带 INCLUDE 列(免回表)
        for (table, key_col, include_col) in [
            ("chat_message_record", "session_uuid", "timestamp"),
            ("group_message_record", "group_uuid", "timestamp"),
        ] {
            let def = primary_key_def(&test_rb, table)
                .await?
                .ok_or_else(|| anyhow!("{} 无主键约束", table))?;
            if !def.contains(key_col) {
                return Err(anyhow!("{} PK 应含分区键 {}: {}", table, key_col, def));
            }
            if !def.contains("INCLUDE") || !def.contains(include_col) {
                return Err(anyhow!("{} PK 应带 INCLUDE {}: {}", table, include_col, def));
            }
            info!("{} PK 校验通过: {}", table, def);
        }
        Ok(())
    }
    .await;

    // 是否执行删库清理由环境变量 DROP_TEST_DATABASE 控制（1/true 才删除）
    if should_drop_test_database() {
        // 无论测试成功还是失败，最后都执行删库清理，保证测试可重复执行
        drop_test_database(&admin, &test_db_name).await?;
    } else {
        info!(
            "已保留测试库 {}（未设置 DROP_TEST_DATABASE=true，不执行删库，可直接在 pgAdmin/psql 查看）",
            test_db_name
        );
    }

    result
}

/// 查询表是否存在
async fn table_exists(rb: &rbatis::RBatis, table: &str) -> Result<bool> {
    let sql = format!(
        "SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '{}'",
        table
    );
    let result: rbs::Value =
        rb.query(&sql, vec![]).await.map_err(|e| anyhow!("查询表信息失败: {}", e))?;
    Ok(result.as_array().map_or(0, |rows| rows.len()) > 0)
}

/// 查询父表的分区数（pg_inherits 中该父表的分区子表条数）
async fn partition_count(rb: &rbatis::RBatis, table: &str) -> Result<i64> {
    let sql = format!("SELECT count(*) FROM pg_inherits WHERE inhparent = '{}'::regclass", table);
    let result: rbs::Value =
        rb.query(&sql, vec![]).await.map_err(|e| anyhow!("查询分区数失败: {}", e))?;
    let rows = result.as_array().cloned().unwrap_or_default();
    let count = rows
        .first()
        .and_then(|row| row.as_map())
        .map(|map| map.get(&rbs::Value::from("count")))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    Ok(count)
}

/// 该表是否存在含指定列的唯一约束(任务09 §2.3: 分区表唯一约束必须含分区键)
async fn unique_constraint_has_column(
    rb: &rbatis::RBatis,
    table: &str,
    column: &str,
) -> Result<bool> {
    let defs = constraint_defs(rb, table, "u").await?;
    Ok(defs.iter().any(|d| d.contains(column)))
}

/// 该表主键约束定义(任务11 §7: PK 含分区键 + INCLUDE)
async fn primary_key_def(rb: &rbatis::RBatis, table: &str) -> Result<Option<String>> {
    Ok(constraint_defs(rb, table, "p").await?.into_iter().next())
}

/// 查询某表指定类型(contype: p=主键 / u=唯一)的约束定义
async fn constraint_defs(rb: &rbatis::RBatis, table: &str, contype: &str) -> Result<Vec<String>> {
    let sql = format!(
        "SELECT pg_get_constraintdef(c.oid) AS def FROM pg_constraint c \
         JOIN pg_class t ON t.oid = c.conrelid \
         WHERE t.relname = '{}' AND c.contype = '{}'",
        table, contype
    );
    let result: rbs::Value =
        rb.query(&sql, vec![]).await.map_err(|e| anyhow!("查询约束定义失败: {}", e))?;
    Ok(result
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    row.as_map()
                        .map(|m| m.get(&rbs::Value::from("def")))
                        .and_then(|v| v.as_str().map(str::to_string))
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default())
}
