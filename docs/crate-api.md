# api crate

应用组装层，把 http_service、quic_service、common 组合成可运行的 HTTP/HTTPS 服务。提供 TLS 初始化、依赖注入装配（AppState）、以及需要跨服务协作的"集成接口"（好友+通知、上传+业务记录）。

## 职责

- **服务装配**：`init_server::start_server()` 初始化配置/DB/Redis/S3/email，构造 `AppState` 并注入 actix；配置 TLS（`config/ssl/`）、注册中间件与全部路由
- **集成服务**：`service/` 下实现需要跨模块编排的业务：
  - `user_integrated_service`：加好友+发系统通知、处理好友申请+通知、分配 QUIC 节点（hash 取模）
  - `upload_file_integrated_service`：用户/群头像与聊天文件上传（S3）+ 建业务记录 + 更新用户/群头像
- **可执行入口**：`bin/api_server.rs` —— 拆分部署时独立运行的 HTTP API 进程

## 依赖

- `http_service`、`quic_service`、`common`、`s3_service`、`email_service`
- 是依赖关系图的顶点：它依赖其它所有业务 crate，但不会被它们反向依赖

## 结构

```
api/src/
├── lib.rs                # mod controller / service，pub mod init_server
├── bin/api_server.rs     # 拆分部署的 API 进程入口（仅 HTTP）
├── init_server.rs        # start_server()：装配 + TLS + 路由 + AppState 注入
├── controller/           # user_integrated_controller（/add_friend_with_notify 等）、
│                         #   upload_file_integrated_controller（/upload/user_avatar 等）
└── service/              # user_integrated_service、upload_file_integrated_service
```

## 关键约定

- controller 与 http_service 的 controller 一样，只引用 `web::Data<AppState>`（`http_service::state::AppState`）。
- `AppState` 在 init_server 中构造一次，`HttpServer::new` 闭包里 `.clone()` 后每个 worker 一份。
- 单例初始化（`init_sql_pool` / `init_redis`）与 quic_service 幂等共享——两者先后调用不会重复建池。
- `#![deny(clippy::unwrap_used)]` 生效。

## 部署形态

- 单体模式：根 crate `src/main.rs` 调用 `api::init_server::start_server()` + `quic_service::init_server::start_server()`
- 拆分模式：`cargo run -p api --bin api_server`（仅 HTTP），与 `quic_server` 分开部署
