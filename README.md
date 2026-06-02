# OnlyTalk RS

> 高性能即时通讯服务器 — Rust + Actix-web + QUIC + PostgreSQL + Redis

[English Version](README_EN.md)

## 项目简介

OnlyTalk RS 是一个基于 Rust 构建的高性能即时通讯（IM）后端服务器，采用 Actix-web 提供 RESTful API，QUIC 协议处理实时消息传输，支持 P2P 打洞、NAT 穿透、群聊广播等高级 IM 功能。

## 技术栈

| 组件 | 技术 | 说明 |
|------|------|------|
| Web 框架 | Actix-web 4.9 | HTTP/HTTPS API 服务 |
| 协议 | QUIC (quinn 0.10) | 实时消息传输、P2P 连接 |
| 数据库 | PostgreSQL + rbatis 4.6 | ORM 数据持久化 |
| 缓存 | Redis + deadpool-redis | 连接池、会话管理 |
| 对象存储 | AWS S3 SDK | 支持 MinIO / 阿里云 OSS / AWS S3 |
| 认证 | JWT (jsonwebtoken 8.0) | 用户身份验证 |
| 日志 | tracing + tracing-subscriber | 结构化日志、JSON 输出 |
| 加密 | rustls 0.21 + rcgen | TLS 证书管理 |
| Rust Edition | 2024 | 最新语言特性 |

## 工作区结构

```
only-talk-rs/
├── src/main.rs                  # 独立模式入口（同时启动 QUIC + HTTP）
├── crates/
│   ├── api/                     # API 服务层（整合 HTTP + QUIC 路由）
│   ├── common/                  # 公共工具：配置管理、JWT、RSA 加密、QUIC 配置、tracing
│   ├── entity/                  # 数据模型与数据库操作（rbatis）、DDL 脚本
│   ├── email_service/           # 邮件服务（SMTP via reqwest，含连接池）
│   ├── http_service/            # HTTP 端点：用户、好友、群组、聊天、通知、文件
│   ├── quic_service/            # QUIC 服务：外网连接、内网通信、NAT 打洞、P2P 转发
│   └── s3_service/              # S3 对象存储抽象层（多提供商支持）
├── config/
│   ├── app_config.toml          # 应用配置（数据库、Redis、QUIC、文件、S3）
│   └── ssl/                     # TLS 证书（fullchain.pem + privkey.pem）
├── docs/                        # 技术文档
├── .env.example                 # 环境变量模板
├── Cargo.toml                   # 工作区配置
├── clippy.toml                  # Clippy 规则
└── rustfmt.toml                 # 代码格式化规则
```

## 快速开始

### 前置要求

- **Rust**: 1.75+ (Edition 2024)
- **PostgreSQL**: 12+
- **Redis**: 6+
- **TLS 证书**: 自签名或正式证书（放置于 `config/ssl/` 目录）

### 1. 克隆项目

```bash
git clone <repository-url>
cd only-talk-rs
```

### 2. 配置环境变量

```bash
cp .env.example .env
```

编辑 `.env` 文件，填入实际配置：

```env
# 数据库
DATABASE_URL=postgres://postgres:YOUR_PASSWORD@127.0.0.1:5432/postgres

# Redis
REDIS_URL=redis://:YOUR_PASSWORD@127.0.0.1:6379/

# S3 对象存储（可选）
S3_ENDPOINT=http://127.0.0.1:9000
S3_ACCESS_KEY=your-access-key
S3_SECRET_KEY=your-secret-key
```

### 3. 配置应用

编辑 `config/app_config.toml`，调整以下关键配置：

```toml
[server]
address = "0.0.0.0:8443"        # HTTPS API 监听地址
log_level = "info"              # 日志级别：debug / info / warn / error

[quic_server]
address = "0.0.0.0:4433"        # QUIC 外网监听端口

[internal_quic_server]
address = "127.0.0.1:4434"      # QUIC 内网通信端口

[s3]
enabled = false                 # 是否启用 S3 存储（false 则使用本地存储）
provider = "minio"              # minio / aliyun_oss / aws_s3
```

### 4. 放置 TLS 证书

将 TLS 证书放置到 `config/ssl/` 目录：

```
config/ssl/fullchain.pem    # 证书链
config/ssl/privkey.pem      # 私钥（支持 RSA / EC / PKCS8 格式）
```

开发环境可使用 rcgen 自动生成自签名证书。

### 5. 初始化数据库

执行 `crates/entity/` 中的 DDL 脚本创建数据表。

### 6. 启动服务

```bash
# 开发模式
cargo run

# 指定日志级别
RUST_LOG=debug cargo run

# 发布模式
cargo run --release
```

服务启动后将同时运行：
- **HTTPS API**：`https://0.0.0.0:8443`
- **QUIC 外网服务**：`0.0.0.0:4433`
- **QUIC 内网服务**：`127.0.0.1:4434`

## 部署模式

| 模式 | 入口 | 端口 | 说明 |
|------|------|------|------|
| **独立模式** | `src/main.rs` | 8443 + 4433 + 4434 | QUIC + HTTP 同进程运行 |
| **QUIC 网关** | `crates/quic_service/src/bin/quic_server.rs` | 4433 + 4434 | 仅 QUIC 连接管理 |
| **API 服务** | `crates/api/src/main.rs` | 8443 | 仅 HTTP REST API |

### 独立模式（默认）

```bash
cargo run --release
```

QUIC 服务与 HTTP 服务在同一进程中启动，适合中小型部署。

### 分离部署

适合大规模集群部署，QUIC 网关与 API 服务独立运行。

## API 端点

### HTTP REST API

| 路由前缀 | 模块 | 功能 |
|----------|------|------|
| `/user` | user_service | 用户注册、登录、资料管理、密码重置 |
| `/friend` | friend_service | 好友申请、好友列表、好友关系管理 |
| `/group` | group_service | 群组创建、成员管理、群消息历史 |
| `/msg` | chat_service | 文本消息、消息查询、消息漫游 |
| `/file` | file_service | 文件上传/下载、头像管理、S3 预签名 URL |
| `/notify` | notify_service | 系统通知、推送通知 |
| `/integrated` | api/controller | 综合用户服务与文件上传服务 |

### QUIC 协议

- **外网连接**：客户端通过 `4433` 端口建立 QUIC 长连接
- **内网通信**：服务器节点间通过 `4434` 端口内部通信
- **NAT 打洞**：UDP 端口 `9562-9565` 用于 P2P 穿透

## 特性

### 消息传输

- 基于 QUIC 双向 Stream 实现实时消息推送
- 支持文本消息、文件消息、图片预览
- 消息 CRC 校验确保传输完整性

### P2P 连接

- UDP NAT 发现（端口 9562-9565）
- P2P 打洞建立直连通道
- 打洞失败自动降级为服务器中转

### 文件存储

- 支持本地存储与 S3 对象存储无缝切换
- 图片自动压缩为 WebP 格式
- 分片上传（大文件支持）
- 预签名 URL 安全访问

### 认证与安全

- JWT Token 认证
- Argon2 密码哈希
- RSA 公钥加密传输敏感数据
- HTTPS (TLS 1.3) API 传输加密
- QUIC 内置 TLS 加密

### 日志与监控

- 结构化日志（tracing + JSON 输出）
- TraceId 全链路追踪
- 日志文件自动轮转
- 错误请求自动记录

## 配置说明

### app_config.toml

```toml
[database]
url = "${DATABASE_URL}"         # PostgreSQL 连接字符串

[redis]
url = "${REDIS_URL}"            # Redis 连接字符串

[server]
address = "0.0.0.0:8443"        # HTTPS 监听地址
locales = "zh-CN"               # 国际化语言
log_level = "info"              # 日志级别

[quic_server]
address = "0.0.0.0:4433"        # QUIC 外网地址
cert_path = "./config/ssl/fullchain.pem"
key_path = "./config/ssl/privkey.pem"

[file_upload]
max_file_size = 20971520        # 最大文件上传大小（20MB）

[s3]
enabled = true                  # 是否启用 S3
provider = "minio"              # 存储提供商
endpoint = "${S3_ENDPOINT}"
access_key = "${S3_ACCESS_KEY}"
secret_key = "${S3_SECRET_KEY}"
default_bucket = "only-talk-rs"
```

### 文件类型配置

支持图片、文档、压缩包、音频、视频等多种文件类型，每种类型可配置允许的扩展名和 MIME 类型。

## 开发指南

### 代码规范

- `clippy::unwrap_used` 被拒绝 — 使用 `expect("reason")` 或正确的错误处理
- `rustfmt`: 4 空格缩进、Unix 换行符、分组导入（StdExternalCrate）
- 错误处理：`anyhow` 用于应用层错误，`thiserror` 用于领域层错误

### 常用命令

```bash
# 构建
cargo build --release

# 运行
cargo run --release

# Clippy 检查
cargo clippy --workspace

# 代码格式化
cargo fmt --all

# 运行测试
cargo test --workspace

# 运行 Benchmark
cargo bench -p quic_service
```

### 新增 API 端点

1. 在 `crates/http_service/src/http_service/` 下创建对应模块
2. 创建 controller（处理 HTTP 请求）
3. 创建 service（业务逻辑）
4. 在 `mod.rs` 中注册路由

### 新增 QUIC 消息类型

1. 在 `crates/quic_service/src/msg_service/` 中定义消息类型
2. 实现消息序列化/反序列化
3. 注册到消息分发器

## 文档

- [启动部署指南](docs/启动部署指南.md)
- [集群路由方案](docs/集群路由方案.md)
- [群聊广播方案](docs/群聊广播方案.md)
- [Entity 核心拆分方案](docs/entity-core拆分方案.md)

## License

[LICENSE](LICENSE)
