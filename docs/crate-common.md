# common crate

基础设施层，被所有业务 crate（http_service / quic_service / s3_service / api）共享。提供配置、连接池、JWT、tracing、加解密、消息协议等通用能力。

## 职责

- **配置管理**：读取 `config/app_config.toml`，支持 `${VAR}` 环境变量替换（`init_app_config` / `read_global_config!`），扁平化存入全局 `DashMap`
- **连接池**：PostgreSQL（rbatis `init_sql_pool`）与 Redis（deadpool `init_redis`）的连接池构造；池实例由调用方（`api` / `quic_service`）放入 `CoreState` 依赖注入，**无全局单例**
- **共享状态**：`state::CoreState { db, redis }` —— 被 `http_service::AppState` 组合，未来 quic_service 也可复用
- **JWT**：RS256 非对称签名，首次启动自动生成 RSA 密钥并落盘 `config/jwt/`，短效 access_token（24h）+ 长效 refresh_token（30d）
- **密码/加密**：Argon2 密码哈希校验、RSA 公钥加密
- **tracing**：双通道日志（文件 + stdout），日志级别从配置读取
- **消息协议**：单聊/群聊消息的 bincode + CRC 序列化（`text_msg.rs`）、消息类型常量（`message_types.rs`）、QUIC 内部转发消息（`internal_quic_msg.rs`）
- **集群辅助**：QUIC 节点注册与 server_count 后台同步（`server_count_sync.rs`）、内网 QUIC 客户端（`internal_quic_client.rs`）
- **Redis 工具**：分布式锁（`acquire_lock` / `release_lock`）、连通性校验（`verify_redis`）

## 依赖

- `entity`（重新导出其模型）、`rbatis`、`deadpool-redis`、`jsonwebtoken`、`rsa`、`argon2`、`quinn`/`rustls`（供协议复用）、`tracing` 全家

## 结构

```
common/src/
├── lib.rs               # 模块声明、env 替换、re-export
├── config_manager.rs    # 全局配置 DashMap 读写
├── config_str.rs        # 字符串常量（Redis key 前缀、消息类型、平台等）
├── state.rs             # CoreState（db + redis 共享状态）
├── macros/              # read_global_config! 等宏
├── tracing.rs           # 日志初始化
└── utils/
    ├── jwt_util.rs      # JWT 签发/校验（RS256）
    ├── rsa_util.rs      # RSA 密钥 + Argon2 密码哈希
    ├── redis_utils.rs   # Redis 连接池 + 分布式锁
    ├── sql_utils.rs     # SQL 连接池
    ├── text_msg.rs      # 单聊消息编解码（HeadMsg/TextQuicMsg + CRC）
    ├── internal_quic_*.rs # 节点间 QUIC 转发
    ├── server_count_sync.rs # 集群节点注册/计数
    └── time.rs / validators.rs / fatal_error.rs 等
```

## 注意事项

- 连接池不做全局单例：`init_sql_pool` / `init_redis` 每次调用都会创建新的池实例，由调用方放入 `CoreState { db, redis }` 后依赖注入（http_service 的 `AppState`、quic_service 的 `ChatNode` 均如此）。standalone 模式下 `api` 与 `quic_service` 各自持有独立连接池。
- `init_app_config` 会替换 `${VAR}`，缺失的变量会静默替换为空字符串——配置错误可能推迟到运行时才暴露。
- `#![deny(clippy::unwrap_used)]` 生效，禁止 `.unwrap()`，用 `expect("reason")` 或错误处理。
