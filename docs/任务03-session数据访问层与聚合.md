# 任务 03：session 数据访问层与聚合

> 系列：《会话列表与离线同步方案》分阶段执行的第三份任务书
> 来源：主方案 §4.3 / §6.1–6.2 / §6.5 / §7.1–7.4 / §9.2 / §12 阶段 3 + 阶段 4 的聚合触发点
> 状态：已实施（**依赖任务 01、02**：表与模型就位、消息表已含 `session_uuid`）—— 实施记录见 §12
> 定位：补齐 `session` / `user_session` 的全部数据访问方法 + 聚合任务 + 触发点接线。本任务后 `session` 表开始有数据

---

## 1. 目标与边界

### 1.1 本任务做什么

| # | 内容 |
|---|------|
| 1 | `Session` DAO：懒创建 upsert、`last_message_*` 单调更新 |
| 2 | `UserSession` DAO：upsert、已读游标（前进+钳制）、同步游标（前进）、软删（推到底）、入群初始化 |
| 3 | 聚合函数 `aggregate_user_sessions`：按用户收敛会话状态 → 建/更新 `session` 行 → 单调更新 |
| 4 | 触发点接线：~~`user_online` / `user_offline`（quic_service）~~ **已删除**（偏差记录 §1.7）→ 改 http_service 读路径（`/session/list` 首页 + `prepare_control`）；`accept_group_invitation`（http_service）保留 |
| 5 | 集成测试：共享测试设施重构 + 8 个数据访问用例 |

### 1.2 本任务不做什么

| 不做 | 归属 |
|------|------|
| HTTP 接口（`/session/read`、`/session/synced`、`/session/list`） | 任务 04 / 06 |
| 复活判定（读侧谓词）与 keyset 分页查询 | 任务 06 |
| `last_preview` 的生成策略 | 任务 06（开放问题，见 §3.4）—— 本任务聚合先填 NULL |
| Kafka（落库仍为同步直写） | 后续独立任务 |
| 定时兜底聚合（§6.1 触发表的第 4 行） | ~~任务 09~~ —— **已随偏差记录 §1.7 删除**：改读路径惰性聚合 |

---

## 2. 前置与两个实现决策

- [x] 任务 01 / 02 完成：`Session` / `UserSession` 模型存在、消息表已分区且含 `session_uuid`
- [x] 引用路径确认：entity 经 `common::models` re-export（`common/src/lib.rs:16` `pub use entity::models`），所有跨 crate 引用写 `common::models::session_entity::...`

### 2.1 决策 A：时间戳取 DB 端时钟

`created_at` / `updated_at` / `deleted_at` 的赋值**不在 Rust 侧取时间**，SQL 内联：

```sql
(extract(epoch from clock_timestamp()) * 1000)::bigint
```

理由：entity crate 不引 chrono（当前无此依赖）；集群多节点时钟差异收敛到 DB 单一时钟源。项目本就是 PG-only，方言无碍。

### 2.2 决策 B：写方法用 `rb.exec`，读方法用 `py_sql`

- **读**（select）：沿用 `#[rbatis::py_sql]` 宏（`select_unread` 等先例）
- **写**（insert upsert / update）：用 `rb.exec` + 原生 SQL + `value!` 参数绑定（`raw_insert` 的先例写法，但本任务是活代码）。原因：`crud!` 宏的 `insert` 不支持 `ON CONFLICT`；`py_sql` 对非 SELECT 语句的返回行为在不同 rbatis 版本有差异，`exec` 完全显式、无宏魔法

---

## 3. Session DAO（`entity/src/models/session_entity/session.rs`）

在现有 `crud!(Session {})` 之后追加 impl 块：

```rust
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbs::value;

impl Session {
    /// 懒创建会话行(幂等): 已存在时不覆盖任何字段。
    /// 用于聚合任务与控制信息变更前确保行存在。
    pub async fn upsert(rb: &dyn Executor, s: &Session) -> Result<(), rbatis::Error> {
        rb.exec(
            "INSERT INTO session (session_uuid, session_type, created_at, updated_at)
             VALUES ($1, $2,
                     (extract(epoch from clock_timestamp()) * 1000)::bigint,
                     (extract(epoch from clock_timestamp()) * 1000)::bigint)
             ON CONFLICT (session_uuid) DO NOTHING",
            vec![
                value!(s.session_uuid),
                value!(s.session_type),
            ],
        )
        .await?;
        Ok(())
    }

    /// 单调更新最后一条消息: 仅当新 id 更大时生效(§6.2)。
    /// last_preview 可为 None: NULL 不覆盖已有值(见 §3.4)。
    pub async fn update_last_message(
        rb: &dyn Executor,
        session_uuid: &Uuid,
        last_message_id: i64,
        last_message_at: i64,
        last_preview: Option<&str>,
    ) -> Result<u64, rbatis::Error> {
        let res = rb
            .exec(
                "UPDATE session
                 SET last_message_id = $1,
                     last_message_at = $2,
                     last_preview    = COALESCE($3, last_preview),
                     updated_at      = (extract(epoch from clock_timestamp()) * 1000)::bigint
                 WHERE session_uuid = $4
                   AND (last_message_id IS NULL OR last_message_id < $1)",
                vec![
                    value!(last_message_id),
                    value!(last_message_at),
                    value!(last_preview),
                    value!(session_uuid),
                ],
            )
            .await?;
        Ok(res.rows_affected)
    }
}
```

> `update_last_message` 返回受影响行数 —— 0 表示该会话行不存在（调用方应先 `upsert` 再 `update`）或 id 未前进。聚合逻辑里两者都属正常，记日志即可。

### 3.4 `last_preview` 开放问题（本任务置 NULL）

`chat_message_record.raw` 是 bincode 协议体（`HeadMsg` + `TextQuicMsg`），直接截断是乱码。preview 生成需要解协议取文本 —— 协议代码在 `common::utils::text_msg`，聚合在 entity 层做不了。**方案留给任务 06**（候选：聚合下沉到 quic_service / http_service 层做；或消息落库时由生产方顺手算好 preview 传入）。`COALESCE($3, last_preview)` 保证本任务填 NULL 不破坏后续任何赋值。

---

## 4. UserSession DAO（`entity/src/models/session_entity/user_session.rs`）

```rust
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbs::value;

impl UserSession {
    /// 懒创建用户会话行(幂等): 已存在时不覆盖任何字段(游标只前进,禁止回写覆盖)。
    pub async fn upsert(rb: &dyn Executor, u: &UserSession) -> Result<(), rbatis::Error> {
        rb.exec(
            "INSERT INTO user_session
                 (user_uuid, session_uuid, session_type, peer_uuid,
                  last_read_id, synced_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4,
                     COALESCE($5, 0), COALESCE($6, 0),
                     (extract(epoch from clock_timestamp()) * 1000)::bigint,
                     (extract(epoch from clock_timestamp()) * 1000)::bigint)
             ON CONFLICT (user_uuid, session_uuid) DO NOTHING",
            vec![
                value!(u.user_uuid),
                value!(u.session_uuid),
                value!(u.session_type),
                value!(u.peer_uuid),
                value!(u.last_read_id),
                value!(u.synced_id),
            ],
        )
        .await?;
        Ok(())
    }

    /// 已读游标推进: 只前进 + 钳制到 synced_id(§7.1/§4.3 不变式)。
    /// 返回受影响行数(0 = 无需推进或行不存在)。
    pub async fn update_last_read_id(
        rb: &dyn Executor,
        user_uuid: &Uuid,
        session_uuid: &Uuid,
        reported: i64,
    ) -> Result<u64, rbatis::Error> {
        let res = rb
            .exec(
                "UPDATE user_session
                 SET last_read_id = LEAST($1, synced_id),
                     updated_at   = (extract(epoch from clock_timestamp()) * 1000)::bigint
                 WHERE user_uuid = $2
                   AND session_uuid = $3
                   AND last_read_id < $1",
                vec![value!(reported), value!(user_uuid), value!(session_uuid)],
            )
            .await?;
        Ok(res.rows_affected)
    }

    /// 同步游标推进: 只前进(§7.3)。
    pub async fn update_synced_id(
        rb: &dyn Executor,
        user_uuid: &Uuid,
        session_uuid: &Uuid,
        new_id: i64,
    ) -> Result<u64, rbatis::Error> {
        let res = rb
            .exec(
                "UPDATE user_session
                 SET synced_id   = $1,
                     updated_at  = (extract(epoch from clock_timestamp()) * 1000)::bigint
                 WHERE user_uuid = $2
                   AND session_uuid = $3
                   AND synced_id < $1",
                vec![value!(new_id), value!(user_uuid), value!(session_uuid)],
            )
            .await?;
        Ok(res.rows_affected)
    }

    /// 软删会话: deleted_at 置当前时刻, 同时把 last_read_id 推到 session 最后一条消息(§9.2)。
    /// 推到底是必须的 —— 否则复活时未读数会把删除前的旧消息一起算上。
    pub async fn soft_delete(
        rb: &dyn Executor,
        user_uuid: &Uuid,
        session_uuid: &Uuid,
    ) -> Result<u64, rbatis::Error> {
        let res = rb
            .exec(
                "UPDATE user_session us
                 SET deleted_at  = (extract(epoch from clock_timestamp()) * 1000)::bigint,
                     last_read_id = GREATEST(us.last_read_id, s.last_message_id),
                     updated_at  = (extract(epoch from clock_timestamp()) * 1000)::bigint
                 FROM session s
                 WHERE us.user_uuid = $1
                   AND us.session_uuid = $2
                   AND s.session_uuid = us.session_uuid",
                vec![value!(user_uuid), value!(session_uuid)],
            )
            .await?;
        Ok(res.rows_affected)
    }

    /// 入群初始化游标(§7.4): last_read_id / synced_id 初始化为群当前最新消息 id,
    /// 避免新成员把全部群历史视为未读。幂等(ON CONFLICT DO NOTHING)。
    pub async fn init_for_group_join(
        rb: &dyn Executor,
        user_uuid: &Uuid,
        group_uuid: &Uuid,
    ) -> Result<(), rbatis::Error> {
        rb.exec(
            "INSERT INTO user_session
                 (user_uuid, session_uuid, session_type, peer_uuid,
                  last_read_id, synced_id, created_at, updated_at)
             VALUES ($1, $2, 2, NULL,
                     (SELECT COALESCE(MAX(id), 0) FROM group_message_record WHERE group_uuid = $2),
                     (SELECT COALESCE(MAX(id), 0) FROM group_message_record WHERE group_uuid = $2),
                     (extract(epoch from clock_timestamp()) * 1000)::bigint,
                     (extract(epoch from clock_timestamp()) * 1000)::bigint)
             ON CONFLICT (user_uuid, session_uuid) DO NOTHING",
            vec![value!(user_uuid), value!(group_uuid)],
        )
        .await?;
        Ok(())
    }
}
```

---

## 5. 聚合函数（新文件 `entity/src/models/session_entity/aggregate.rs`）

### 5.1 发现查询（读侧，py_sql）

`ChatMessageRecord` impl 追加（`chat_message_record.rs`）：

```rust
/// 每个相关会话的最新一条消息(distinct on 按会话取 id 最大行)。
/// ⚠️ 不含 session_uuid 等值条件, 分区表上跨全分区扫描 —— 开发期可接受,
/// 数据量上来后由任务 09 评估(见 §8 性能注记)。
#[rbatis::py_sql(
    "select distinct on (session_uuid) * from chat_message_record
     where recv_user = #{me} or send_user = #{me}
     order by session_uuid, id desc"
)]
async fn select_latest_per_session_for_user(
    rb: &dyn Executor,
    me: &Uuid,
) -> Vec<ChatMessageRecord> {}

pub async fn latest_per_session_for_user(
    rb: &dyn Executor,
    me: &Uuid,
) -> rbatis::Result<Vec<ChatMessageRecord>> {
    Ok(Self::select_latest_per_session_for_user(rb, me).await?)
}
```

`GroupMember` impl 追加（若已有等价方法则复用）：

```rust
/// 我所在(状态正常)的全部群 uuid。
#[rbatis::py_sql(
    "select * from group_member where user_uuid = #{me} and status = 1"
)]
async fn select_by_user_status_normal(rb: &dyn Executor, me: &Uuid) -> Vec<GroupMember> {}
```

`GroupMessageRecord` impl 追加：

```rust
/// 指定群的最新一条消息。
#[rbatis::py_sql(
    "select * from group_message_record where group_uuid = #{group_uuid} order by id desc limit 1"
)]
async fn select_latest_by_group_inner(rb: &dyn Executor, group_uuid: &Uuid) -> Vec<GroupMessageRecord> {}

pub async fn select_latest_by_group(
    rb: &dyn Executor,
    group_uuid: &Uuid,
) -> rbatis::Result<Option<GroupMessageRecord>> {
    Ok(Self::select_latest_by_group_inner(rb, group_uuid).await?.into_iter().next())
}
```

### 5.2 聚合主函数

```rust
//! 会话聚合: 在会话列表被读取时(读路径)把消息表的最新状态收敛进 session 表。
//! 触发点: /session/list 首页 + 控制操作(http_service); 单聊由 user_session 驱动,
//! 群聊由成员表驱动, 均逐会话点查(无跨分区全扫)。

use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use tracing::{info, warn};

use crate::models::chat_entity::chat_message_record::ChatMessageRecord;
use crate::models::group_entity::group_message_record::GroupMessageRecord;
use crate::models::session_entity::session::{Session, SESSION_TYPE_GROUP, SESSION_TYPE_SINGLE};
use crate::models::session_entity::user_session::UserSession;

/// 聚合结果(仅用于日志观测)。
#[derive(Debug, Default)]
pub struct AggregateReport {
    pub single_sessions: usize,
    pub group_sessions: usize,
    pub user_session_upserted: usize,
}

/// 聚合指定用户的全部会话(§6.1 后台聚合任务)。
///
/// - 单聊: 从消息表发现会话 → session upsert + 单调更新 → 建 me 视角的 user_session(含 peer)
/// - 群聊: 从群成员表发现 → session upsert + 单调更新 → 建 me 视角的 user_session
/// - 只建 me 自己的 user_session 行; 对方的行在对方聚合时收敛
/// - 幂等: 重复执行结果一致(全部写路径单调/DO NOTHING)
pub async fn aggregate_user_sessions(
    rb: &dyn Executor,
    me: &Uuid,
) -> Result<AggregateReport, rbatis::Error> {
    let mut report = AggregateReport::default();

    // 1. 单聊
    let latest = ChatMessageRecord::latest_per_session_for_user(rb, me).await?;
    report.single_sessions = latest.len();
    for msg in latest {
        let session_uuid = msg.session_uuid;
        let peer = if msg.send_user == *me { msg.recv_user } else { msg.send_user };

        Session::upsert(
            rb,
            &Session {
                session_uuid,
                session_type: Some(SESSION_TYPE_SINGLE),
                last_message_id: None,
                last_message_at: None,
                last_preview: None,
                created_at: None,
                updated_at: None,
            },
        )
        .await?;
        Session::update_last_message(
            rb,
            &session_uuid,
            msg.id.unwrap_or(0),
            msg.timestamp.unwrap_or(0),
            None, // preview 策略见任务 06(§3.4)
        )
        .await?;

        UserSession::upsert(
            rb,
            &UserSession {
                id: None,
                user_uuid: *me,
                session_uuid,
                session_type: Some(SESSION_TYPE_SINGLE),
                peer_uuid: Some(peer),
                last_read_id: None,
                synced_id: None,
                pinned: None,
                muted: None,
                deleted_at: None,
                created_at: None,
                updated_at: None,
            },
        )
        .await?;
        report.user_session_upserted += 1;
    }

    // 2. 群聊
    let memberships = GroupMember::select_by_user_status_normal(rb, me).await?;
    report.group_sessions = memberships.len();
    for m in memberships {
        let Some(group_uuid) = m.group_uuid else { continue };
        Session::upsert(
            rb,
            &Session {
                session_uuid: group_uuid,
                session_type: Some(SESSION_TYPE_GROUP),
                ../* 同上填 None */
            },
        )
        .await?;
        if let Some(latest_msg) = GroupMessageRecord::select_latest_by_group(rb, &group_uuid).await? {
            Session::update_last_message(
                rb,
                &group_uuid,
                latest_msg.id.unwrap_or(0),
                latest_msg.timestamp.unwrap_or(0),
                None,
            )
            .await?;
        }
        // 群聊 user_session 走 init_for_group_join 语义(游标初始化), 但对已存在行为幂等
        UserSession::init_for_group_join(rb, me, &group_uuid).await?;
        report.user_session_upserted += 1;
    }

    info!("[session] 聚合完成: user={}, 单聊会话={}, 群会话={}",
        me, report.single_sessions, report.group_sessions);
    Ok(report)
}
```

> **`init_for_group_join` 复用说明**：对已存在的 `user_session` 行它是 no-op（`ON CONFLICT DO NOTHING`），不会把老成员游标重置 —— 行为恰好是聚合需要的「存在即跳过」。

### 5.3 注册

`session_entity/mod.rs`：

```rust
pub mod aggregate;
pub mod session;
pub mod user_session;
```

> aggregate 依赖 `GroupMember`（`crate::models::group_entity::group_member`），同 crate 内跨域引用无碍。

---

## 6. 触发点接线

> ⚠️ **§6.1 / §6.2 已被修订替代**（见偏差记录 §1.7）：quic_service 的 `user_online` / `user_offline`
> 聚合触发已**删除**（生产者不维护读侧派生状态）。聚合改由 **http_service 读路径**惰性触发：
> `list_sessions` 首页（`/session/list`，登录/断线重连必然发起）+ `prepare_control`（控制操作）。
> 单聊聚合从 `distinct on` 全扫改为 `user_session` 驱动的 `latest_by_session` 逐会话点查。
> 以下 §6.3 入群初始化钩子**保留**（行创建职责，非聚合）。

### 6.1 `user_online` / `user_offline`（已删除）

原实现在 `crates/quic_service/src/external/quic_server.rs`（`user_online` 成功返回路径末尾 + `user_offline`），
上线/下线时异步 spawn `aggregate_user_sessions`。现已整体移除，不再有代码落点。

### 6.3 入群初始化（http_service）

`crates/http_service/src/http_service/group_service/group_service.rs` 的 `accept_group_invitation_service`（接受入群邀请，入群成功分支）追加一行：

```rust
// 入群初始化会话游标(§7.4): 避免新成员把全部群历史视为未读
UserSession::init_for_group_join(rb, &user_uuid, &group_uuid).await?;
```

> 该函数当前会调用 `sync_group_members_to_redis` 刷新成员缓存 —— `init_for_group_join` 与其并列。若函数内没有现成的 `rb` 变量名以实际代码为准。

---

## 7. 测试

### 7.1 共享测试设施重构（先做）

把 `entity/tests/ddl_integration_test.rs` 的建库/连接/删库函数抽到共享模块：

```text
crates/entity/tests/
  ├── common/mod.rs                  ← 新增: build_pool / ensure_database /
  │                                     drop_test_database / admin_database_url /
  │                                     test_database_name / init_tracing
  ├── ddl_integration_test.rs        ← 改为 use common::*
  └── session_dao_integration_test.rs ← 新增
```

> Rust 集成测试目录下每个 `*_test.rs` 是独立二进制，`tests/common/mod.rs` 是标准共享方式。注意：抽出的函数改为 `pub`，逻辑不动（减小 diff，便于 review）。

### 7.2 集成测试用例（`session_dao_integration_test.rs`）

全部 `#[ignore]`（需本地 PG），与 ddl_integration_test 相同的运行方式。数据构造直接 `INSERT`（消息用 `ChatMessageRecord::insert`，`session_uuid` 用**固定 uuid 字面量**即可 —— 聚合不关心它的来源，真派生值的对称性已由任务 02 的集成测试覆盖）。

| 测试名 | 步骤与断言 |
|--------|-----------|
| `session_upsert_idempotent` | upsert 两次 → 查询仅 1 行，字段不被第二次覆盖 |
| `session_last_message_monotonic` | update(id=100) → update(id=50) → 读回 100（不回退） |
| `aggregate_single_chat` | 同会话插 2 条消息 → `aggregate(me)` → `session.last_message_id` = 第 2 条 id；`user_session(me)` 存在且 `peer_uuid` = 对方；再 `aggregate(peer)` → 对方的 `user_session` 也生成（**session 行共享，双方 upsert 同一行**） |
| `aggregate_group` | 造 `group_info` + `group_member(me)` + 2 条群消息 → 聚合 → `session(group_uuid).last_message_id` 正确；`user_session(me)` 游标 = max(id) |
| `read_id_forward_and_clamped` | 建行 synced=100 → `update_last_read_id(reported=150)` → 读回 **100**（钳制）；`reported=80` → 读回 80（前进） |
| `synced_id_forward_only` | `update_synced_id(100)` → `update_synced_id(50)` → 读回 100 |
| `soft_delete_advances_read` | 建行 last_read=0；session.last_message_id=200 → `soft_delete` → `deleted_at IS NOT NULL` 且 `last_read_id = 200` |
| `group_join_initializes_cursor` | 群内已有消息 max=300 → `init_for_group_join` → 新行 `last_read_id = synced_id = 300`；再次调用 → 值不变（幂等） |

### 7.3 单测（无 DB）

| 测试名 | 内容 |
|--------|------|
| `aggregate_report_default` | 结构体 Default 可用（防误删 derive） |
| `latest_by_session_sql_text` | py_sql 宏内 SQL 含 `where session_uuid = #{session_uuid}`（分区剪枝）与 `order by id desc limit 1`（反向取首行）—— 单聊聚合点查回归（原 `select_latest_per_session_sql_text` 的 distinct on 断言随全扫删除而移除） |

### 7.4 运行命令

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo +nightly fmt --all --check
cargo test --workspace --tests
cargo test -p entity --test session_dao_integration_test -- --ignored
cargo test -p entity --test ddl_integration_test -- --ignored
# 手动回归: 启动服务 → 登录(触发 user_online) → 查 session/user_session 表出现聚合数据
```

---

## 8. 性能注记（已随偏差记录 §1.7 落地）

- ~~`latest_per_session_for_user` 不含 `session_uuid` 等值条件 —— 分区表上跨 16 分区扫描~~ —— **已删除**。
  单聊聚合改 `user_session` 驱动的 `latest_by_session`（`session_uuid` 等值 + `id desc limit 1`，单分区点查），
  全扫问题随之消除
- **单次聚合耗时** = 用户会话数 × 单分区索引点查（O(会话数 × log n)），且仅在 `/session/list` 首页触发 —— 无读者零开销
- 若将来仍成为瓶颈，候选方案：消息落库时写轻量「脏会话」标记（Redis set），聚合只处理脏会话 —— 属优化，不在本系列范围

---

## 9. 验收（DoD）

- [x] `Session::upsert` / `update_last_message`、`UserSession` 5 个方法全部实现并被集成测试覆盖
- [x] `aggregate_user_sessions` 三类用例绿（单聊 / 群聊 / 双方收敛）
- [x] 单调性 3 项断言绿（last_message / last_read 钳制 / synced 不回退）
- [x] `soft_delete` 推到底 + `init_for_group_join` 幂等
- [x] `user_online` / `user_offline` / `accept_group_invitation_service` 三处接线完成，`cargo check` 全绿 —— **§6.1/§6.2 已随偏差记录 §1.7 删除**，聚合改读路径
- [ ] 手动验证：启动 → 登录 → PG 里 `session` / `user_session` 出现数据，重复登录不产生重复行（待人工验证）
- [x] 共享测试设施抽取后，`ddl_integration_test` 原用例全部仍绿（零行为变化）
- [x] `cargo clippy -D warnings` 无告警

---

## 10. 本任务产出（供后续任务消费）

| 产出 | 消费方 |
|------|--------|
| `Session::upsert` / `update_last_message` | 任务 06（控制信息变更前确保行存在） |
| `UserSession::update_last_read_id` | 任务 04（`/session/read` 直接调用，逻辑已在数据层） |
| `UserSession::update_synced_id` | 任务 04（`/session/synced` 直接调用） |
| `UserSession::soft_delete` | 任务 06（`/session/delete`） |
| `aggregate_user_sessions` + 触发点 | 任务 06（`/session/list` 的数据来源；偏差记录 §1.7 后触发点为列表首页 + 控制操作） |
| ~~`select_latest_per_session_for_user`~~ → `latest_by_session` | 已删除（跨分区全扫）；`latest_by_session`（单分区点查）供聚合与任务 06 对账 |

---

## 11. 系列任务规划（更新）

| 任务 | 主题 | 状态 |
|------|------|------|
| 01 | 会话表与会话标识 | 已实施（服务端 `4025bf2` / 客户端 `3cae1b1`） |
| 02 | 消息表分区与会话标识 + 消息表 bug | 已实施（`bd1e82f`） |
| **03（本档）** | session 数据访问层与聚合 | 已实施（见 §12） |
| 04 | 游标归一（`/session/read` + `/session/synced` + 删旧已读表） | 待写 —— 薄：数据层方法已就位，接口层只是薄壳 |
| 05 | 离线同步 `/session/sync`（7 天窗口） | 待写 |
| 06 | 会话列表 `/session/list` + preview 策略 + 控制信息接口 | 待写 |
| 07 | 客户端改造 | 待写 |
| 08 | 历史接口入参 `session_uuid` 化 + HTTP `Client` 复用 | 待写 |
| 09 | 分区行为验证 + 数据清理 + 定时兜底聚合 | 待写 |

---

## 12. 实施记录（实施后补记）

任务 03 已实施完成，与任务书原文的差异、补充决策与验证结果如下。

### 12.1 复用既有方法

群成员发现**复用** `GroupMember::select_groups_by_user`（`group_member.rs`），其 SQL 与任务书 §5.1 拟新增的 `select_by_user_status_normal` 完全一致（`where user_uuid = #{user_uuid} and status = 1`），故未新增重复方法。

### 12.2 触发点接线

- `quic_server.rs` 新增私有助手 `spawn_session_aggregate(core, uuid)`（`tokio::spawn` + 失败仅 `warn!`），在 `user_online` 成功返回前、`user_offline` 中调用，两处共用，避免重复代码。
- 任务书称"连接注册完成之后"，实际连接注册（`set_conn_info`）在 `handle_conn` 中于 `user_online` 返回后进行；聚合为异步 spawn 且不依赖连接注册，故按任务书置于 `user_online` 末尾（`Ok(lock_token)` 前）。
- `accept_group_invitation_service` 成功分支在 `sync_group_members_to_redis` 之后调用 `UserSession::init_for_group_join(rb, &u_uuid, &group_uuid)`（`&RBatis` 自动强转为 `&dyn Executor`）。

### 12.3 共享测试设施

- 抽取到 `crates/entity/tests/common/mod.rs`（函数改 `pub`、模块头 `#![allow(dead_code)]`，逻辑不动）。任务书所列 `ensure_database` 实际为任务 02 已改名的 `recreate_database`（分区表无法由普通表原地转换），一并迁入。
- `ddl_integration_test.rs` 改为 `mod common;`，仅保留 ddl 专属的 `TABLES` / `PARTITIONED_TABLES` / `table_exists` / `partition_count`。

### 12.4 测试库隔离与运行时（重要）

- `session_dao_integration_test.rs` 使用**独立默认库名** `only_talk_session_test`（`DEFAULT_SESSION_TEST_DATABASE`，`TEST_DATABASE_NAME` 仍可覆盖），与 ddl 测试的 `only_talk_test` 隔离，两个测试二进制可并行，避免 DROP 竞争。
- 该测试启动时重建空库 + 应用全部 DDL（`DB_READY` `OnceCell` 保证仅一次）；未接入 `DROP_TEST_DATABASE` 末尾清理 —— 每次运行开头都会重建，测试库保留供人工查看。
- **连接池绑定 tokio 运行时**：`#[tokio::test]` 每个用例各有独立运行时，跨用例共享同一连接池会在首个用例运行时关闭后失效（报 `A Tokio 1.x context was found, but it is being shutdown`）。故每个用例在自己的运行时内 `build_pool` 并 `Box::leak` 为 `&'static`，不改动各用例签名。

### 12.5 用例构造补充

- `read_id_forward_and_clamped`：任务书"reported=150 读回 100；reported=80 读回 80"不能在同一行成立（钳制后 `last_read=100`，再报 80 被"只前进"挡住）。改为两条独立行：一条验证钳制 + 不回退，一条验证从 0 前进到 80。
- `Session::upsert` 依赖 `session_type` 非空（列 NOT NULL），聚合处恒传 `Some(...)`。

### 12.6 验证结果

- `cargo check --workspace --all-targets` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo +nightly fmt --all --check` 全绿。
- `cargo test --workspace --tests` 全绿。
- `cargo test -p entity --test session_dao_integration_test -- --ignored`：8 passed。
- `cargo test -p entity --test ddl_integration_test -- --ignored`：1 passed（重构后零行为变化）。
- `cargo test -p quic_service --test external_server_integration_test -- --ignored`：2 passed。
- `cargo test -p http_service --test http_service_integration_test -- --ignored`：1 passed。
- 待人工验证：启动 → 登录 → PG 中 `session` / `user_session` 出现数据，重复登录不产生重复行。
