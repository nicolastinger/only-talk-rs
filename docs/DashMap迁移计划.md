# GLOBAL_QUIC_SERVER_LIST DashMap 迁移计划

## 1. 目标

将 `GLOBAL_QUIC_SERVER_LIST` 从 `Arc<RwLock<HashMap<K, V>>>` 迁移为 `Arc<DashMap<K, V>>`，消除读写锁带来的竞争开销，简化代码。

---

## 2. 现状分析

### 2.1 当前定义

[lib.rs](../crates/quic_service/src/lib.rs#L22-L24)

```rust
lazy_static! {
    pub static ref GLOBAL_QUIC_SERVER_LIST: Arc<RwLock<HashMap<String, QuicConnection>>> =
        Arc::new(RwLock::new(HashMap::new()));
}
```

### 2.2 所有访问点

| # | 文件 | 行号 | 操作 | 当前写法 |
|---|------|------|------|---------|
| 1 | `quic_server.rs` | L162 | 读 `.len()` | `.read().await.len()` |
| 2 | `quic_server.rs` | L191 | 写 `.insert()` | `.write().await.insert(k, v)` |
| 3 | `quic_server.rs` | L202 | 读 `.len()` | `.read().await.len()` |
| 4 | `quic_server.rs` | L293-299 | 写 `.get_mut()` + `.remove()` | `.write().await.get_mut(k)` → `.remove(k)` |
| 5 | `quic_server.rs` | L312 | 读 `.len()` | `.read().await.len()` |
| 6 | `msg_service/mod.rs` | L20-21 | 读 `.get()` | `.read().await.get(k)` |
| 7 | `msg_service/send_msg.rs` | L24-25 | 读 `.get()` | `.read().await.get(k)` |
| 8 | `msg_service/process_msg_service.rs` | L32-33 | 读 `.get()` | `.read().await.get(k)` |
| 9 | `msg_service/process_msg_service.rs` | L130-131 | 读 `.get()` | `.read().await.get(k)` |

### 2.3 涉及文件

- `crates/quic_service/src/lib.rs` — 全局变量定义
- `crates/quic_service/src/quic_server.rs` — 连接注册、下线清理
- `crates/quic_service/src/msg_service/mod.rs` — 按 uuid 查找发送流
- `crates/quic_service/src/msg_service/send_msg.rs` — 系统消息发送
- `crates/quic_service/src/msg_service/process_msg_service.rs` — 消息路由投递
- `crates/quic_service/Cargo.toml` — 新增 dashmap 依赖

---

## 3. 迁移方案

### 3.1 变更定义（lib.rs）

```rust
// 改造前
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

lazy_static! {
    pub static ref GLOBAL_QUIC_SERVER_LIST: Arc<RwLock<HashMap<String, QuicConnection>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

// 改造后
use std::sync::Arc;
use dashmap::DashMap;

lazy_static! {
    pub static ref GLOBAL_QUIC_SERVER_LIST: Arc<DashMap<String, QuicConnection>> =
        Arc::new(DashMap::new());
}
```

### 3.2 全局写操作迁移

| 操作 | 改造前 | 改造后 |
|------|--------|--------|
| insert | `list.write().await.insert(k, v)` | `list.insert(k, v)` |
| remove | `list.write().await.remove(k)` | `list.remove(k)` |
| get_mut | `list.write().await.get_mut(k)` | `list.get_mut(k)` |

### 3.3 全局读操作迁移

| 操作 | 改造前 | 改造后 |
|------|--------|--------|
| get | `list.read().await.get(k)` | `list.get(k)` |
| len | `list.read().await.len()` | `list.len()` |

### 3.4 临界区消除（可选优化）

`end_server()` 中的 `get_mut` + `remove` 之间存在隐式临界区（先读后写，依赖 `update_time` 判断）：

```rust
// 改造前 — 在 write() 锁的保护下连贯执行
let mut server_book = GLOBAL_QUIC_SERVER_LIST.write().await;
if let Some(book) = server_book.get_mut(close_key) {
    let now = book.update_time;
    if now == close_now as u64 {
        uuid = book.uuid.clone();
        server_book.remove(close_key);
        // ...
    }
}

// 改造后 — DashMap 的 get_mut 持有 shard 级别的锁，remove 会再次获取锁
// 需要将 get_mut 和 remove 合并为单一原子操作
if let Some(mut book) = GLOBAL_QUIC_SERVER_LIST.get_mut(close_key) {
    if book.update_time == close_now as u64 {
        uuid = book.uuid.clone();
        drop(book);  // 主动释放锁再执行 remove
        GLOBAL_QUIC_SERVER_LIST.remove(close_key);
        // ...
    }
}
```

**不迁移 `QuicConnection` 内部的 `send_stream`**：`QuicConnection.send_stream` 已经是 `Arc<RwLock<SendStream>>`，该锁不在本计划范围内。

---

## 4. 逐文件变更清单

### 4.1 `crates/quic_service/Cargo.toml`

在 `[dependencies]` 下新增一行：

```toml
dashmap = "6.1"
```

### 4.2 `crates/quic_service/src/lib.rs`

- 移除：`use std::collections::HashMap;`
- 移除：`use tokio::sync::RwLock;`
- 新增：`use dashmap::DashMap;`
- 修改 `GLOBAL_QUIC_SERVER_LIST` 类型和初始化

### 4.3 `crates/quic_service/src/quic_server.rs`

移除 `tokio::sync::RwLock` 导入（如果不再使用，保留其他用途的 `Arc<RwLock<SendStream>>` 导入）：

| 行号 | 改动 |
|------|------|
| L162 | `GLOBAL_QUIC_SERVER_LIST.read().await.len()` → `GLOBAL_QUIC_SERVER_LIST.len()` |
| L191 | `GLOBAL_QUIC_SERVER_LIST.write().await.insert(...)` → `GLOBAL_QUIC_SERVER_LIST.insert(...)` |
| L202 | `GLOBAL_QUIC_SERVER_LIST.read().await.len()` → `GLOBAL_QUIC_SERVER_LIST.len()` |
| L293-299 | 见 §3.4 临界区改造 |
| L312 | `GLOBAL_QUIC_SERVER_LIST.read().await.len()` → `GLOBAL_QUIC_SERVER_LIST.len()` |

### 4.4 `crates/quic_service/src/msg_service/mod.rs`

| 行号 | 改动 |
|------|------|
| L5 | 移除 `use tokio::sync::RwLock;`（若不再使用 `RwLock`） |
| L20 | `GLOBAL_QUIC_SERVER_LIST.read().await.get(&connection_key)` → `GLOBAL_QUIC_SERVER_LIST.get(&connection_key)` |

### 4.5 `crates/quic_service/src/msg_service/send_msg.rs`

| 行号 | 改动 |
|------|------|
| L24 | `GLOBAL_QUIC_SERVER_LIST.read().await.get(&user_key)` → `GLOBAL_QUIC_SERVER_LIST.get(&user_key)` |

### 4.6 `crates/quic_service/src/msg_service/process_msg_service.rs`

| 行号 | 改动 |
|------|------|
| L32 | `GLOBAL_QUIC_SERVER_LIST.read().await.get(connection_key)` → `GLOBAL_QUIC_SERVER_LIST.get(connection_key)` |
| L130 | `GLOBAL_QUIC_SERVER_LIST.read().await.get(&user_key)` → `GLOBAL_QUIC_SERVER_LIST.get(&user_key)` |

---

## 5. 注意事项

### 5.1 `RwLock` 导入的保留

`quic_server.rs` 和 `msg_service/mod.rs` 中 `tokio::sync::RwLock` 还被用于 `Arc<RwLock<SendStream>>`，**不要移除**。

### 5.2 返回值差异

`DashMap::get()` 返回 `Option<dashmap::mapref::one::Ref<K, V>>`，而 `RwLock<HashMap>::get()` 返回 `Option<&V>`。两者都实现了 `Deref`，所以 `send.send_stream.clone()` 可以直接工作，无需额外改动。

### 5.3 并发语义

- `DashMap` 的 `len()` 返回的是近似值（非精确值），但在当前场景（日志统计）下完全可以接受
- `DashMap::get_mut()` 返回 `Option<dashmap::mapref::one::RefMut<K, V>>`，持有该 shard 的写锁，使用后应 `drop()` 再调用 `remove()` 避免死锁

### 5.4 无需变更的文件

| 文件 | 原因 |
|------|------|
| `docs/集群路由方案.md` | 文档中的伪代码仅为架构描述，非实际代码 |
| `docs/QUIC_SERVICE.md` | 由 `lib.rs` 自动生成/同步即可 |
| `src/main.rs` | 不直接操作 `GLOBAL_QUIC_SERVER_LIST` |

---

## 6. 实施步骤

1. `quic_service/Cargo.toml` — 新增 `dashmap = "6.1"`
2. `quic_service/src/lib.rs` — 修改类型定义和导入
3. `quic_service/src/quic_server.rs` — 替换 5 处读写锁调用
4. `quic_service/src/msg_service/mod.rs` — 替换 1 处读锁调用
5. `quic_service/src/msg_service/send_msg.rs` — 替换 1 处读锁调用
6. `quic_service/src/msg_service/process_msg_service.rs` — 替换 2 处读锁调用
7. 执行 `cargo check` 确认编译通过
8. 执行 `cargo test` 确认功能正常
