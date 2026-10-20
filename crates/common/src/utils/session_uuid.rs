//! 会话标识派生。
//!
//! 单聊: 由用户对规范化后经 UUID v5 确定性派生, (A,B) 与 (B,A) 恒等;
//! 群聊: 不派生, 直接使用 group_uuid 作为 session_uuid。

use once_cell::sync::Lazy;
use uuid::Uuid;

use crate::config_str::CONV_NAMESPACE_STR;

/// 命名空间常量(启动时解析一次)。
static CONV_NAMESPACE: Lazy<Uuid> =
    Lazy::new(|| Uuid::parse_str(CONV_NAMESPACE_STR).expect("CONV_NAMESPACE_STR 非法"));

/// 单聊会话标识: 由用户对派生, 双向对称。
///
/// - 规范化: 比较使用字节序(`as_bytes`), 与字符串大小写写法无关
///   (`Uuid::parse_str` 本身大小写不敏感, 解析后即为统一形态)
/// - 拼接: `{小uuid}:{大uuid}`, 分隔符 `:` 不会出现在 uuid 文本中, 无歧义
pub fn single_session_uuid(a: &Uuid, b: &Uuid) -> Uuid {
    let (lo, hi) = if a.as_bytes() <= b.as_bytes() { (a, b) } else { (b, a) };
    Uuid::new_v5(&CONV_NAMESPACE, format!("{lo}:{hi}").as_bytes())
}

/// 群聊会话标识: 恒等映射, group_uuid 即 session_uuid。
///
/// 提供此函数是为了让调用方对"单聊要派生 / 群聊直接用"的分支有统一的表达,
/// 也避免调用方散落硬编码。
pub const fn group_session_uuid(group_uuid: Uuid) -> Uuid {
    group_uuid
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const A: Uuid = Uuid::nil();
    const B: Uuid = Uuid::from_u128(1);

    #[test]
    fn symmetric() {
        assert_eq!(single_session_uuid(&A, &B), single_session_uuid(&B, &A));
    }

    #[test]
    fn self_session_is_stable() {
        let s = single_session_uuid(&A, &A);
        assert_eq!(s, single_session_uuid(&A, &A));
        assert_ne!(s, A); // 自会话标识 != 用户自身 uuid
    }

    #[test]
    fn different_pairs_differ() {
        let c = Uuid::from_u128(2);
        assert_ne!(single_session_uuid(&A, &B), single_session_uuid(&A, &c));
    }

    #[test]
    fn case_insensitive_parse_then_same_session() {
        // 大小写两种写法解析后派生结果一致
        let lo = "00000000-0000-0000-0000-000000000000";
        let hi = "00000000-0000-0000-0000-000000000001";
        let s1 = single_session_uuid(
            &Uuid::parse_str(lo).expect("解析 lo 失败"),
            &Uuid::parse_str(hi).expect("解析 hi 失败"),
        );
        let s2 = single_session_uuid(
            &Uuid::parse_str(&hi.to_uppercase()).expect("解析大写 hi 失败"),
            &Uuid::parse_str(lo).expect("解析 lo 失败"),
        );
        assert_eq!(s1, s2);
    }

    #[test]
    fn group_identity() {
        let g = Uuid::from_u128(42);
        assert_eq!(group_session_uuid(g), g);
    }

    #[derive(Deserialize)]
    struct VectorCase {
        a: String,
        b: String,
        expected: String,
    }

    #[derive(Deserialize)]
    struct Vectors {
        cases: Vec<VectorCase>,
    }

    /// ⚠️ 跨端一致性守护: 向量文件由任务01一次性生成, 客户端(src-tauri)复刻同一份。
    /// 两端派生不一致的第一排查入口即核对这份文件是否两端同版。
    #[test]
    fn matches_generated_vectors() {
        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/session_uuid_vectors.json"
        ))
        .expect("缺少向量文件 crates/common/testdata/session_uuid_vectors.json");
        let v: Vectors = serde_json::from_str(&raw).expect("向量文件格式错误");
        assert!(!v.cases.is_empty(), "向量文件不应为空");
        for c in &v.cases {
            let got = single_session_uuid(
                &Uuid::parse_str(&c.a).expect("解析 a 失败"),
                &Uuid::parse_str(&c.b).expect("解析 b 失败"),
            );
            assert_eq!(got.to_string(), c.expected, "case a={} b={}", c.a, c.b);
        }
    }
}
