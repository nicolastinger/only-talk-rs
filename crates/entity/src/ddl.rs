//! 集成测试辅助：解析并执行 `entity/ddl` 目录下的全部 SQL 脚本。
//!
//! [`split_sql_statements`] 为纯函数，将一段多语句 SQL 拆分为独立语句；
//! [`apply_all_ddl`] / [`apply_sql_dir`] / [`apply_sql_file`] 按文件顺序建表。
//! 建表语句均使用 `IF NOT EXISTS`，可安全重复执行。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use rbatis::RBatis;
use tracing::info;

/// entity crate 内 ddl 目录的绝对路径（编译期固定，与调用方所在 crate 无关）
const DDL_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ddl");

/// 将 ddl 目录（含 `migrations` 子目录）下全部 SQL 文件按文件名顺序依次执行
pub async fn apply_all_ddl(rb: &RBatis) -> Result<()> {
    apply_sql_dir(rb, Path::new(DDL_DIR)).await
}

/// 递归执行目录（含子目录）下全部 `.sql` 文件，按路径字典序
pub async fn apply_sql_dir(rb: &RBatis, dir: &Path) -> Result<()> {
    let files = collect_sql_files(dir)?;
    info!("开始执行 DDL，目录: {}，共 {} 个文件", dir.display(), files.len());
    for file in &files {
        apply_sql_file(rb, file).await?;
    }
    Ok(())
}

/// 解析并执行单个 SQL 文件中的全部语句
pub async fn apply_sql_file(rb: &RBatis, path: &Path) -> Result<()> {
    let sql = fs::read_to_string(path)
        .with_context(|| format!("读取 SQL 文件失败: {}", path.display()))?;
    let statements = split_sql_statements(&sql);
    info!("执行 SQL 文件: {}（{} 条语句）", path.display(), statements.len());
    for (idx, stmt) in statements.iter().enumerate() {
        execute_statement(rb, stmt)
            .await
            .with_context(|| format!("文件 {} 第 {} 条语句执行失败", path.display(), idx + 1))?;
    }
    Ok(())
}

/// 执行单条语句：返回结果集的（SELECT/WITH）走 `query`，其余走 `exec`
async fn execute_statement(rb: &RBatis, stmt: &str) -> Result<()> {
    let first_word = stmt.split_whitespace().next().unwrap_or("");
    if first_word.eq_ignore_ascii_case("SELECT") || first_word.eq_ignore_ascii_case("WITH") {
        rb.query(stmt, vec![]).await.map(|_| ()).map_err(|e| anyhow!("执行查询失败: {}", e))?;
    } else {
        rb.exec(stmt, vec![]).await.map(|_| ()).map_err(|e| anyhow!("执行语句失败: {}", e))?;
    }
    Ok(())
}

/// 递归收集目录（含子目录）下所有 `.sql` 文件，返回排序后的路径列表
fn collect_sql_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = fs::read_dir(&current)
            .with_context(|| format!("读取目录失败: {}", current.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| format!("读取目录项失败: {}", current.display()))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "sql") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// 将一段多语句 SQL 拆分为独立语句列表。
///
/// 支持：`--` 行注释、单引号字符串（`''` 转义）、双引号标识符（`""` 转义）、
/// `$tag$...$tag$` 美元引用（如 `DO $$ ... $$`）。语句以顶层分号分割，
/// 忽略注释与空语句。
pub fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut chars = sql.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '-' if chars.peek() == Some(&'-') => {
                for ch in chars.by_ref() {
                    if ch == '\n' {
                        break;
                    }
                }
            }
            '\'' => {
                current.push('\'');
                while let Some(ch) = chars.next() {
                    current.push(ch);
                    if ch == '\'' {
                        if chars.peek() == Some(&'\'') {
                            current.push('\'');
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
            }
            '"' => {
                current.push('"');
                while let Some(ch) = chars.next() {
                    current.push(ch);
                    if ch == '"' {
                        if chars.peek() == Some(&'"') {
                            current.push('"');
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
            }
            '$' => {
                let mut tag = Vec::new();
                while let Some(&t) = chars.peek() {
                    if t.is_ascii_alphanumeric() || t == '_' {
                        tag.push(t);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if chars.peek() == Some(&'$') {
                    chars.next();
                    let opening: String =
                        std::iter::once('$').chain(tag).chain(std::iter::once('$')).collect();
                    current.push_str(&opening);
                    let closing = opening.clone();
                    let mut body = String::new();
                    for ch in chars.by_ref() {
                        body.push(ch);
                        if body.ends_with(closing.as_str()) {
                            break;
                        }
                    }
                    current.push_str(&body);
                } else {
                    current.push('$');
                    current.extend(tag);
                }
            }
            ';' => {
                let stmt = current.trim().to_string();
                if !stmt.is_empty() {
                    statements.push(stmt);
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }

    let tail = current.trim().to_string();
    if !tail.is_empty() {
        statements.push(tail);
    }
    statements
}

#[cfg(test)]
mod tests {
    use super::split_sql_statements;

    #[test]
    fn splits_multiple_statements() {
        let stmts = split_sql_statements("CREATE TABLE a (id int);CREATE TABLE b (id int);");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "CREATE TABLE a (id int)");
        assert_eq!(stmts[1], "CREATE TABLE b (id int)");
    }

    #[test]
    fn strips_line_comments() {
        let sql =
            "-- header comment\nCREATE TABLE a (id int); -- trailing\nCREATE TABLE b (id int);";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "CREATE TABLE a (id int)");
        assert_eq!(stmts[1], "CREATE TABLE b (id int)");
    }

    #[test]
    fn keeps_semicolon_inside_single_quotes() {
        let stmts = split_sql_statements("COMMENT ON TABLE t IS 'a;b';COMMENT ON TABLE t2 IS 'c';");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "COMMENT ON TABLE t IS 'a;b'");
    }

    #[test]
    fn keeps_semicolon_inside_double_quotes() {
        let stmts = split_sql_statements(
            r#"ALTER TABLE t ADD COLUMN "col;name" int;COMMENT ON TABLE t IS 'x';"#,
        );
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], r#"ALTER TABLE t ADD COLUMN "col;name" int"#);
    }

    #[test]
    fn keeps_dollar_quoted_block_whole() {
        let sql =
            "DO $$\nBEGIN\n  ALTER TABLE a ADD COLUMN c int;\nEND\n$$;CREATE TABLE b (id int);";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].starts_with("DO $$"));
        assert!(stmts[0].contains("ALTER TABLE a ADD COLUMN c int;"));
        assert!(stmts[0].ends_with("$$"));
    }

    #[test]
    fn keeps_tagged_dollar_quote_whole() {
        let sql = "$body$SELECT 1; SELECT 2;$body$;CREATE TABLE b (id int);";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "$body$SELECT 1; SELECT 2;$body$");
    }

    #[test]
    fn handles_escaped_quotes() {
        let stmts = split_sql_statements("COMMENT ON TABLE t IS 'it''s';");
        assert_eq!(stmts.len(), 1);
        assert_eq!(stmts[0], "COMMENT ON TABLE t IS 'it''s'");
    }

    #[test]
    fn preserves_unicode_in_strings_and_comments() {
        let stmts = split_sql_statements("-- 中文注释\nCOMMENT ON TABLE t IS '中文;内容';");
        assert_eq!(stmts.len(), 1);
        assert_eq!(stmts[0], "COMMENT ON TABLE t IS '中文;内容'");
    }

    #[test]
    fn empty_and_comment_only_input_yield_nothing() {
        assert!(split_sql_statements("").is_empty());
        assert!(split_sql_statements("-- just a comment\n").is_empty());
    }

    #[test]
    fn ignores_dollar_parameters() {
        let stmts = split_sql_statements("SELECT $1, $2;");
        assert_eq!(stmts.len(), 1);
        assert_eq!(stmts[0], "SELECT $1, $2");
    }
}
