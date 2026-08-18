# only_talk_rs（根 crate / 单体入口）

工作区根 crate，是单体部署模式的进程入口。本身不含业务代码，只负责把 quic_service 与 api 两个服务在同一进程内顺序启动。

## 职责

- 加载 `.env`（`dotenvy`）
- 初始化 tracing（`common::tracing::init_tracing`）
- 依次启动：
  1. **QUIC 服务**（`quic_service::init_server::start_server`）—— 客户端长连接、NAT/P2P、节点间通信
  2. **HTTP API 服务**（`api::init_server::start_server`）—— HTTPS REST 接口
- 任一步启动失败即 `fatal_panic_async` 终止进程

## 依赖

- `common`（tracing、fatal_panic）、`quic_service`、`api`

## 结构

```
src/main.rs    # 唯一的源文件（约 30 行）
```

## 关键约定

- 必须在工作区根目录运行（`./config/ssl/...`、`./config/app_config.toml` 是相对 cwd 的路径）。
- 端口：8443 HTTPS / 4433 QUIC 外部 / 4434 QUIC 内部 / 19562-19565 NAT UDP（以 `config/app_config.toml` 为准）。
- 前置依赖：PostgreSQL、Redis、S3 兼容对象存储（启动必选，`api::init_server` 在 `s3.enabled=false` 时拒绝启动）、`.env`（含 `S3_*`、`APP_DOMAIN` 等变量）、TLS 证书。

## 部署形态

- 单体模式（本 crate）：`cargo run` —— QUIC + HTTP 同进程
- 拆分模式：`cargo run -p quic_service --bin quic_server` + `cargo run -p api --bin api_server`
