# Rust IM - 即时通讯系统

一个基于 Rust 开发的高性能即时通讯系统，支持用户管理、好友关系、实时消息传输等功能。

## 功能特性

- 用户注册与登录
- 好友管理系统（添加好友、处理好友请求）
- 实时消息传输（基于 QUIC 协议）
- P2P 直连通信
- 系统通知推送
- 聊天记录管理

## 技术架构

### 核心技术栈

- **语言**: Rust
- **Web框架**: Actix-web
- **数据库**: PostgreSQL
- **缓存**: Redis
- **传输协议**: 
  - HTTP/HTTPS (API接口)
  - QUIC (实时消息)
  - UDP (P2P直连)
- **认证**: JWT Token

### 项目结构

```
rust_my_app/
├── api/              # API服务入口
├── entity/           # 数据实体及数据库操作
├── http_service/     # HTTP服务实现
├── port_service/     # 端口相关服务(P2P)
├── quic_service/     # QUIC协议服务
└── config/           # 配置文件
```

### 主要组件说明

1. **用户服务** (`user_service`)
   - 用户注册、登录
   - 用户信息管理
   - 用户资料维护

2. **好友服务** (`friend_service`)
   - 好友请求发送与处理
   - 好友列表管理
   - 好友关系维护

3. **聊天服务** (`chat_service`)
   - 文本消息发送与接收
   - 聊天记录存储
   - 消息已读状态管理

4. **通知服务** (`notify_service`)
   - 系统通知推送
   - 通知状态管理

5. **P2P服务** (`p2p_service`)
   - UDP服务器用于P2P直连
   - NAT穿透支持

6. **QUIC服务** (`quic_service`)
   - 基于QUIC协议的可靠消息传输
   - 连接管理

## 快速开始

### 环境要求

- Rust 1.70 或更高版本
- PostgreSQL 数据库
- Redis 服务

### 配置文件

在 [config/app_config.toml](config/app_config.toml) 中配置数据库和Redis连接：

```toml
[database]
url = "postgres://admin:password@localhost:5432/postgres"

[redis]
url = "redis://:password@localhost:6379/"

[server]
address="0.0.0.0:8443"

[quic_server]
address="0.0.0.0:4433"
```

### 安装与运行

```bash
# 克隆项目
git clone <repository-url>
cd rust_my_app

# 构建项目
cargo build --release

# 运行服务
cargo run
```

服务启动后将在以下端口监听：
- HTTP API: 8443
- QUIC 服务: 4433
- P2P UDP: 9562/9563/9564/9565

## 数据库设计

项目使用 PostgreSQL 数据库，主要表包括：

- `basic_user`: 用户基本信息
- `user_info`: 用户详细信息
- `friend_link`: 好友关系
- `friend_request_info`: 好友请求
- `chat_message_record`: 聊天记录
- `system_notification`: 系统通知

完整的数据库结构请参考 [entity/ddl/v1_0_0.sql](crates/entity/ddl/v1_0_0.sql)

## API 接口

### 用户相关
- `POST /user/signUp` - 用户注册
- `POST /user/login` - 用户登录
- `GET /user/info` - 获取用户信息

### 好友相关
- `POST /friend/add` - 添加好友
- `POST /friend/process` - 处理好友请求
- `GET /friend/list` - 获取好友列表

### 消息相关
- `POST /msg/send` - 发送消息

### 通知相关
- `GET /notify/list` - 获取通知列表
- `PUT /notify/read/{id}` - 标记通知已读

## 日志

应用日志保存在 `log/rust_im.log` 文件中。

## 许可证

[MIT License](LICENSE)