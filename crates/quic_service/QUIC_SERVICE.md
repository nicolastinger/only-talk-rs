# QUIC Service 文档

## 概述

QUIC Service 是一个基于 QUIC 协议的实时通信服务，为 OnlyTalk 聊天应用提供持久化连接、消息路由、P2P NAT 穿透协调和消息持久化功能。

### 核心特性

- **QUIC 长连接**: 基于 UDP 的低延迟、多路复用连接
- **TLS 加密**: 使用 RS256 算法的 TLS 1.3 加密传输
- **消息路由**: 支持点对点消息、系统消息、心跳消息
- **P2P 协调**: UDP 辅助 NAT 穿透，支持 Full Cone 和 Symmetric NAT
- **消息持久化**: 自动将聊天记录写入数据库
- **跨平台支持**: 同时支持 PC 和 Mobile 客户端

---

## 架构设计

### 模块结构

```
quic_service/
├── src/
│   ├── lib.rs                      # 库入口、全局状态定义
│   ├── init_server.rs              # 服务启动引导
│   ├── set_server.rs               # QUIC 端点配置（TLS、证书、超时）
│   ├── quic_server.rs              # 服务端连接处理与生命周期管理
│   ├── quic_client.rs              # 客户端测试/调试连接逻辑
│   ├── models/
│   │   ├── mod.rs                  # 模块声明
│   │   ├── first_quic_msg.rs       # 初始握手消息结构
│   │   ├── quic_connection.rs      # 连接类型、QuicConnection 结构
│   │   └── text_msg.rs             # 线协议：HeadMsg、TextQuicMsg、MessageType
│   ├── msg_service/
│   │   ├── mod.rs                  # 模块声明、流查找函数
│   │   ├── send_msg.rs             # 系统消息发送工具
│   │   ├── process_msg_service.rs  # 服务端消息处理与路由
│   │   └── text_msg_service.rs     # 消息序列化/反序列化、CRC、组帧
│   └── p2p_service/
│       ├── mod.rs                  # 模块声明
│       ├── model.rs                # P2P 握手结构体
│       └── p2p_udp_service.rs      # 基于 UDP 的 P2P NAT 穿透协调服务
└── Cargo.toml
```

### 全局状态

```rust
// 存储所有活跃的 QUIC 连接，key 格式: "{PLATFORM}:QUIC:SERVER:{UUID}:{CONNECTION_TYPE}"
pub static ref GLOBAL_QUIC_SERVER_LIST: Arc<RwLock<HashMap<String, QuicConnection>>>
```

- 类型: `Arc<RwLock<HashMap<String, QuicConnection>>>`
- 最大连接数: 1000 (`MAX_QUIC_SERVERS`)

---

## 公共 API

### 服务启动

```rust
// quic_service/src/init_server.rs
pub async fn start_server() -> anyhow::Result<()>
```

读取 `./config/app_config.toml` 配置文件，解析 QUIC 服务器地址，启动 QUIC 服务。

### P2P UDP 服务

```rust
// quic_service/src/p2p_service/p2p_udp_service.rs
pub async fn run_udp_server() -> Result<(), anyhow::Error>
```

启动 4 个 UDP 监听器：
- `0.0.0.0:9562` (IPv4)
- `[::]:9563` (IPv6)
- `0.0.0.0:9564` (IPv4)
- `[::]:9565` (IPv6)

### TLS 配置

```rust
// quic_service/src/set_server.rs
pub fn configure_client() -> ClientConfig
pub fn make_server_endpoint(bind_addr: SocketAddr) -> Result<(Endpoint, Vec<u8>), Box<dyn Error>>
pub fn configure_server() -> Result<(ServerConfig, Vec<u8>), Box<dyn Error>>
```

- 证书路径: `./config/ssl/fullchain.pem`
- 私钥路径: `./config/ssl/privkey.pem`
- 支持 RSA、EC、PKCS8 三种密钥格式
- 服务端空闲超时: 190 秒
- 客户端空闲超时: 1800 秒 (30 分钟)

### 消息发送

```rust
// quic_service/src/msg_service/mod.rs
pub async fn get_send_stream_by_uuid(uuid: &String, connection_type: &String) 
    -> Result<Arc<RwLock<SendStream>>, anyhow::Error>

// quic_service/src/msg_service/send_msg.rs
pub async fn send_quic_system_msg(current_user: String, msg_type: u16, text: String) 
    -> anyhow::Result<()>
```

### 消息构建

```rust
// quic_service/src/msg_service/text_msg_service.rs
pub fn generate_text_msg(text_type: u16, raw: Vec<u8>, recv_user: String, send_user: String) 
    -> anyhow::Result<Vec<u8>>

pub fn generate_text_msg_with_id(nano_id: String, text_type: u16, raw: Vec<u8>, 
    recv_user: String, send_user: String) -> anyhow::Result<Vec<u8>>

pub fn generate_text_msg_with_time(nano_id: String, text_type: u16, raw: Vec<u8>, 
    recv_user: String, send_user: String, timestamp: i64) -> anyhow::Result<Vec<u8>>

pub fn build_text_msg<H: TextMsg, G: TextMsg>(text_head: &H, text_msg: &G) 
    -> anyhow::Result<Vec<u8>>
```

### 时间戳工具

```rust
// entity/src/utils/time.rs
pub fn get_now_time_stamp_as_secs() -> Result<i64, io::Error>   // 秒级时间戳
pub fn get_now_time_stamp_as_millis() -> Result<i64, io::Error> // 毫秒级时间戳
```

---

## 数据结构

### 连接类型枚举

```rust
pub enum ConnectionType {
    Text,   // 文本消息
    Img,    // 图片传输
    Video,  // 视频流
    File,   // 文件传输
    Other,  // 其他类型
}
```

### QuicConnection

```rust
pub struct QuicConnection {
    pub is_online: bool,
    pub uuid: String,                      // 用户账号 ID
    pub connection_type: ConnectionType,   // 连接类型
    pub send_stream: Arc<RwLock<SendStream>>, // Quinn 发送流（共享）
    pub create_time: u64,                  // 连接创建时间戳 (ms)
    pub update_time: u64,                  // 最后更新时间戳 (ms)
    pub ipv4addr: String,                  // 客户端 IPv4 地址
    pub ipv6addr: String,                  // 客户端 IPv6 地址
}
```

### 首次握手消息 (FirstQuicMsg)

客户端连接时发送的 JSON 格式握手消息：

```rust
pub struct FirstQuicMsg {
    pub token: String,              // JWT 认证令牌
    pub uuid: String,               // 用户账号 ID
    pub msg_type: ConnectionType,   // 流数据类型
    pub text_serde_struct: String,  // 文本序列化格式标识
    pub dyn_buffer_size: usize,     // 动态缓冲区大小
    pub dyn_header_size: usize,     // 头部大小 (固定为 9)
}
```

### 线协议头部 (HeadMsg)

```rust
pub struct HeadMsg {
    pub version: u8,       // 协议版本 (当前为 1)
    pub crc: u16,          // CRC-16/X25 校验码
    pub body_len: u32,     // 消息体长度 (字节)
    pub message_type: u16, // 消息类型常量
}
```

序列化方式: **bincode**，总头部大小 = 9 字节 (1 + 2 + 4 + 2)

### 文本消息 (TextQuicMsg)

```rust
pub struct TextQuicMsg {
    pub nano_id: String,      // 消息唯一 ID (nanoid)
    pub text_type: u16,       // 消息子类型
    pub raw: Vec<u8>,         // 消息负载字节
    pub recv_user: String,    // 接收者用户 UUID
    pub send_user: String,    // 发送者用户 UUID
    pub timestamp: i64,       // 毫秒级时间戳
}
```

### 消息类型常量

| 常量 | 值 | 含义 |
|------|-----|------|
| `MSG_TYPE_TEXT` | 1 | 普通文本消息 |
| `MSG_TYPE_IMAGE` | 2 | 图片消息 |
| `MSG_TYPE_FILE` | 3 | 文件消息 |
| `MSG_TYPE_P2P` | 4 | P2P 消息 |
| `MSG_TYPE_P2P_VIDEO_CALL` | 5 | P2P 视频通话 |
| `MSG_TYPE_P2P_VIDEO_DATA` | 6 | P2P 视频数据 |
| `MSG_TYPE_P2P_VIDEO_CONFIG` | 7 | P2P 视频配置 |
| `MSG_TYPE_PING` | 99 | 心跳 ping |
| `MSG_TYPE_RECALL_SUCCESS` | 201 | 消息确认 (ACK) |
| `MSG_TYPE_RECALL_FAILURE` | 202 | 消息确认失败 (NACK) |
| `MSG_TYPE_P2P_USER_SERVER` | 203 | P2P: 作为服务端 |
| `MSG_TYPE_P2P_USER_CLIENT` | 204 | P2P: 作为客户端 |
| `MSG_TYPE_SYSTEM` | 10001 | 系统通知 |

### P2P 数据结构

```rust
pub struct P2pInitMsg {
    pub accept_addr: String,     // 接受方地址
    pub request_addr: String,    // 请求方地址
    pub request_uuid: String,    // 请求方 UUID
    pub request_token: String,   // 请求方令牌
    pub accept_uuid: String,     // 接受方 UUID
    pub accept: bool,            // 是否接受
    pub ip_type: u8,             // IP 版本 (4 或 6)
    pub step: u8,                // 协商步骤
    pub is_server: bool,         // 是否作为服务端
}

pub struct UserAddressInfo {
    pub uuid: String,           // 用户 UUID
    pub address: String,        // 观察到的 UDP 地址 (IP:port)
    pub token: String,          // JWT 令牌
    pub ip_type: u8,            // IP 版本
    pub target_uuid: String,    // 目标对等方 UUID
    pub nat_type: u8,           // NAT 类型 (3=Full Cone, 4=Symmetric)
    pub is_server: bool,        // 是否作为 P2P 服务端
    pub lock_uuid: String,      // 分布式锁持有者 ID
    pub is_lock: bool,          // 是否有锁
}
```

---

## 协议流程

### QUIC 连接生命周期

```
                    CLIENT                              SERVER
                      |                                   |
  [QUIC 连接]         |-------- QUIC Connect ----------->|
                      |                                   |
                      |--- FirstQuicMsg (JSON) --------->| [process_first_msg]
                      |   {token, uuid, msg_type, ...}   |
                      |                                   |
                      |<------ (隐式确认) ---------------| [verify_token + verify_max_client]
                      |                                   |
                      |--- [文本消息] ------------------>| [process_rec_msg]
                      |   [HeadMsg(9B) + TextQuicMsg]    |   -> 解析粘包/半包
                      |                                   |   -> 验证 CRC-16
                      |                                   |   -> 存储到数据库 (异步)
                      |                                   |   -> 路由到接收者
                      |                                   |   -> 发送 ACK (201)
                      |                                   |
                      |<--- [文本消息] ------------------| [send_msg_to_user]
                      |   (来自其他用户的路由消息)         |
                      |                                   |
                      |--- [PING (type=99)] ------------>| [send_ping]
                      |<--- [PONG (type=99)] ------------|
                      |                                   |
                      |---- (流关闭) ------------------->| [end_server]
                      |                                   |  -> 从全局映射中移除
                      |                                   |  -> 删除 Redis 键
                      |                                   |  -> user_offline()
                      |                                   |  -> 持久化已读记录
```

### 连接建立详细流程

1. **`start_server()`** 读取配置，调用 `init_server(addr)`
2. **`run_server(addr)`** 创建 Endpoint，无限循环接受连接
3. **`handle_connection(conn)`** 接受双向流循环
4. **`handle_conn(send_stream, recv_stream, address)`** 单个流的完整生命周期:
   - `process_first_msg()` - 读取 JSON 握手消息
   - `verify_token()` - 验证 JWT，检查 UUID 匹配 (JWT 过期时间: 5 分钟)
   - `verify_max_client()` - 检查连接数是否超过 1000
   - `user_online()` - 用户上线逻辑 (占位符)
   - `set_conn_info()` - 注册连接到全局映射和 Redis (TTL: 7200 秒)
   - 循环读取数据 (10KB 块) -> `process_rec_msg()` 解析消息
   - `end_server()` - 流关闭时清理资源

### 消息组帧协议

消息格式: `[HeadMsg (9 字节, bincode)] + [TextQuicMsg (可变长度, bincode)]`

`get_text_msg()` 实现 **粘包/半包处理**:
1. 追加上一轮未处理完的残留字节 (存储在 `buffer_msg` 中)
2. 遍历组合后的缓冲区:
   - 从位置 `i` 反序列化 `HeadMsg` (9 字节)
   - 检查 `body_len + header_size` 是否在剩余数据中
   - 如果不够，存储剩余字节作为半包并返回
   - 如果足够，反序列化 `TextQuicMsg` 消息体
   - 验证 CRC-16 校验码
   - 推进位置并重复

### 客户端流程 (`quic_client.rs`)

```rust
pub async fn run_client(server_addr: SocketAddr)
```

1. 创建 Quinn Endpoint 在 `0.0.0.0:0`
2. 使用域名 `"onlytalk.cn"` 连接服务器
3. 打开双向流
4. 生成接收消息循环
5. `init_send_msg()`:
   - 构造并发送 JSON 握手消息
   - 发送 5 条测试文本消息
   - 启动 60 秒心跳循环 (每 60 秒发送 `MSG_TYPE_PING`)

---

## P2P NAT 穿透服务

### 概述

P2P 服务是一个 **NAT 穿透协调服务** (类似 STUN/TURN 信令服务器)。它通过交换两个客户端的观察公网地址，帮助它们建立直接 UDP 连接。

### P2P 握手协议

```
   客户端 A                    UDP 服务器                    客户端 B
     |                              |                              |
     |-- UDP: {uuid:A, target:B} -->|                              |
     |                              |-- Redis 锁获取 --------------|
     |                              |-- 存储 A 的地址到 Redis      |
     |                              |                              |
     |                              |<-- UDP: {uuid:B, target:A} --|
     |                              |-- Redis 锁: 已被持有         |
     |                              |-- 判定 NAT 类型              |
     |                              |-- 从 Redis 获取 B 的地址     |
     |                              |                              |
     |                              |-- NAT 兼容性检查:            |
     |                              |   (3,3) -> A=服务端, B=客户端|
     |                              |   (3,4) -> A=服务端, B=客户端|
     |                              |   (4,3) -> B=服务端, A=客户端|
     |                              |   (4,4) -> 无法连接          |
     |                              |                              |
     |<-- QUIC: P2P 信息 (B 的地址)-|-- QUIC: P2P 信息 (A 的地址)-->|
     |    MSG_TYPE=203/204           |    MSG_TYPE=203/204           |
     |                              |                              |
     |-- 直接 UDP P2P -----------> (客户端建立直接连接)
```

### NAT 类型判定

使用 **锁竞争技术** 判定 NAT 类型:
1. 用户 A 发送 UDP 信息，服务器尝试获取 Redis 锁
2. 如果用户 B 在 A 的锁过期前发送，B 发现锁已被持有
3. B 比较自己的 UDP 地址与锁持有者的地址:
   - 相同地址 = NAT 类型 3 (Full Cone / 可预测端口)
   - 不同地址 = NAT 类型 4 (Symmetric NAT / 不可预测端口)

### 关键设计

- 使用 **Redis 分布式锁** (`acquire_lock` / `release_lock`)，TTL 30 秒
- 用户地址信息存储在 Redis 中，TTL 60 秒 (超时需重新注册)
- P2P 协调消息通过已有的 QUIC 连接发送 (非 UDP)
- Symmetric-to-Symmetric NAT 对无法建立直接连接 (已知限制)
- 服务器根据 NAT 类型决定哪个对等方作为 "服务端" (监听) vs "客户端" (发起)

---

## 配置与常量

### Redis 键前缀

| 常量 | 值 | 用途 |
|------|-----|------|
| `REDIS_QUIC_SERVERS` | `"QUIC:SERVER:"` | QUIC 连接键前缀 |
| `USER_READ_MSG` | `"USER:READ:MSG:"` | 已读记录键前缀 |
| `REDIS_SPLIT` | `":"` | Redis 键分隔符 |

### 系统常量

| 常量 | 值 | 用途 |
|------|-----|------|
| `SYSTEM` | `"system"` | 系统消息发送者 ID |
| `PING` | `"ping"` | 心跳负载 |
| `PONG` | `"pong"` | 心跳响应负载 |
| `SERVER_NAME` | `"SERVER_1"` | 服务器标识符 |
| `MAX_QUIC_SERVERS` | `1000` | 最大并发 QUIC 连接数 |
| `MAX_QUIC_BUFFER_LEN` | `10,485,760` (10MB) | 最大半包消息缓冲区 |
| `PC_PLATFORM` | `"PC"` | 桌面客户端标识符 |
| `MOBILE_PLATFORM` | `"MOBILE"` | 移动客户端标识符 |

### 文件路径

| 路径 | 用途 |
|------|------|
| `./config/app_config.toml` | 服务器配置 |
| `./config/ssl/fullchain.pem` | TLS 证书链 |
| `./config/ssl/privkey.pem` | TLS 私钥 |

---

## 错误处理

1. **`anyhow::Result`** - 所有函数统一使用 `anyhow::Error` 进行错误传播
2. **握手阶段错误** - 优雅关闭发送流后返回错误 (`send_stream.finish()`)
3. **日志记录** - 所有错误路径使用 `tracing` 记录 (`error!` / `warn!` / `info!`)
4. **流处理循环** - 使用 `unwrap_or_else` 记录错误而不中断连接
5. **缓冲区溢出保护** - 超过 10MB 时断开连接
6. **Redis 容错** - 使用 `unwrap_or_else` 容忍部分 Redis 操作失败

---

## JWT 认证

- 算法: RS256 (RSA 签名)
- 过期时间: **5 分钟** (300 秒)
- Claims 结构:
  ```rust
  pub struct Claims {
      pub sub: String,     // 扩展信息 (平台标识)
      pub uuid: String,    // 用户唯一 ID
      pub exp: i64,        // 过期时间戳 (秒级 Unix 时间戳)
  }
  ```
- `jsonwebtoken` 库在 `decode` 时自动校验 `exp`，过期会返回 `ExpiredSignature` 错误
