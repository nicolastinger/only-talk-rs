# Entity / Core 职责拆分方案

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

当前 `entity` crate 的完整内容：

```
entity/src/
├── lib.rs                     # REDIS_CLIENT, RBATIS_DATABASE, OnceLock
├── config_manager.rs          # GLOBAL_CONFIG DashMap
├── config_str.rs              # 字符串常量
├── tracing.rs                 # tracing 初始化
├── marcos/
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
│       ├── basic_user_salt.rs
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
│   └── pub mod models;
└── models/
    ├── mod.rs              # pub mod chat_entity; pub mod file_entity; ...
    ├── chat_entity/        # 不变
    ├── file_entity/        # 不变
    ├── notify_entity/      # 不变
    └── user_entity/        # 不变
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
```

移除的依赖：`quinn`, `rustls`, `serde_json`, `anyhow`, `tracing`, `tracing-subscriber`, `tracing-appender`, `lazy_static`, `tokio`, `deadpool-redis`, `jsonwebtoken`, `rsa`, `rand`, `sha2`, `once_cell`, `regex`, `dashmap`。

### 3.2 `core` — 所有基础设施

```
core/src/
├── lib.rs                     # REDIS_CLIENT, RBATIS_DATABASE, OnceLock, re-exports
├── config_manager.rs          # GLOBAL_CONFIG DashMap
├── config_str.rs              # 所有字符串常量
├── tracing.rs                 # init_tracing
├── marcos/
│   ├── mod.rs
│   └── read_config.rs         # 宏
├── models/
│   ├── mod.rs
│   └── internal_quic_msg.rs   # InternalQuicRequest / InternalQuicResponse
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

**`core/Cargo.toml` 依赖**：

```toml
[dependencies]
entity = { path = "../entity" }
quinn = { workspace = true }
rustls = { workspace = true }
serde_json = { workspace = true }
serde = { workspace = true }
rbatis = { workspace = true }
rbdc-pg = { workspace = true }
rbdc-pool-fast = { workspace = true }
anyhow = { workspace = true }
rbs = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["local-time"] }
tracing-appender = { workspace = true }
lazy_static = { workspace = true }
tokio = { workspace = true }
deadpool-redis = { workspace = true }
jsonwebtoken = { workspace = true }
rsa = { workspace = true }
uuid = { workspace = true }
rand = { workspace = true }
sha2 = { workspace = true }
once_cell = { workspace = true }
regex = { workspace = true }
dashmap = "6.1"
```

---

## 4. 各 crate 依赖变更

### 4.1 根 `Cargo.toml`

```toml
[workspace]
members = [
    "crates/entity",
    "crates/core",         # 新增
    "crates/http_service",
    "crates/quic_service",
    "crates/api",
    "crates/email_service",
    "crates/s3_service"
]

[dependencies]
entity = { path = "crates/entity" }
# 不需要显式添加 core，main.rs 通过 quic_service 或 api 间接使用

[workspace.dependencies]
# 不变
```

### 4.2 `quic_service/Cargo.toml`

```toml
[dependencies]
entity = { path = "../entity" }    # 改为 core，entity 由 core re-export
core = { path = "../core" }        # 新增
# 移除 entity 依赖（core 已包含）
```

### 4.3 `http_service/Cargo.toml`

```toml
[dependencies]
entity = { path = "../entity" }    # 改为 core
core = { path = "../core" }        # 新增
s3_service = { path = "../s3_service" }
# 移除 entity 依赖
```

### 4.4 `api/Cargo.toml`

```toml
[dependencies]
http_service = { path = "../http_service" }
quic_service = { path = "../quic_service" }
entity = { path = "../entity" }    # 改为 core
core = { path = "../core" }        # 新增
s3_service = { path = "../s3_service" }
# 移除 entity 依赖
```

### 4.5 保持不变的 crate

- `email_service` — 不依赖 `entity` 或 `core`
- `s3_service` — 不依赖 `entity` 或 `core`

---

## 5. 各 crate import 路径变更对照

### 5.1 `quic_service`

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
| `entity::models::internal_quic_msg::*` | `core::models::internal_quic_msg::*` |
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
use std::sync::Arc;
use std::sync::OnceLock;

use deadpool_redis::Pool;
use lazy_static::lazy_static;
use rbatis::RBatis;
use tokio::sync::RwLock;

pub mod config_str;
pub mod config_manager;
pub mod marcos;
pub mod models;
pub mod tracing;
pub mod utils;

// Re-export
pub use utils::internal_quic_client::send_internal_quic_msg;
pub use utils::redis_utils::init_redis;
pub use utils::sql_utils::init_sql_pool;

// Re-export entity 的 models，使外部只需依赖 core 即可访问 DB 实体
pub use entity::models;

/// 确保 Redis / SQL 只初始化一次
static REDIS_INIT_ONCE: OnceLock<()> = OnceLock::new();
static SQL_INIT_ONCE: OnceLock<()> = OnceLock::new();

lazy_static! {
    pub static ref REDIS_CLIENT: Arc<RwLock<Option<Pool>>> = Arc::new(RwLock::new(None));
    pub static ref RBATIS_DATABASE: Arc<RwLock<Option<RBatis>>> = Arc::new(RwLock::new(None));
}
```

> **关键设计**：`core` 通过 `pub use entity::models;` 将 DB 实体 re-export，其他 crate 只需依赖 `core` 即可同时访问基础设施和 DB 实体，无需单独依赖 `entity`。

---

## 7. `entity/src/lib.rs` 精简

```rust
pub mod models;
```

极简。不再包含任何全局状态、工具函数、配置。

---

## 8. 重复定义问题：`MessageType` enum

### 8.1 现状

存在两处消息类型定义：

| 位置 | 形式 |
|------|------|
| [text_msg.rs:8-24](../crates/quic_service/src/models/text_msg.rs#L8) | `enum MessageType` (带 `#[repr(u16)]`) |
| [message_types.rs](../crates/entity/src/utils/message_types.rs) | `const MSG_TYPE_*: u16` 常量 |

两处功能重复，数值基本对应但有细微差异。

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
6. 从 `entity/src/marcos/` 迁移全部文件到 `core/src/marcos/`
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
| `core/src/utils/sql_utils.rs` | `init_sql_pool()` 使用 `RBATIS_DATABASE` + `SQL_INIT_ONCE`，需确保在 `core` 的 `lib.rs` 中可见 |
| `core/src/utils/redis_utils.rs` | `init_redis()` 使用 `REDIS_CLIENT` + `REDIS_INIT_ONCE`，同上 |
| `core/src/utils/internal_quic_client.rs` | 引用 `crate::models::internal_quic_msg`，迁移后需改为 `core::models::internal_quic_msg` |
| `quic_service/src/internal_quic_server.rs` | 使用 `entity::models::internal_quic_msg`，需改为 `core::models::internal_quic_msg` |
| `api/src/service/user_integrated_service.rs` | 使用 `entity::utils::internal_quic_client` 和 `entity::models::internal_quic_msg`，需改为 `core::` |
| `quic_service/src/init_server.rs` | 使用 `entity::init_redis`、`entity::init_sql_pool`、`entity::config_str`，需改为 `core::` |
| `api/src/init_server.rs` | 使用多个 `entity::` 项（`init_global_config`、`init_redis`、`init_sql_pool` 等），需改为 `core::` |
| `quic_service/src/quic_server.rs` | 使用 `entity::REDIS_CLIENT`、`entity::config_str`、`entity::utils`，需改为 `core::` |
| `quic_service/src/nat_ip/nat_udp_service.rs` | 使用 `entity::REDIS_CLIENT`、`entity::config_str`、`entity::utils`，需改为 `core::` |

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

| 文件 | 说明 |
|------|------|
| `crates/core/Cargo.toml` | 新 crate 依赖声明 |
| `crates/core/src/lib.rs` | 核心入口 + 全局状态 + re-exports |
| `crates/core/src/config_manager.rs` | 从 entity 迁移 |
| `crates/core/src/config_str.rs` | 从 entity 迁移 |
| `crates/core/src/tracing.rs` | 从 entity 迁移 |
| `crates/core/src/models/mod.rs` | 新 models 模块入口 |
| `crates/core/src/models/internal_quic_msg.rs` | 从 entity 迁移 |
| `crates/core/src/utils/mod.rs` | 新 utils 模块入口 |
| `crates/core/src/utils/internal_quic_client.rs` | 从 entity 迁移 |
| `crates/core/src/utils/jwt_util.rs` | 从 entity 迁移 |
| `crates/core/src/utils/message_types.rs` | 从 entity 迁移 |
| `crates/core/src/utils/redis_utils.rs` | 从 entity 迁移 |
| `crates/core/src/utils/rsa_util.rs` | 从 entity 迁移 |
| `crates/core/src/utils/sql_utils.rs` | 从 entity 迁移 |
| `crates/core/src/utils/time.rs` | 从 entity 迁移 |
| `crates/core/src/utils/validators.rs` | 从 entity 迁移 |
| `crates/core/src/marcos/mod.rs` | 从 entity 迁移 |
| `crates/core/src/marcos/read_config.rs` | 从 entity 迁移 |

### 12.2 修改文件

| 文件 | 改动内容 |
|------|---------|
| `Cargo.toml`（根） | `[workspace].members` 新增 `"crates/core"` |
| `crates/entity/Cargo.toml` | 精简依赖 |
| `crates/entity/src/lib.rs` | 仅保留 `pub mod models;` |
| `crates/quic_service/Cargo.toml` | 新增 `core` 依赖，移除 `entity` |
| `crates/http_service/Cargo.toml` | 新增 `core` 依赖，移除 `entity` |
| `crates/api/Cargo.toml` | 新增 `core` 依赖，移除 `entity` |
| `src/main.rs` | `entity::tracing` → `core::tracing` |
| `quic_service/src/**/*.rs` | 全局替换 `entity::` → `core::` |
| `http_service/src/**/*.rs` | 全局替换 `entity::` → `core::` |
| `api/src/**/*.rs` | 全局替换 `entity::` → `core::` |

### 12.3 删除文件（从 entity 中移除）

| 文件 | 原因 |
|------|------|
| `crates/entity/src/utils/` | 全部迁移到 core |
| `crates/entity/src/config_manager.rs` | 迁移到 core |
| `crates/entity/src/config_str.rs` | 迁移到 core |
| `crates/entity/src/tracing.rs` | 迁移到 core |
| `crates/entity/src/marcos/` | 全部迁移到 core |
| `crates/entity/src/models/internal_quic_msg.rs` | 迁移到 core |
