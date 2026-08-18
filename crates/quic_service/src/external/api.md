# 外网 QUIC 连接详解（客户端 → 服务器）

本文档基于 `crates/quic_service/src/external/` 与 `crates/common/src/utils/text_msg.rs` 的实际代码，描述**外网（公网）QUIC 连接**的完整链路：连接建立、首包认证、帧协议、消息收发、心跳与下线清理，以及涉及的所有类型与 struct。

---

## 1. 总览

| 项 | 说明 |
| --- | --- |
| 端口 | `4433`（默认，配置项 `[quic_server].address`） |
| 协议 | QUIC（quinn crate，基于 rustls） |
| 传输安全 | TLS 1.3，证书路径 `[quic_server].cert_path` / `key_path`（默认 `./config/ssl/fullchain.pem` / `privkey.pem`） |
| 应用层认证 | **首包 JWT**：连接建立后客户端必须发送一条 JSON 首包（`FirstQuicMsg`），服务器校验 token 与 uuid 一致后才注册连接 |
| 双向流（bidi） | 仅用于**初始化握手**（发送首包 + 保持存活）；后续普通消息通过**按需单向流（uni stream）**发送 |
| 单向流（uni） | 客户端→服务器：发送消息（`open_uni`）；服务器→客户端：推送消息 / ACK / PONG（`open_uni`） |
| 消息编码 | `HeadMsg`（bincode，固定 9 字节）+ `TextQuicMsg`（bincode），CRC-16/X25 校验 |
| 粘包/半包 | 每连接一个 `Arc<Mutex<Vec<u8>>>` 残包缓冲区，跨多次 `read` 拼接恢复 |

相关文件：

```
crates/quic_service/src/external/
├── quic_server.rs    # 服务器：接受连接、首包认证、连接注册、收流/下流循环
├── quic_client.rs    # 测试用客户端（演示握手、发消息、心跳）
├── set_server.rs     # TLS 端点/配置构建（服务端 + 客户端）
├── tls_monitor.rs    # 证书文件监听 + 热重载 + 到期告警
├── chat_node.rs      # ChatNode：生命周期管理（init/start/stop）
├── lifecycle.rs      # ServiceLifecycle trait
├── state.rs          # ServiceState / ServiceError
├── config.rs         # ChatNodeConfig（TOML 解析）
└── mod.rs            # 模块导出
crates/quic_service/src/models/
├── first_quic_msg.rs # FirstQuicMsg（首包）
├── quic_connection.rs# ConnectionType / QuicConnection
└── text_msg.rs       # 重新导出 common::utils::text_msg
crates/quic_service/src/msg_service/
├── text_msg_service.rs     # 帧解析（get_text_msg：粘包/半包处理）
├── process_msg_service.rs  # 收到消息后的处理与路由（含转发到内网）
└── send_msg.rs             # 服务器主动向用户推送（系统消息）
crates/common/src/utils/text_msg.rs  # HeadMsg / TextQuicMsg / X25 / 消息构造函数（标准位置）
```

---

## 2. 连接流程

### 2.1 服务器启动

```
ChatNode::init()  ──►  make_server_endpoint()  创建 TLS 端点（绑定 0.0.0.0:4433）
ChatNode::start() ──►  创建 watch 关闭通道
                      ├── start_tls_monitor()  证书热重载 + 到期告警
                      └── run_server()         接受连接循环（后台任务）
```

- `ChatNode::start()`（`chat_node.rs:85`）将 endpoint 包进 `Arc`，创建 `watch::channel(false)` 作为关闭信号，随后：
  1. `start_tls_monitor(...)` —— 独立任务，见 §6；
  2. `tokio::spawn(run_server(endpoint, connections, config, core, shutdown_rx))` —— 接受循环。

- `run_server()`（`quic_server.rs:29`）主循环：

```
tokio::select! {
    shutdown_rx.changed()  ──► 收到关闭信号，return（停止接受新连接）
    endpoint.accept()      ──► 有新连接 incoming_conn
}
incoming_conn.await ──► handle_connection()（spawn，每连接一个任务）
```

- `handle_connection()`（`quic_server.rs:77`）在 `accept_bi()` 循环中等待双向流；**每来一个双向流 spawn 一个 `handle_conn` 任务**，允许同一 QUIC 连接上开多个流。

### 2.2 客户端连接（握手）

客户端流程（见 `quic_client.rs`，可作为接入参考）：

```
Endpoint::client("0.0.0.0:0")           建立本地端点
  └── set_default_client_config(configure_client())
        · 信任 webpki_roots 系统根证书（CA 签名证书可直接验证）
        · idle_timeout = 190s，max_concurrent_uni_streams = 32
endpoint.connect(server_addr, "onlytalk.cn")   ← SNI 必须是证书域名
  └── open_bi()                           打开双向流
        └── write_all(FirstQuicMsg JSON)  发送首包（认证）
        └── 保持 send_stream 存活（spawn pending future，防止服务器判离线）
```

> ⚠️ **SNI 校验**：`configure_client()` 使用 `webpki_roots`（系统根证书）校验服务器证书，`connect()` 的第二参数是 SNI 域名，必须与服务器证书 CN/SAN 匹配，否则 TLS 握手失败。

### 2.3 服务器握手流程（`handle_conn`，`quic_server.rs:237`）

```
handle_conn(send_stream, recv_stream, conn, address, ...)
  │
  ├─ 1. process_first_msg()            读取首包（100KB 缓冲区，JSON 解析为 FirstQuicMsg）
  │      ├─ 解析失败 ──► send_stream.finish() + 断开
  │      └─ 客户端未发首包就关流 ──► 断开
  │
  ├─ 2. authenticate_connection()      首包 JWT 认证（见 §2.4）
  │      └─ verify_token(token) → claims.uuid == first_quic_msg.uuid 才通过
  │
  ├─ 3. 提取 platform = claims.sub（"PC" / "MOBILE"）
  │     提取 uuid    = claims.uuid
  │
  ├─ 4. verify_max_client()            当前 DashMap 连接数 > max_connections 则拒绝
  │
  ├─ 5. user_online(uuid, platform)    TODO 占位（暂只打日志）
  │
  ├─ 6. 构造连接 key 并 set_conn_info()（见 §2.5）
  │
  ├─ 7. spawn 单向流接收循环           处理客户端 open_uni 发来的消息
  │      （每连接一个 buffer_msg 残包缓冲）
  │
  ├─ 8. 双向流接收循环                 处理该流上收到的消息（process_rec_msg）
  │      （半包残包同样缓冲，跨 read 拼接）
  │      · 流关闭（Ok(None) / 读错误 / 缓冲超限）──► 退出循环
  │
  └─ 9. uni_shutdown.store(true)       通知单向流循环退出
        end_server()                   下线清理（见 §2.6）
```

> **双接收循环的意义**：服务器在双向流上只收首包和（兼容性的）消息，但把消息收发的**主力通道放在单向流**上；双向流只要不关，连接就算"存活"（`quic_client.rs:132` 特意保持 send_stream 不 drop 即为此原因）。

### 2.4 首包 JWT 认证（`authenticate_connection`，`quic_server.rs:162`）

```rust
let claims = verify_token(first_quic_msg.token.as_ref())
    .map_err(|_| "Failed to parse token")?;
if claims.uuid != first_quic_msg.uuid {
    // token 与账号不匹配 → finish 流并断开
}
```

- token 由 `common::utils::jwt_util::generate_access_token(uuid, platform)` 签发（RSA 签名，进程级缓存 `EncodingKey`/`DecodingKey`）。
- **校验规则**：JWT 签名有效 **且** `claims.uuid == 首包中的 uuid`，两者缺一不可。
- `claims.sub` 是平台标识（`PC` / `MOBILE`），会被用作连接 key 的前缀。

### 2.5 连接 key 与连接注册（`set_conn_info`，`quic_server.rs:202`）

**key 格式**（全部转大写）：

```
{platform}:QUIC:SERVER:{uuid}:{消息类型}
```

例如：`PC:QUIC:SERVER:01965D95-0FFC-7D23-911E-1111485FB9BE:TEXT`

- `platform` 来自 `claims.sub`（`PC` / `MOBILE`）；
- `消息类型` 来自首包的 `msg_type`（`ConnectionType`，Display 为 `text`/`img`/`video`/`file`/`other`）。

注册动作：

| 存储 | 内容 | 生命周期 |
| --- | --- | --- |
| 内存 `connections: DashMap<String, QuicConnection>` | key → `QuicConnection`（含 `quinn::Connection` 句柄） | 连接存活期间 |
| Redis `set_ex(key, server_index, 7200)` | key → 本节点 `server_index`（用于跨节点寻址） | TTL 7200 秒，**下线时主动 DEL** |

`QuicConnection` 结构见 §4.3。

### 2.6 下线清理（`end_server`，`quic_server.rs:384`）

1. 从 DashMap 取连接，**校验 `update_time == close_now`**（防止"同一 key 的新连接刚注册，旧连接才退出"误删新连接）；
2. `connections.remove(key)`；
3. Redis `DEL {connection_key}`；
4. `user_offline(core, uuid)`：把 Redis 中缓存的该用户已读消息（`USER:READ:MSG:{uuid}`）持久化到数据库：
   - 群聊已读（`chat_type == CHAT_TYPE_GROUP`）：校验群消息存在、读者是群成员、游标只推进不回退，更新 `group_member.last_read_msg_id`；
   - 单聊已读：校验 `chat_message_record` 中收发双方与记录匹配，`chat_message_record_read` 表 upsert。

---

## 3. 帧协议

### 3.1 帧格式

每一条消息 = **HeadMsg（bincode，固定 9 字节）** + **TextQuicMsg（bincode）**：

```
┌─────────────────────────────┬──────────────────────────────┐
│ HeadMsg (9 bytes, bincode)  │ TextQuicMsg (bincode body)   │
│ version: u8   = 1           │ nano_id: String              │
│ crc: u16      = X25(body)   │ text_type: u16               │
│ body_len: u32 = body 长度    │ raw: Vec<u8>                 │
│ message_type: u16           │ recv_user: String            │
└─────────────────────────────┴ recv_user: String 等 6 字段   ┘
```

- **HeadMsg 定长 9 字节**：`1(u8) + 2(u16) + 4(u32) + 2(u16)`，客户端硬编码 `head_length = 9`（`quic_client.rs:53`）。
- `crc` 是对 **body 的二进制**计算 CRC-16/X25（`crc::CRC_16_IBM_SDLC`）。
- `body_len` 是 body 的字节长度，用于从流中切分消息。
- `message_type` 与 `TextQuicMsg.text_type` 一致（单聊 TEXT=1、PING=99、群聊 2001-2004、ACK 201/2201 等，见 §5.4）。

### 3.2 粘包/半包处理（`get_text_msg`，`text_msg_service.rs:13`）

每个连接（客户端侧每平台）持有一个残包缓冲 `buffer_msg: Arc<Mutex<Vec<u8>>>`：

```
每次 read 到数据后：
  1. 若 buffer_msg 非空 → 与本次数据合并，清空 buffer_msg
  2. 循环解析：
     a. 剩余数据 < 9 字节         ──► 全部存入 buffer_msg，等下次（半包）
     b. 反序列化 HeadMsg 失败      ──► 存 buffer_msg，返回已解析部分
     c. body 长度超过剩余数据      ──► 存 buffer_msg，等下次（半包）
     d. 反序列化 TextQuicMsg 失败  ──► 存 buffer_msg，返回已解析部分
     e. X25(body) != head.crc     ──► 返回 Err（CRC 校验失败，上层终止）
     f. 成功                       ──► 收入结果集，继续解析下一条（粘包）
```

一个网络包内可以塞多条消息（粘包），一条消息也可以跨多个网络包到达（半包），全部由该机制恢复。

### 3.3 消息构造（`common/src/utils/text_msg.rs`）

| 函数 | 说明 |
| --- | --- |
| `generate_text_msg(text_type, raw, recv_user, send_user)` | 生成帧：nanoid 生成 nano_id，毫秒时间戳，`version=1`，CRC 自动计算 |
| `generate_text_msg_with_id(nano_id, ...)` | 指定 nano_id（重发/ACK 场景） |
| `generate_text_msg_with_time(nano_id, ..., timestamp)` | 指定 nano_id 与时间戳（转发时保留原始消息信息） |
| `build_text_msg(head, body)` | 通用拼接：head 字节 + body 字节 |

---

## 4. 类型与 struct 详解

### 4.1 `FirstQuicMsg` —— 连接首包（`models/first_quic_msg.rs`）

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct FirstQuicMsg {
    pub token: String,             // 用户 token（JWT，由登录接口签发）
    pub uuid: String,              // 用户账号（uuid）
    pub msg_type: ConnectionType,  // 流数据类型：文本、图片、视频、其他
    pub text_serde_struct: String, // 文本类型序列化结构体（客户端协商字段，如 "user_chat_json"）
    pub dyn_buffer_size: usize,    // 缓冲区大小（预留）
    pub dyn_header_size: usize,    // 头部大小（= 9，HeadMsg bincode 长度）
}
```

- 以 **JSON** 形式写在双向流的**第一段数据**中（服务器用 100KB 缓冲区读取）；
- `dyn_header_size` 会被服务器记住（`head_length`），之后所有消息解析都依赖它；
- 服务器验证 `token`（JWT）与 `uuid` 一致，验证通过前**不注册连接、不收消息**。

### 4.2 `ConnectionType` —— 连接/流类型（`models/quic_connection.rs`）

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConnectionType {
    Text,   // 文本（Display: "text"）
    Img,    // 图片
    Video,  // 视频
    File,   // 文件
    Other,  // 其他
}
```

- 用于首包的 `msg_type` 字段，决定连接 key 的后缀；
- 当前代码中除 `Text` 外的分支均未实现（`quic_client.rs:222` 的 `process_rec_msg` 对 Img/Video/File/Other 空操作）。

### 4.3 `QuicConnection` —— 在线连接记录（`models/quic_connection.rs`）

```rust
#[derive(Debug, Clone)]
pub struct QuicConnection {
    pub is_online: bool,
    pub uuid: String,
    pub connection_type: ConnectionType,
    pub conn: Connection,     // quinn::Connection 句柄（发消息、开新流都靠它）
    pub create_time: u64,     // 毫秒时间戳
    pub update_time: u64,     // 毫秒时间戳（下线清理时用于防误删）
    pub ipv4addr: String,     // 客户端 IPv4 地址
    pub ipv6addr: String,     // 客户端 IPv6 地址（当前恒为 ""）
}
```

存放于 `ChatNode.connections: Arc<DashMap<String, QuicConnection>>`，key 见 §2.5。

### 4.4 `ChatNodeConfig` —— 节点配置（`external/config.rs`）

```rust
pub struct ChatNodeConfig {
    pub bind_address: SocketAddr,            // 监听地址（[quic_server].address，如 "0.0.0.0:4433"）
    pub cert_path: String,                   // TLS 证书链（默认 ./config/ssl/fullchain.pem）
    pub key_path: String,                    // TLS 私钥（默认 ./config/ssl/privkey.pem）
    pub max_connections: usize,              // 最大在线连接数（默认 1000）
    pub max_buffer_length: usize,            // 残包缓冲上限（默认 10MB，超限断开）
    pub idle_timeout_secs: u64,              // QUIC 空闲超时（默认 190s，代码未实际注入）
    pub max_concurrent_uni_streams: u8,      // 并发单向流数（默认 0，实际由 set_server.rs 固定 32）
    pub server_name: String,                 // 节点名称（默认 "127.0.0.1:4433"，日志用）
    pub cert_watch_interval_secs: u64,       // 证书文件轮询间隔（默认 60s）
    pub cert_expiry_warning_days: i64,       // 到期告警阈值（默认 3 天）
    pub cert_expiry_check_interval_secs: u64,// 告警日志节流（默认 3600s）
    pub server_index: u32,                   // 本节点集群索引（[cluster].server_index）
    pub node_address: String,                // 本节点外网地址（默认 "127.0.0.1:4433"）
}
```

TOML 对应关系（`config/app_config.toml` 的 `[quic_server]` 段）：

```toml
[cluster]
server_index = 0        # → ChatNodeConfig.server_index

[quic_server]
address   = "0.0.0.0:4433"     # 必填，→ bind_address
cert_path = "./config/ssl/fullchain.pem"   # 可选，有默认值
key_path  = "./config/ssl/privkey.pem"     # 可选
server_name = "127.0.0.1:4433" # 可选
node_address = "127.0.0.1:4433"# 可选
```

其余字段（max_connections 等）**只能走默认值**，TOML 中不读取。

### 4.5 `ServiceState` —— 生命周期状态机（`external/state.rs`）

```rust
pub enum ServiceState { Uninitialized, Initializing, Running, Stopping, Stopped }
```

合法迁移：

```
Uninitialized ──► Initializing ──► Running
                      │
                      └───► Stopping ──► Stopped
```

非法迁移返回 `ServiceError::InvalidStateTransition { from, to }`。

### 4.6 `ServiceError` —— 生命周期错误（`external/state.rs`）

```rust
pub enum ServiceError {
    InvalidStateTransition { from: ServiceState, to: ServiceState },
    Config(String),        // 配置/端点构建错误
    Runtime(anyhow::Error),// 运行时错误（From<anyhow::Error>）
}
```

### 4.7 `ServiceLifecycle` —— 生命周期 trait（`external/lifecycle.rs`）

```rust
#[async_trait]
pub trait ServiceLifecycle: Send + Sync {
    fn name(&self) -> &str;                                    // 服务名
    async fn init(&mut self) -> Result<(), ServiceError>;      // 建端点（状态必须为 Uninitialized）
    async fn start(&self) -> Result<(), ServiceError>;         // 启动后台循环（Initializing/Running）
    async fn stop(&self) -> Result<(), ServiceError>;          // 优雅关闭（Running）
    fn status(&self) -> ServiceState;
}
```

### 4.8 `ChatNode` —— QUIC 节点主体（`external/chat_node.rs`）

```rust
pub struct ChatNode {
    config: ChatNodeConfig,
    core: CoreState,                          // DB + Redis 连接池（common::state）
    state: RwLock<ServiceState>,
    endpoint: RwLock<Option<Endpoint>>,       // 创建后持有，stop 时 close
    connections: Arc<DashMap<String, QuicConnection>>,  // 在线连接表（对外只读访问）
    shutdown_tx: Mutex<Option<watch::Sender<bool>>>,     // 关闭信号
    name: String,
}
```

- `new(config, core)` 创建，状态为 `Uninitialized`；
- `connections()` 返回连接表 `Arc`，供其他服务（如 HTTP 推送）读取在线连接；
- `stop()`：发 watch 信号 → `endpoint.close(0, b"server shutdown")` → 等 100ms → 置 `Stopped`。

### 4.9 `CertStatus` —— 证书状态（`external/tls_monitor.rs`）

```rust
#[derive(Debug, Clone)]
pub struct CertStatus {
    pub not_before: SystemTime,
    pub not_after: SystemTime,
    pub subject: String,
    pub days_remaining: i64,
    pub is_expired: bool,
    pub is_near_expiry: bool,   // days_remaining > 0 且 <= 告警阈值
}
```

### 4.10 `HeadMsg` / `TextQuicMsg` / `TextMsg` / `MessageType` / `X25`（`common/src/utils/text_msg.rs`）

```rust
// CRC-16/X25 计算器（校验 body）
pub const X25: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

// 帧头（bincode 定长 9 字节）
pub struct HeadMsg {
    pub version: u8,        // 版本（恒为 1）
    pub crc: u16,           // X25(body_bytes)
    pub body_len: u32,      // 消息体长度
    pub message_type: u16,  // 消息类型（见 §5.4）
}

// 文本消息体
pub struct TextQuicMsg {
    pub nano_id: String,
    pub text_type: u16,     // 消息类型（与 HeadMsg.message_type 一致）
    pub raw: Vec<u8>,       // 实际内容（文本字节 / 二进制）
    pub recv_user: String,  // 接收用户（群聊时为群 uuid）
    pub send_user: String,  // 发送用户
    pub timestamp: i64,     // 毫秒时间戳
}

// 序列化 trait（HeadMsg / TextQuicMsg 均实现，bincode）
pub trait TextMsg {
    fn get_bytes(&self) -> anyhow::Result<Vec<u8>>;
}
```

`MessageType` 枚举（`#[repr(u16)]`）与常量版 `message_types`（`common/src/utils/message_types.rs`）对应，**代码中实际使用常量**：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `MSG_TYPE_TEXT` | 1 | 单聊文本消息 |
| `MSG_TYPE_IMAGE` | 2 | 图片消息 |
| `MSG_TYPE_FILE` | 3 | 文件消息 |
| `MSG_TYPE_P2P` | 4 | P2P 消息 |
| `MSG_TYPE_P2P_VIDEO_CALL` | 5 | P2P 视频通话 |
| `MSG_TYPE_P2P_VIDEO_DATA` | 6 | P2P 视频数据 |
| `MSG_TYPE_P2P_VIDEO_CONFIG` | 7 | P2P 视频配置 |
| `MSG_TYPE_PING` | 99 | 心跳（客户端→服务器） |
| `MSG_TYPE_RECALL_SUCCESS` | 201 | 单聊消息 ACK（服务器→客户端） |
| `MSG_TYPE_RECALL_FAILURE` | 202 | 消息解析失败通知 |
| `MSG_TYPE_P2P_USER_SERVER` | 203 | 通知作为 P2P 服务端 |
| `MSG_TYPE_P2P_USER_CLIENT` | 204 | 通知作为 P2P 客户端 |
| `MSG_TYPE_SYSTEM` | 10001 | 系统通知 |
| `MSG_TYPE_GROUP_TEXT` | 2001 | 群聊文本消息 |
| `MSG_TYPE_GROUP_IMAGE` | 2002 | 群聊图片消息 |
| `MSG_TYPE_GROUP_FILE` | 2003 | 群聊文件消息 |
| `MSG_TYPE_GROUP_NOTIFICATION` | 2004 | 群聊通知消息 |
| `MSG_TYPE_GROUP_ACK` | 2201 | 群聊消息 ACK（服务器→客户端） |

---

## 5. 消息收发

### 5.1 客户端 → 服务器

两种通道都可用：

1. **按需单向流**（推荐，`send_via_new_stream`，`quic_client.rs:215`）：

```rust
let mut send = conn.open_uni().await?;
send.write_all(data).await?;   // data = HeadMsg + TextQuicMsg 帧
send.finish().await?;
```

2. 初始化双向流（首包之后也可复用，服务器兼容处理）。

服务器侧由两个接收循环（§2.3 步骤 7/8）读取，统一走 `process_rec_msg`。

### 5.2 收到消息后的处理（`process_msg_service.rs`）

```
process_rec_msg() → get_text_msg() 拆帧
  └─ process_text_msg()（逐条）
        ├─ uuid != msg.send_user        ──► 丢弃（防伪造）
        ├─ text_type == PING (99)       ──► 回 PONG（单向流），不落库
        ├─ text_type ∈ {2001..2004}     ──► 群聊：spawn handle_group_msg_from_client()
        │                                    （存库 → 查成员 → 本机投递 + 内网广播）
        │                                    → 回 MSG_TYPE_GROUP_ACK (2201)
        └─ 其他（单聊，如 TEXT=1）       ──► 覆盖 nano_id（服务器重发）+ 时间戳
              ├─ spawn: add_user_chat_record() 存 chat_message_record
              │         + 回 ACK (201)
              └─ send_msg_to_user()     路由给接收方（见下）
```

### 5.3 服务器 → 客户端（推送）

- 本机投递：查 `connections`（DashMap）→ `conn.open_uni()` 发送，**PC 与 MOBILE 两个平台 key 都尝试**；
- 本机没有：封装 `InternalQuicRequest`（payload 即 bincode 帧），按 `compute_preferred_index(recv_user)` 从 Redis `INTERNAL:QUIC:SERVER:{index}` 取目标节点内网地址，走内部 QUIC 转发；
- 心跳 PONG、消息 ACK 同样通过单向流回给发送方。

### 5.4 心跳

- 客户端每 **10 秒**（`quic_client.rs:188`）通过新开的单向流发送 `MSG_TYPE_PING`（raw = "ping"）；
- 服务器收到后回 `MSG_TYPE_PING`（raw = "pong"）；
- 同时服务器侧 `keep_alive_interval = 5s`（`set_server.rs:89`）维持 QUIC 连接活跃。

---

## 6. TLS 证书热重载（`tls_monitor.rs`）

`start_tls_monitor` 启动一个后台任务：

1. 每隔 `cert_watch_interval_secs`（60s）计算证书文件 SHA256，与上次比对；
2. 文件变化 → `load_tls_certificates()` 重读证书链 + 私钥（自动识别 RSA / EC / PKCS8 格式）→ `create_server_config()` → **`endpoint.set_server_config(Some(cfg))` 热替换**，新连接立即使用新证书；
3. 每次轮询解析证书有效期（`x509_parser`），剩余天数 ≤ `cert_expiry_warning_days` 时告警日志（节流 `cert_expiry_check_interval_secs` = 3600s）；
4. 收到 watch 关闭信号后退出。

---

## 7. Redis key 汇总（外网连接相关）

| key（示例） | 类型 | 值 | TTL | 说明 |
| --- | --- | --- | --- | --- |
| `PC:QUIC:SERVER:{uuid}:TEXT` | string | 节点 server_index（如 `"0"`） | 7200s，下线 DEL | **外网用户连接注册**，跨节点寻址用 |
| `MOBILE:QUIC:SERVER:{uuid}:TEXT` | string | 同上 | 同上 | 移动端连接注册 |
| `INTERNAL:QUIC:SERVER:{index}` | string | 节点内网地址（如 `"127.0.0.1:4434"`） | 见集群方案 | 内网转发目标地址 |
| `USER:READ:MSG:{uuid}` | string | JSON 数组（已读消息 DTO） | — | 已读消息缓存，下线时持久化 |

key 统一 **大写**（`to_uppercase()`）。

---

## 8. 备注 / 已知限制

1. **双流并存**：双向流仅承担初始化（首包）与保活，消息收发走按需单向流；双向流若被客户端主动关闭，服务器判定离线。
2. **首包必读**：连接后不发首包（或解析失败）会被直接断开；发首包前消息一律不处理。
3. **`user_online` 为 TODO 空壳**（`quic_server.rs:530`）：上线只打日志，未做锁、缓存同步。
4. **`msg_type` 分支未实现**：`ConnectionType::Img/Video/File/Other` 的收流逻辑为空，实际只有 `Text` 生效。
5. **CRC 校验失败会向上抛错**：`get_text_msg` 对 CRC 不匹配直接 `Err`，上层 `process_rec_msg` 传播错误；而反序列化失败则把数据缓存后正常返回（两种失败行为不一致，代码现状）。
6. **`dyn_header_size` 由客户端告知**：服务器信任首包中的 `head_length`，客户端必须发 `9`，否则拆帧错位。
7. **`max_connections` 含全部类型连接**：`verify_max_client` 统计的是 DashMap 总大小。
8. **`quic_client.rs` 为测试/演示代码**：内置固定 uuid、`#[allow(dead_code)]`，实际客户端应仿照其握手与发流流程实现。
