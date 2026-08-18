# Entity / Core 职责拆分方案

> 说明：方案中提及的连接池全局单例（`REDIS_CLIENT` / `RBATIS_DATABASE` / `OnceLock`）已在当前代码库中移除，连接池一律通过 `CoreState { db, redis }` 依赖注入。方案实施时**不应**在新 `core` 中恢复这些全局单例。

> **✅ 落地状态（2026-08）**：本方案已实施，新 crate 实际命名为 **`common`**（`crates/common`，非 `core`）——下文所有 `core` 均对应 `common`。已落地：entity 精简为纯实体 crate（`lib.rs` = `pub mod ddl; pub mod models;` + tests，`ddl.rs` 为拆分后新增）、全局单例移除、internal_quic_msg 迁至 `common/src/utils/`、api 不再依赖 entity。**未实施**：quic_service / http_service 移除 entity 依赖（仍直接 `use entity::models::...`）、全量 `entity::` → `common::` 替换、MessageType 去重。各节附有实际落地标注。

## 1. 目标

将当前 `entity` crate 拆分为两个独立的 crate：

- **`entity`**：只负责数据库实体（ORM 模型），依赖极简，不依赖 `core`
- **`core`**：承载所有基础设施——工具函数、全局状态、配置、常量、宏、非 DB 模型

拆分后的依赖方向：

```
          ┌──────────┐
          │  entity  │  (ORM 模型，纯数据)
          └────┬─────┘
               │ 依赖
          ┌────▼─────┐
          │   core   │  (基础设施：Redis/SQL/JWT/配置/常量)
          └────┬─────┘
               │ 依赖
     ┌─────────┼─────────┐
     ▼         ▼         ▼
┌──────┐ ┌──────┐ ┌──────┐
│ quic │ │ http │ │ api  │
└──────┘ └──────┘ └──────┘
```

---

## 2. 现状分析

> ⚠️ 本节为拆分**前**的 entity 快照（已过期）。当前 `crates/entity/src` 仅含 `lib.rs`、`ddl.rs`、`models/`、`tests.rs`；`models` 下新增 `group_entity/`（5 个模型），`chat_entity` 多出 `add_read_chat_record.rs`。

当前 `entity` crate 的完整内容（拆分前）：

```
entity/src/
├── lib.rs                     # REDIS_CLIENT, RBATIS_DATABASE, OnceLock
├── config_manager.rs          # GLOBAL_CONFIG DashMap
├── config_str.rs              # 字符串常量
├── tracing.rs                 # tracing 初始化
├── macros/
│   ├── mod.rs
│   └── read_config.rs         # init_global_config!, read_global_config! 宏
├── models/
│   ├── mod.rs
│   ├── internal_quic_msg.rs   # ⚠️ 非 DB 实体
│   ├── chat_entity/
│   │   ├── mod.rs
│   │   ├── chat_list_link.rs
│   │   ├── chat_message_read.rs
│   │   └── chat_message_record.rs
│   ├── file_entity/
│   │   ├── mod.rs
│   │   ├── biz_file_link.rs
│   │   ├── biz_record.rs
│   │   ├── chat_biz_record.rs
│   │   ├── file_upload_record.rs
│   │   └── private_biz_record.rs
│   ├── notify_entity/
│   │   ├── mod.rs
│   │   └── system_notification.rs
│   └── user_entity/
│       ├── mod.rs
│       ├── basic_user.rs
│       ├── friend_link.rs
│       ├── friend_request_info.rs
│       ├── user_info.rs
│       └── user_login_log.rs
└── utils/
    ├── mod.rs
    ├── internal_quic_client.rs
    ├── jwt_util.rs
    ├── message_types.rs
    ├── redis_utils.rs
    ├── rsa_util.rs
    ├── sql_utils.rs
    ├── time.rs
    └── validators.rs
```

---

## 3. 拆分边界

### 3.1 `entity` — 仅保留 DB 实体

```
entity/src/
├── lib.rs
│   └── pub mod ddl; pub mod models;  # 实际：pub mod ddl + models（+ #[cfg(test)] mod tests）
├── ddl.rs                # 拆分后新增：apply_all_ddl 自动执行目录内全部 .sql
├── tests.rs              # 拆分后新增
└── models/
    ├── mod.rs            # pub mod chat_entity; pub mod file_entity; ...
    ├── chat_entity/      # 含新增 add_read_chat_record.rs
    ├── file_entity/      # 不变
    ├── group_entity/     # 拆分后新增：group_info/group_invitation/group_member/
    │                     #   group_message_read/group_message_record
    ├── notify_entity/    # 不变
    └── user_entity/      # 不变
```

**`entity/Cargo.toml` 精简后的依赖**：

```toml
[dependencies]
serde = { workspace = true }
rbatis = { workspace = true }
rbdc-pg = { workspace = true }
rbdc-pool-fast = { workspace = true }
validator = { workspace = true }
rbs = { workspace = true }
uuid = { workspace = true }
anyhow = { workspace = true }    # 实际保留（ddl.rs 需要）
tracing = { workspace = true }   # 实际保留
```

移除的依赖：`quinn`, `rustls`, `serde_json`, `tracing-subscriber`, `tracing-appender`, `lazy_static`, `tokio`, `deadpool-redis`, `jsonwebtoken`, `rsa`, `rand`, `sha2`, `once_cell`, `regex`, `dashmap`。（`anyhow` / `tracing` 实际保留，另有 dev-dependencies。）

### 3.2 `core` — 所有基础设施

> 实际为 `common`。目录无 `models/`（internal_quic_msg 在 `utils/` 下），另多出 `state.rs` 与 `utils/fatal_error.rs`、`utils/group_msg.rs`、`utils/server_count_sync.rs`、`utils/text_msg.rs`。

```
common/src/
├── lib.rs                     # 无全局单例；pub mod config_str; config_manager; macros;
│                              #   state; tracing; utils; + re-exports
├── state.rs                   # 实际新增：CoreState { db: RBatis, redis: Pool }
├── config_manager.rs          # GLOBAL_CONFIG DashMap
├── config_str.rs              # 所有字符串常量
├── tracing.rs                 # init_tracing
├── macros/
│   ├── mod.rs
│   └── read_config.rs         # 宏
└── utils/
    ├── mod.rs
    ├── fatal_error.rs         # 实际新增：fatal_panic(_async)
    ├── group_msg.rs           # 实际新增：InternalGroupBroadcast / BroadcastType / GroupQuicMsg
    ├── internal_quic_client.rs
    ├── internal_quic_msg.rs   # 实际位置：InternalQuicRequest / InternalQuicResponse（无 models/ 目录）
    ├── jwt_util.rs
    ├── message_types.rs
    ├── redis_utils.rs
    ├── rsa_util.rs
    ├── server_count_sync.rs   # 实际新增：SERVER_COUNT + start_server_count_sync
    ├── sql_utils.rs
    ├── text_msg.rs            # 实际新增：TextQuicMsg / MessageType enum（自 quic_service 迁入）
    ├── time.rs
    └── validators.rs
```

**`common/Cargo.toml` 依赖**（实际）：含方案列出的 `quinn` / `rustls` / `serde_json` / `anyhow` / `tracing(-subscriber/appender)` / `tokio` / `deadpool-redis` / `jsonwebtoken` / `rsa` / `uuid` / `rand` / `once_cell` / `regex` 等；差异：**无 `sha2`（改用 `argon2`）**、新增 `bincode` / `crc` / `crc-catalog` / `nanoid` / `toml`，`dashmap = { workspace = true }`。

---

## 4. 各 crate 依赖变更

### 4.1 根 `Cargo.toml`

```toml
[workspace]
members = [
    "crates/entity",
    "crates/common",       # 新增（实际命名 common）
    "crates/http_service",
    "crates/quic_service",
    "crates/api",
    "crates/email_service",
    "crates/s3_service"
]

[dependencies]
entity = { path = "crates/entity" }
common = { path = "crates/common" }   # 实际：根 Cargo.toml 显式添加（main.rs 直接 use common::tracing / common::fatal_panic_async）

[workspace.dependencies]
# 不变
```

### 4.2 `quic_service/Cargo.toml`

```toml
[dependencies]
entity = { path = "../entity" }    # 改为 common，entity 由 common re-export
common = { path = "../common" }    # 新增
# 移除 entity 依赖（common 已包含）
```

> ⚠️ **未实施**：`crates/quic_service/Cargo.toml` 实际**同时**依赖 `common` 和 `entity`；源码中 `external/quic_server.rs`、`msg_service/group_msg_service.rs` 仍直接 `use entity::models::...`。

### 4.3 `http_service/Cargo.toml`

```toml
[dependencies]
entity = { path = "../entity" }    # 改为 common
common = { path = "../common" }    # 新增
s3_service = { path = "../s3_service" }
# 移除 entity 依赖
```

> ⚠️ **未实施**：`crates/http_service/Cargo.toml` 实际同时依赖 `common` 和 `entity`；`group_service.rs` 仍直接 `use entity::models::...`。

### 4.4 `api/Cargo.toml`

```toml
[dependencies]
http_service = { path = "../http_service" }
quic_service = { path = "../quic_service" }
entity = { path = "../entity" }    # 改为 common
common = { path = "../common" }    # 新增
s3_service = { path = "../s3_service" }
# 移除 entity 依赖
```

> ✅ 已实施：`api/Cargo.toml` 无 entity 依赖。

### 4.5 保持不变的 crate

- `email_service` — 不依赖 `entity` 或 `core` ✅
- `s3_service` — 实际已改为依赖 `common`（与方案不符；`common` 不是 `entity` 或 `core`，此处仅修正依赖对象）

---

## 5. 各 crate import 路径变更对照

### 5.1 `quic_service`

> ✅ 大部分已改为 `common::`；仍存在残留：`external/quic_server.rs`、`msg_service/group_msg_service.rs` 中的 `entity::models::...` 导入未替换（见 §4.2）。

| 改动前 | 改动后 |
|--------|--------|
| `entity::RBATIS_DATABASE` | `core::RBATIS_DATABASE` |
| `entity::REDIS_CLIENT` | `core::REDIS_CLIENT` |
| `entity::config_str::*` | `core::config_str::*` |
| `entity::models::chat_entity::*` | `core::models::chat_entity::*`（或 `entity::models::chat_entity::*`） |
| `entity::models::file_entity::*` | `entity::models::file_entity::*`（不变） |
| `entity::utils::message_types` | `core::utils::message_types` |
| `entity::utils::time::*` | `core::utils::time::*` |
| `entity::utils::jwt_util::*` | `core::utils::jwt_util::*` |
| `entity::utils::redis_utils::*` | `core::utils::redis_utils::*` |
| `entity::utils::sql_utils::*` | `core::utils::sql_utils::*` |

### 5.2 `http_service`

| 改动前 | 改动后 |
|--------|--------|
| `entity::config_str::*` | `core::config_str::*` |
| `entity::models::*` | `entity::models::*`（不变） |
| `entity::utils::*` | `core::utils::*` |
| `entity::REDIS_CLIENT` | `core::REDIS_CLIENT` |

### 5.3 `api`

| 改动前 | 改动后 |
|--------|--------|
| `entity::init_redis` | `core::init_redis` |
| `entity::init_sql_pool` | `core::init_sql_pool` |
| `entity::init_global_config` | `core::init_global_config` |
| `entity::read_global_config` | `core::read_global_config` |
| `entity::config_str::*` | `core::config_str::*` |
| `entity::utils::internal_quic_client::*` | `core::utils::internal_quic_client::*` |
| `entity::models::internal_quic_msg::*` | `common::utils::internal_quic_msg::*`（实际无 models/ 目录） |
| `entity::config_manager::*` | `core::config_manager::*` |

### 5.4 `src/main.rs`（根）

| 改动前 | 改动后 |
|--------|--------|
| `entity::tracing::init_tracing` | `core::tracing::init_tracing` |

### 5.5 `quic_service/src/bin/quic_server.rs`

如果有独立的测试二进制入口，同样替换 `entity::` 为 `core::`。

---

## 6. `core/src/lib.rs` 关键实现

```rust
// 实际（crates/common/src/lib.rs）：无 models 模块，无全局单例
pub mod config_str;
pub mod config_manager;
pub mod macros;
pub mod state;     // CoreState { db, redis }
pub mod tracing;
pub mod utils;

// Re-export（实际还包括 verify_redis、fatal_panic(_async)、
// SERVER_COUNT / get_server_count / start_server_count_sync 等）
pub use utils::internal_quic_client::send_internal_quic_msg;
pub use utils::redis_utils::init_redis;
pub use utils::sql_utils::init_sql_pool;

// Re-export entity 的 models，使外部只需依赖 core 即可访问 DB 实体
pub use entity::models;
```


> **关键设计**：`core` 通过 `pub use entity::models;` 将 DB 实体 re-export，其他 crate 只需依赖 `core` 即可同时访问基础设施和 DB 实体，无需单独依赖 `entity`。

---

## 7. `entity/src/lib.rs` 精简

```rust
pub mod ddl;                 // 实际新增：apply_all_ddl 自动执行目录内全部 .sql
pub mod models;

#[cfg(test)]
mod tests;
```

极简。不再包含任何全局状态、工具函数、配置。

---

## 8. 重复定义问题：`MessageType` enum

### 8.1 现状

存在两处消息类型定义：

| 位置 | 形式 |
|------|------|
| [text_msg.rs](../crates/common/src/utils/text_msg.rs) | `enum MessageType` (带 `#[repr(u16)]`；已从 quic_service/src/models/ 迁入，原处仅剩 re-export) |
| [message_types.rs](../crates/common/src/utils/message_types.rs) | `const MSG_TYPE_*: u16` 常量（已迁至 common/src/utils/） |

两处功能重复，数值基本对应但有细微差异（**该问题仍未解决**）。

### 8.2 建议

统一由 `core::utils::message_types` 的常量作为唯一来源，`quic_service` 的 `MessageType` enum 改为引用常量，或者直接删除 enum 改用常量。

**本次拆分方案暂不处理此问题**，留待后续统一。

---

## 9. 实施步骤

### 阶段一：创建 `core` crate

1. 创建 `crates/core/` 目录结构（`src/` + 子目录）
2. 编写 `crates/core/Cargo.toml`，包含所有需要的基础设施依赖
3. 编写 `crates/core/src/lib.rs`（全局状态 + re-exports）

### 阶段二：迁移文件到 `core`

4. 从 `entity/src/utils/` 迁移全部文件到 `core/src/utils/`
5. 从 `entity/src/` 迁移 `config_manager.rs`、`config_str.rs`、`tracing.rs` 到 `core/src/`
6. 从 `entity/src/macros/` 迁移全部文件到 `core/src/macros/`
7. 从 `entity/src/models/internal_quic_msg.rs` 迁移到 `core/src/models/`
8. 更新 `core` 内部各文件的 `crate::` 引用

### 阶段三：精简 `entity`

9. 删除 `entity/src/` 中已迁移的文件和目录
10. 精简 `entity/src/lib.rs` 为 `pub mod models;`
11. 精简 `entity/Cargo.toml` 依赖，移除不再需要的包

### 阶段四：更新依赖

12. 在根 `Cargo.toml` 的 `[workspace].members` 中添加 `"crates/core"`
13. 更新 `quic_service/Cargo.toml`：添加 `core` 依赖，移除 `entity` 依赖
14. 更新 `http_service/Cargo.toml`：添加 `core` 依赖，移除 `entity` 依赖
15. 更新 `api/Cargo.toml`：添加 `core` 依赖，移除 `entity` 依赖

### 阶段五：全局 import 替换

16. `quic_service/` 下所有文件：`entity::` → `core::`
17. `http_service/` 下所有文件：`entity::` → `core::`
18. `api/` 下所有文件：`entity::` → `core::`
19. 根 `src/main.rs`：`entity::` → `core::`
20. `quic_service/src/bin/quic_server.rs`（如有）：`entity::` → `core::`

### 阶段六：编译验证

21. 执行 `cargo check`，修复所有编译错误
22. 执行 `cargo build`，确保完整编译通过
23. 执行 `cargo test`（如有测试），确保测试通过

---

## 10. 需要额外注意的文件

以下文件依赖关系较复杂，需重点检查：

| 文件 | 原因 |
|------|------|
| `common/src/utils/sql_utils.rs` | `init_sql_pool()`（纯连接池构造，无全局单例） |
| `common/src/utils/redis_utils.rs` | `init_redis()`（纯连接池构造，无全局单例） |
| `common/src/utils/internal_quic_client.rs` | 实际引用 `crate::utils::internal_quic_msg`（internal_quic_msg 在 utils/ 下，无 models/ 目录） |
| `quic_service/src/internal/internal_quic_server.rs` | 使用 `common::utils::internal_quic_msg`，已改 |
| `api/src/service/user_integrated_service.rs` | 使用 `common::utils::internal_quic_client` 和 `common::utils::internal_quic_msg`，已改 |
| `quic_service/src/init_server.rs` | 使用 `common::init_redis`、`common::init_sql_pool`、`common::config_str`，已改 |
| `api/src/init_server.rs` | 使用多个 `common::` 项（`init_global_config`、`init_redis`、`init_sql_pool` 等），已改 |
| `quic_service/src/external/quic_server.rs` | 使用 `common::config_str`、`common::utils`，已改（仍残留 `entity::models::` 导入） |
| `quic_service/src/nat_ip/nat_udp_service.rs` | 使用 `common::config_str`、`common::utils`，已改 |

---

## 11. 风险与回退

| 风险 | 缓解措施 |
|------|----------|
| import 路径遗漏 | 步骤 16-20 使用全局搜索替换 `entity::` → `core::` 后，再用 `cargo check` 逐文件验证 |
| entity 模型被 core re-export 后，`entity::models` 和 `core::models` 歧义 | 统一使用 `core::models`，`entity` 不再被外部直接依赖 |
| 宏路径变更 | `read_global_config!` / `init_global_config!` 宏改为 `core::` 前缀后，确保宏内部引用也正确 |
| 编译时间变长 | 新增一个 crate 编译单元，但总体依赖图未膨胀，影响可忽略 |

---

## 12. 文件变更清单

### 12.1 新增文件

| 文件 | 说明 | 落地状态 |
|------|------|---------|
| `crates/common/Cargo.toml` | 新 crate 依赖声明 | ✅ |
| `crates/common/src/lib.rs` | 核心入口 + re-exports（无全局单例） | ✅ |
| `crates/common/src/state.rs` | `CoreState { db, redis }` | ✅（方案未列） |
| `crates/common/src/config_manager.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/config_str.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/tracing.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/utils/internal_quic_msg.rs` | 从 entity 的 `models/internal_quic_msg.rs` 迁移 | ✅（实际放 utils/，无 models/ 目录） |
| `crates/common/src/utils/mod.rs` | 新 utils 模块入口 | ✅ |
| `crates/common/src/utils/internal_quic_client.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/utils/jwt_util.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/utils/message_types.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/utils/redis_utils.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/utils/rsa_util.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/utils/sql_utils.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/utils/time.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/utils/validators.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/macros/mod.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/macros/read_config.rs` | 从 entity 迁移 | ✅ |
| `crates/common/src/utils/fatal_error.rs` / `group_msg.rs` / `server_count_sync.rs` / `text_msg.rs` | 新增（方案未列） | ✅ |

### 12.2 修改文件

| 文件 | 改动内容 | 落地状态 |
|------|---------|---------|
| `Cargo.toml`（根） | `[workspace].members` 新增 `"crates/common"` | ✅ |
| `crates/entity/Cargo.toml` | 精简依赖（anyhow/tracing 实际保留） | ✅ |
| `crates/entity/src/lib.rs` | 仅保留 `pub mod models;`（实际另有 `pub mod ddl;` + tests） | ✅ |
| `crates/quic_service/Cargo.toml` | 新增 `common` 依赖，移除 `entity` | ❌ 未实施（两依赖共存） |
| `crates/http_service/Cargo.toml` | 新增 `common` 依赖，移除 `entity` | ❌ 未实施（两依赖共存） |
| `crates/api/Cargo.toml` | 新增 `common` 依赖，移除 `entity` | ✅（无 entity） |
| `src/main.rs` | `entity::tracing` → `common::tracing` | ✅ |
| `quic_service/src/**/*.rs` | 全局替换 `entity::` → `common::` | ⚠️ 大部分完成，`external/quic_server.rs`、`group_msg_service.rs` 有残留 |
| `http_service/src/**/*.rs` | 全局替换 `entity::` → `common::` | ⚠️ 大部分完成，`group_service.rs` 有残留 |
| `api/src/**/*.rs` | 全局替换 `entity::` → `common::` | ✅ |

### 12.3 删除文件（从 entity 中移除）

| 文件 | 原因 |
|------|------|
| `crates/entity/src/utils/` | 全部迁移到 core |
| `crates/entity/src/config_manager.rs` | 迁移到 core |
| `crates/entity/src/config_str.rs` | 迁移到 core |
| `crates/entity/src/tracing.rs` | 迁移到 core |
| `crates/entity/src/macros/` | 全部迁移到 core |
| `crates/entity/src/models/internal_quic_msg.rs` | 迁移到 core |
