# http_service crate

HTTP REST 服务层（actix-web），实现所有面向客户端的业务 API：用户、好友、群聊、单聊消息、文件、通知、S3 管理。被 `api` crate 组装进 HTTP server。

## 职责

- 提供 actix `configure_routes` 路由注册入口（`/user` `/friend` `/group` `/msg` `/file` `/notify` 六个 scope）
- controller 层：接收 `web::Data<AppState>` 依赖注入，解析请求 DTO/路径/Header，调用 service
- service 层：业务逻辑，通过窄签名（`rb: &RBatis` / `redis: &Pool` / `s3: Option<Arc<S3Client>>`）使用基础设施
- 中间件：`TraceIdMiddleware`（全链路 TraceId）、`error_record_middleware`（全局 JWT 鉴权 + 错误记录）
- 依赖注入宿主：定义 `AppState { core, s3, email }`（组合 `common::CoreState`），供 controller 统一取用

## 依赖

- `common`（实体模型、配置、JWT）、`entity`、`s3_service`、`email_service`、`actix-web`、`deadpool-redis`、`rbatis`
- 被 `api` 依赖，不依赖 `quic_service`

## 结构

```
http_service/src/
├── lib.rs                # pub mod 声明（common / http_service / middleware / state / utils）
├── state.rs              # AppState（core + s3 + email）及 db()/redis()/s3() 访问器
├── middleware/           # trace_id.rs（TraceId）、record_bad_http.rs（JWT 鉴权）
├── utils/                # http_response.rs（统一响应）、http_macros.rs（validate/respond 宏）、file_utils.rs
├── common/dto/           # 跨模块 DTO（AuthAccount、BasePageDTO、ReqList）
└── http_service/
    ├── user_service/     # 用户：注册/登录/token/资料；好友：申请/列表/删除
    ├── chat_service/     # 单聊：聊天记录、未读、已读回执（Redis）
    ├── group_service/    # 群聊：建群/成员/邀请/消息历史/未读/解散，成员缓存到 Redis
    ├── notify_service/   # 系统通知查询
    └── file_service/     # 文件：上传/下载/预签名 URL、S3 bucket 管理（s3_controller）
```

## 关键约定

- **依赖注入**：controller 只引用 `web::Data<AppState>`，经 `state.db()` / `state.redis()` / `state.s3()` 取连接；service 层一律通过窄签名（`rb: &RBatis` / `redis: &Pool`）使用基础设施，不访问任何全局单例（连接池全局单例已移除）。
- 鉴权：除 `sign_up`/`sign_in`/`refresh_token` 外所有端点经中间件校验 Bearer JWT，用户 UUID 注入 `AuthAccount` 扩展。
- service 函数统一返回 `Result<String, anyhow::Error>`（已是 JSON 字符串），controller 用 `respond_json_any!` 宏包装。
- `s3_controller`（`/s3/*`）是通用 S3 管理端点；S3 未启用时 `state.s3` 为 `None`，这些端点返回"未初始化"错误。
- 集成测试：`tests/http_service_integration_test.rs`（`#[ignore]`，需本地 PG/Redis，见 [http_service 集成测试说明](./http_service_integration_test.md)）。

## 部署形态

仅作为库被 `api` crate 的 `init_server.rs` 加载路由，本身不提供可执行入口。
