# Rust IM - 即时通讯系统

一个基于 Rust 开发的高性能即时通讯系统，支持用户管理、好友关系、实时消息传输、文件上传等功能。

## 功能特性

- 用户注册与登录
- 好友管理系统（添加好友、处理好友请求）
- 实时消息传输（基于 QUIC 协议）
- 客户端 P2P 连接支持（NAT 发现 + 请求转发）
- 系统通知推送
- 聊天记录管理
- 文件上传（头像 / 聊天文件，支持 S3 + 本地存储）
- 邮件服务

## 技术架构

### 核心技术栈

- **语言**: Rust (edition 2024)
- **Web 框架**: Actix-web
- **数据库**: PostgreSQL (rbatis ORM)
- **缓存**: Redis (deadpool-redis 连接池)
- **传输协议**: 
  - HTTP/HTTPS (API 接口)
  - QUIC (实时消息 + 节点间通信)
  - UDP (NAT 发现 + P2P 请求转发)
- **认证**: JWT Token
- **对象存储**: S3 兼容协议 (MinIO / Aliyun OSS / AWS S3)

### 项目结构

```
only_talk_rs/
├── src/main.rs                    # 单机模式二进制入口
├── config/                        # 配置文件 & TLS 证书
├── docs/                          # 方案文档
├── crates/
│   ├── entity/                    # 数据实体、DB 操作、工具函数
│   ├── http_service/              # HTTP 服务实现（用户/好友/通知/文件）
│   ├── quic_service/              # QUIC 协议服务（连接管理 + NAT 发现 + 内网桥接）
│   │   ├── src/bin/quic_server.rs # 集群模式 QUIC 网关独立入口
│   │   └── src/nat_ip/            # NAT 发现 + P2P 请求转发
│   ├── api/                       # API 服务入口（整合 HTTP + QUIC）
│   ├── s3_service/                # S3 对象存储服务
│   └── email_service/             # 邮件服务
└── log/                           # 应用日志
```

### 部署模式

| 模式 | 入口 | 说明 |
|------|------|------|
| **单体** | `src/main.rs` | 单进程运行 QUIC + HTTP |
| **QUIC 网关** | `crates/quic_service/src/bin/quic_server.rs` | 仅 QUIC 服务，集群部署 |
| **API 服务** | `crates/api/src/main.rs`（待创建） | 仅 HTTP API，集群部署 |

详见 [启动部署指南](docs/启动部署指南.md) 和 [集群路由方案](docs/集群路由方案.md)。

### 主要组件说明

1. **用户服务** (`http_service::user_service`)
   - 用户注册、登录
   - 用户信息管理
   - 用户头像管理

2. **好友服务** (`http_service::friend_service`)
   - 好友请求发送与处理
   - 好友列表管理
   - 好友关系维护

3. **聊天服务** (`http_service::chat_service`)
   - 文本消息发送与接收
   - 聊天记录存储
   - 消息已读状态管理

4. **通知服务** (`http_service::notify_service`)
   - 系统通知推送
   - 通知状态管理

5. **文件服务** (`http_service::file_service`)
   - 用户头像上传（S3 / 本地）
   - 聊天文件上传（S3 / 本地）
   - 支持分片上传、预签名 URL

6. **QUIC 服务** (`quic_service`)
   - 基于 QUIC 协议的消息收发
   - 外网 QUIC（`:4433`）— 客户端连接
   - 内网 QUIC（`:4434`）— HTTP API 桥接
   - NAT UDP（`:9562-9565`）— NAT 发现 + P2P 请求转发

7. **S3 服务** (`s3_service`)
   - MinIO / Aliyun OSS / AWS S3 统一接口
   - 分片上传、预签名 URL
   - 可配置降级为本地存储

## 快速开始

### 环境要求

- Rust 1.82 或更高版本
- PostgreSQL 数据库
- Redis 服务

### 配置文件

在 `config/app_config.toml` 中配置：

```toml
[database]
url = "${DATABASE_URL}"

[redis]
url = "${REDIS_URL}"

[server]
address = "0.0.0.0:8443"

[quic_server]
address = "0.0.0.0:4433"
server_name = "127.0.0.1:4433"

[internal_quic_server]
address = "127.0.0.1:4434"
server_name = "INTERNAL_SERVER_1"

[s3]
provider = "minio"
enabled = true
# ... 见 config/app_config.toml
```

配置文件支持 `${ENV_VAR}` 环境变量占位符，配合 `.env` 文件使用。

### 安装与运行

```bash
# 克隆项目
git clone <repository-url>
cd only_talk_rs

# 构建项目
cargo build --release

# 运行服务（单体模式）
cargo run --release
```

服务启动后在以下端口监听：

| 端口 | 服务 |
|------|------|
| 8443 | HTTPS API |
| 4433 | QUIC 外网（客户端连接） |
| 4434 | QUIC 内网（API 桥接） |
| 9562-9565 | NAT UDP |

## 数据库设计

主要表：

- `basic_user` — 用户基本信息
- `user_info` — 用户详细信息
- `friend_link` — 好友关系
- `friend_request_info` — 好友请求
- `chat_message_record` — 聊天记录
- `system_notification` — 系统通知

完整的数据库结构请参考 [entity/ddl/v1_0_0.sql](crates/entity/ddl/v1_0_0.sql)

## API 接口

### 用户相关
- `POST /user/signUp` — 用户注册
- `POST /user/login` — 用户登录
- `GET /user/info` — 获取用户信息

### 好友相关
- `POST /friend/add` — 添加好友
- `POST /friend/process` — 处理好友请求
- `GET /friend/list` — 获取好友列表

### 集成服务
- `POST /integrated/add_friend_with_notify` — 添加好友并发送通知
- `POST /integrated/process_friend_with_notify` — 处理好友请求并通知
- `GET /integrated/quic_servers` — 获取可用 QUIC 服务器列表

### 文件上传
- `POST /file_integrated/upload/user_avatar` — 上传用户头像
- `POST /file_integrated/upload/user_chat/{friend_uuid}` — 上传聊天文件

### 通知相关
- `GET /notify/list` — 获取通知列表
- `PUT /notify/read/{id}` — 标记通知已读

## 日志

应用日志保存在 `log/rust_im.log` 文件中。

## 许可证

[MIT License](LICENSE)
