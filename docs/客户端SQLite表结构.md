# 客户端 SQLite 表结构总览

> 仓库：`only-talk-app/src-tauri/src/entity/`（每表一个 `SqliteStore` 实现：`create_table` 建表 + `update_table` ALTER 补列，启动时依次执行 —— **表终态 = 两者叠加**）
> 整理于 2026-09-17（服务端任务 01–12、客户端任务 12 已实施后）
> 标注：**★ = 任务12 新增/变更**（**已实施**，见 `任务12-同意式离线同步.md` §4.5/§4.6/§9.2）；其余均为已交付形态
> 心智模型：Kafka 消费者 —— 会话 = topic、设备 = 消费者组、消费水位本地维护（任务12 §2.1）；架构不变式：QUIC 只管在线实时投递，离线补齐永远 HTTP 拉取

共 **17 张已交付表 + 2 张任务12 新增表**，按域划分：

| 域 | 表 | 职责一句话 |
|----|----|-----------|
| 会话域 | `chat_session` | 会话列表（展示属性 + 会话事实） |
| 消息域 | `chat_record` / `group_chat_record` | 单聊 / 群消息本体（含 server_id） |
| 同步域 | `session_sync_state`★ / `sync_task`★ / `chat_record_read` / `group_message_read` | 拉取水位与批次记录 + 已读上报事件 |
| 发送域 | `chat_record_send` / `chat_record_ack` / `group_message_ack` | 出站队列、ACK 跟踪、重试 |
| 资料域 | `user_info` / `friend` / `group_info` / `group_member` | 用户 / 好友 / 群 / 群成员资料缓存 |
| 功能域 | `system_notification` / `user_token` / `file_record` / `webrtc_signal` / `app_log` | 通知 / 凭据 / 文件 / RTC 信令 / 日志 |

---

## 1. 会话域

### `chat_session` —— 会话列表（本地 UI 主数据源）

```sql
CREATE TABLE IF NOT EXISTS chat_session (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    nano_id          TEXT NOT NULL,               -- 最新一条消息 nano_id(展示冗余)
    timestamp        INTEGER NOT NULL,            -- 最新一条消息时间
    send_user        TEXT NOT NULL,               -- 单聊:对方(to_canonical 归一化) / 群:群id
    recv_user        TEXT NOT NULL,               -- 单聊:我 / 群:群id
    text_type        INTEGER NOT NULL DEFAULT 0,
    unread_count     INTEGER NOT NULL DEFAULT 0,  -- 本地计数的角标(与服务端 unread 合并)
    last_message     TEXT NOT NULL,               -- 列表摘要
    is_show          INTEGER NOT NULL DEFAULT 1,
    is_top           INTEGER NOT NULL DEFAULT 0,
    session_type     INTEGER NOT NULL DEFAULT 0,  -- 1-单聊 2-群聊 3-系统 4-公众号
    session_uuid     TEXT DEFAULT NULL,           -- 任务01: 单聊 v5 派生 / 群=group_id
    synced_id        INTEGER NOT NULL DEFAULT 0,  -- 任务01 加; ★任务12 迁出 → session_sync_state
    group_id         TEXT DEFAULT NULL,           -- 仅 update_table ALTER 补列(历史加列)
    last_message_id  INTEGER NOT NULL DEFAULT 0,  -- ★任务12 新增: 会话事实(/session/list), 缺口检测输入
    UNIQUE(send_user, recv_user),
    UNIQUE(session_uuid)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_chat_session_su ON chat_session(session_uuid);
```

**★任务12 变更**：加 `last_message_id`（会话事实 —— 它是会话的属性）；**删 `synced_id`**（执行位置，迁入同步域水位表）。变更后该表纯会话域：只管展示与事实，不再持任何执行状态。

---

## 2. 消息域

### `chat_record` —— 单聊消息

```sql
CREATE TABLE IF NOT EXISTS chat_record (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    nano_id    TEXT NOT NULL UNIQUE,     -- 去重键(宁重勿漏的客户端侧)
    raw        TEXT NOT NULL,            -- 消息内容
    timestamp  INTEGER NOT NULL,
    send_user  TEXT NOT NULL,
    recv_user  TEXT NOT NULL,
    text_type  INTEGER NOT NULL DEFAULT 0,
    server_id  INTEGER DEFAULT NULL      -- 任务07: 服务端消息 id(offset); QUIC 在线消息 NULL
);
```

> 定位方式：消息按 `(send_user, recv_user)` 对或 `nano_id` 定位，**不带 `session_uuid`**（已交付形态，任务12 不变）。`server_id` 是消息的属性（Kafka 术语：offset），被已读上报消费（跨域读，取会话内 max）。

### `group_chat_record` —— 群消息

```sql
CREATE TABLE IF NOT EXISTS group_chat_record (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    nano_id    TEXT NOT NULL UNIQUE,
    raw        TEXT NOT NULL,
    timestamp  INTEGER NOT NULL,
    send_user  TEXT NOT NULL,
    group_id   TEXT NOT NULL,            -- 群定位键(等价于会话的 session_uuid)
    text_type  INTEGER NOT NULL DEFAULT 0,
    server_id  INTEGER DEFAULT NULL      -- 任务07, 同上
);
```

---

## 3. 同步域

### `session_sync_state` ★任务12 新增 —— 拉取水位（每会话一行，持久）

```sql
CREATE TABLE IF NOT EXISTS session_sync_state (
    session_uuid TEXT PRIMARY KEY,
    synced_id    INTEGER NOT NULL DEFAULT 0,  -- 前沿: 已回报的最大服务端 id(Kafka: 已提交位移)
    hist_floor   INTEGER DEFAULT NULL,        -- 连续前沿: 一切 id ≥ floor 的消息都在本地(seek 回放记账)
    backfill     INTEGER NOT NULL DEFAULT 0,  -- 0-未回填过 1-完成 2-跳过(「不再提示」) —— 提示抑制持久位
    updated_at   INTEGER NOT NULL
);
```

### `sync_task` ★任务12 新增 —— 批次执行记录（批次 × 会话，append）

```sql
CREATE TABLE IF NOT EXISTS sync_task (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    batch_id     INTEGER NOT NULL,            -- 批次 = 触发时刻毫秒: 一次重连/同意 = 一批
    session_uuid TEXT NOT NULL,
    kind         INTEGER NOT NULL,            -- 0-静默补拉(重连轮) 1-回填
    status       INTEGER NOT NULL DEFAULT 0,  -- 0-待执行 1-执行中 2-成功 3-失败
    batches      INTEGER NOT NULL DEFAULT 0,  -- 本任务已消化拉取批次数
    new_count    INTEGER NOT NULL DEFAULT 0,  -- 本任务累计新增消息数
    attempt      INTEGER NOT NULL DEFAULT 0,  -- 重试次数
    last_error   TEXT DEFAULT NULL,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sync_task_batch   ON sync_task(batch_id);
CREATE INDEX IF NOT EXISTS idx_sync_task_pending ON sync_task(kind, status);
```

> 保留策略：最近 50 批（仅全终态批可清），pending/running 永不清（队列本体）。批的成败视图 = `GROUP BY batch_id` 派生，不存计数。

### `chat_record_read` —— 单聊已读上报事件（每会话一行水位）

```sql
CREATE TABLE IF NOT EXISTS chat_record_read (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    nano_id    TEXT NOT NULL,      -- 最后已读消息
    timestamp  INTEGER NOT NULL,   -- 水位: 10s 读上报循环从此增量取事件
    send_user  TEXT NOT NULL,
    recv_user  TEXT NOT NULL,
    UNIQUE(send_user, recv_user)
);
```

### `group_message_read` —— 群已读上报事件

```sql
CREATE TABLE IF NOT EXISTS group_message_read (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    nano_id     TEXT NOT NULL,
    group_uuid  TEXT NOT NULL,
    user_uuid   TEXT NOT NULL,
    timestamp   INTEGER NOT NULL,  -- 水位, 同上(群侧)
    UNIQUE(group_uuid, user_uuid)
);
```

> 这两张是同步域的**上报侧**：10 秒循环取水位之后的新阅读事件 → 聚合为 `/session/read`（报每会话 max(server_id)），成功后推进水位（任务07 §6）。

---

## 4. 发送域

### `chat_record_send` —— 单聊出站队列（发送/重试）

```sql
CREATE TABLE IF NOT EXISTS chat_record_send (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    send_id      TEXT NOT NULL,               -- 发送批次 id
    msg_id       TEXT NOT NULL,               -- 即消息 nano_id
    platform     INTEGER NOT NULL,
    text_type    INTEGER NOT NULL DEFAULT 0,
    recv_user    TEXT NOT NULL,
    send_user    TEXT NOT NULL,
    raw          TEXT NOT NULL,
    timestamp    INTEGER NOT NULL,
    send_status  INTEGER NOT NULL DEFAULT 0,  -- 0-发送中 1-成功(收到 ACK) 2-失败
    retry_count  INTEGER NOT NULL DEFAULT 0
);
```

### `chat_record_ack` —— 单聊 ACK 跟踪（多端回执去重）

```sql
CREATE TABLE IF NOT EXISTS chat_record_ack (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    msg_id      TEXT NOT NULL,       -- 对应 chat_record_send.msg_id
    prev_id     TEXT NOT NULL,       -- 协议链: 上一条 ACK 的 id(多端去重/排序)
    send_id     TEXT NOT NULL,
    platform    INTEGER NOT NULL,    -- ACK 来源平台(自己另一台设备回推的 ACK 也会落此)
    ack_status  INTEGER NOT NULL DEFAULT 0,
    recv_user   TEXT NOT NULL,
    send_user   TEXT NOT NULL,
    timestamp   INTEGER NOT NULL
);
```

### `group_message_ack` —— 群 ACK 跟踪

```sql
CREATE TABLE IF NOT EXISTS group_message_ack (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    nano_id        TEXT NOT NULL UNIQUE,   -- 服务端消息 id
    local_nano_id  TEXT NOT NULL,          -- 本地发送对应 id(与 nano_id 关联)
    group_uuid     TEXT NOT NULL,
    send_user      TEXT NOT NULL,
    text_type      INTEGER NOT NULL DEFAULT 0,
    ack_status     INTEGER NOT NULL DEFAULT 0,
    raw            TEXT NOT NULL,
    timestamp      INTEGER NOT NULL
);
```

---

## 5. 资料域

### `user_info` —— 用户资料缓存（含自己）

```sql
CREATE TABLE IF NOT EXISTS user_info (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid        TEXT NOT NULL UNIQUE,
    username    TEXT, account TEXT, icon TEXT,
    gender      INTEGER, age INTEGER, birthday INTEGER,
    info        TEXT, phone TEXT, email TEXT, address TEXT,
    status      INTEGER, user_type INTEGER,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
```

### `friend` —— 好友资料 + 本地关系标志（每好友一行）

```sql
CREATE TABLE IF NOT EXISTS friend (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL,
    friend_id        TEXT NOT NULL,
    friend_account   TEXT NOT NULL,
    friend_name      TEXT NOT NULL,
    friend_icon      TEXT NOT NULL,
    friend_info      TEXT NOT NULL,
    friend_user_type INTEGER,
    friend_status    INTEGER NOT NULL DEFAULT 0,
    me               TEXT NOT NULL,            -- 归属本账号(多账号可共存)
    is_del           INTEGER NOT NULL DEFAULT 0,
    is_block         INTEGER NOT NULL DEFAULT 0,
    is_mute          INTEGER NOT NULL DEFAULT 0,
    is_top           INTEGER NOT NULL DEFAULT 0,
    is_show          INTEGER NOT NULL DEFAULT 1,
    version          INTEGER NOT NULL DEFAULT 0,
    UNIQUE(friend_id, me)
);
```

### `group_info`（`entity/group.rs`）—— 群资料

```sql
CREATE TABLE IF NOT EXISTS group_info (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id     TEXT NOT NULL UNIQUE,
    group_name   TEXT NOT NULL,
    group_icon   TEXT NOT NULL DEFAULT '',
    owner_id     TEXT NOT NULL,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    member_count INTEGER NOT NULL DEFAULT 0,
    is_del       INTEGER NOT NULL DEFAULT 0,
    is_show      INTEGER NOT NULL DEFAULT 1,
    version      INTEGER NOT NULL DEFAULT 0
);
```

### `group_member` —— 群成员

```sql
CREATE TABLE IF NOT EXISTS group_member (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id   TEXT NOT NULL,
    user_id    TEXT NOT NULL,
    role       INTEGER NOT NULL DEFAULT 0,   -- 群内角色
    nickname   TEXT NOT NULL DEFAULT '',     -- 群昵称
    joined_at  INTEGER NOT NULL,
    is_del     INTEGER NOT NULL DEFAULT 0,
    UNIQUE(group_id, user_id)
);
```

---

## 6. 功能域

### `system_notification` —— 系统通知（自带已读/已同步水位）

```sql
CREATE TABLE IF NOT EXISTS system_notification (
    id           TEXT PRIMARY KEY,            -- 通知 uuid
    title        TEXT, content TEXT,
    created_at   INTEGER,
    content_type INTEGER,
    user_id      TEXT,
    biz_id       TEXT,
    is_read      INTEGER,
    level1       INTEGER, level2 INTEGER, level3 INTEGER, level4 INTEGER,
    unread_count INTEGER,
    priority     INTEGER NOT NULL DEFAULT 0,
    is_synced    INTEGER NOT NULL DEFAULT 0   -- 已读回传水位(query_read_not_synced_ids 消费)
);
```

### `user_token` —— 凭据托管

```sql
CREATE TABLE IF NOT EXISTS user_token (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id        TEXT UNIQUE,
    refresh_token  TEXT,
    local_credit   TEXT,                      -- 本机设备指纹(SHA-256, device_info 本地生成; 登录/刷新时随 refresh_token 落库, 标记凭据归属设备; 当前只写不读)
    created_at     INTEGER, updated_at INTEGER,
    version        INTEGER DEFAULT 0
);
```

### `file_record` —— 文件消息的本地文件账本

```sql
CREATE TABLE IF NOT EXISTS file_record (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    biz_id               TEXT, uuid TEXT,
    file_name            TEXT, file_path TEXT,
    file_size            INTEGER, mime_type TEXT, file_hash TEXT,
    status               INTEGER DEFAULT 0,
    download_retry_count INTEGER DEFAULT 0,
    created_at           INTEGER, updated_at INTEGER
);
```

### `webrtc_signal` —— RTC 信令暂存

```sql
CREATE TABLE IF NOT EXISTS webrtc_signal (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    nano_id    TEXT NOT NULL,
    session_id TEXT NOT NULL,     -- RTC 会话
    msg_type   TEXT NOT NULL,     -- offer/answer/candidate...
    send_user  TEXT NOT NULL, recv_user TEXT NOT NULL,
    data       TEXT NOT NULL,
    timestamp  INTEGER NOT NULL
);
```

### `app_log` —— 应用日志（ QUIC 帧级调试留痕）

```sql
CREATE TABLE IF NOT EXISTS app_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    log_type    TEXT NOT NULL DEFAULT 'QUIC',
    level       INTEGER NOT NULL DEFAULT 1,
    source      TEXT NOT NULL DEFAULT '',
    raw         TEXT NOT NULL,
    remote_addr TEXT NOT NULL DEFAULT '',
    detail      TEXT NOT NULL DEFAULT '',
    created_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_app_log_created_at ON app_log(created_at);
```

---

## 7. 任务12 迁移清单（无包袱直迁）

| # | 动作 | 语句 |
|---|------|------|
| 1 | 新建 `session_sync_state` / `sync_task` | 各自 `CREATE TABLE IF NOT EXISTS`（`create_table`） |
| 2 | 位置搬运 | `INSERT INTO session_sync_state(session_uuid, synced_id, updated_at) SELECT session_uuid, synced_id, <now> FROM chat_session WHERE synced_id > 0` |
| 3 | 删列 | `ALTER TABLE chat_session DROP COLUMN synced_id`（SQLite 3.35+，Tauri 自带版本满足） |
| 4 | 加列 | `ALTER TABLE chat_session ADD COLUMN last_message_id INTEGER NOT NULL DEFAULT 0` |
| 5 | 摘除代码引用 | `ChatSession` 结构体 / VO / `set_session_sync_cursor`（改写为水位表 DAO）/ 各构造点 —— rg 复核 `synced_id` 在会话域零残留 |

## 8. 设计规约（维护时先读）

1. **三域分离**：会话表只管会话、消息表只管消息、同步表管任务执行；位置类字段一律住同步域。
2. **唯一有意跨域读 = 缺口检测**：`chat_session.last_message_id`（会话事实）vs `session_sync_state.synced_id`（执行位置）。新增第二处跨域读前先读 `任务12` §4.6。
3. **`server_id` 允许 NULL**：QUIC 在线消息不带 offset（架构不变式：QUIC 只管在线实时，离线补齐永远 HTTP 拉取）—— NULL 由拉取重放补齐，勿在写入路径强行填充。
4. **append 类表必须有界**：`sync_task` 保留 50 批；`app_log` 无清理（已知遗留，量大时可按 created_at 滚动清理）。
5. 表结构演进走 `update_table` 的幂等 ALTER（忽略已存在错误），与既有惯例一致。
