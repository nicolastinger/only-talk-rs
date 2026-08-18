# http_service 集成测试说明

文件：`crates/http_service/tests/http_service_integration_test.rs`

该测试在**独立的测试数据库 + 测试 Redis** 上启动真实的 Actix HTTP 服务（包含 `TraceIdMiddleware` 与全局 JWT 鉴权中间件 `error_record_middleware`），通过 `actix_web::test` 直接调用 HTTP 接口，覆盖用户模块的**注册 / 登录 / 查询 / 刷新 token** 等核心流程。

测试结束后自动执行清理：**删除测试数据库**、**清空测试 Redis 的所有 key**。

---

## 1. 前置条件

| 依赖 | 要求 |
| --- | --- |
| PostgreSQL | 本地可连接，`.env` 中 `DATABASE_URL` 指向管理员库（如 `postgres`），需有建库权限 |
| Redis | 本地可连接，测试只消费 `redis.test_url`（`TEST_REDIS_URL`）；`REDIS_URL` 对本测试非必需（仅使 `[redis].url` 替换为空串），测试 Redis 建议使用独立 DB index 或实例 |
| `.env` | 仓库根目录存在，且为**有效 UTF-8** 编码（`dotenvy` 无法读取非 UTF-8 文件，会静默失败） |

### 相关配置项

```toml
# config/app_config.toml
[redis]
url = "${REDIS_URL}"
test_url = "${TEST_REDIS_URL}"   # 测试 Redis 连接串（新增配置项）
```

```dotenv
# .env / .env.example
DATABASE_URL=postgres://postgres:...@127.0.0.1:5432/postgres
REDIS_URL=redis://:...@127.0.0.1:6379/
TEST_REDIS_URL=redis://:...@127.0.0.1:6379/15   # 建议独立 DB index，避免影响主库
```

> ⚠️ 测试会执行 `FLUSHALL` 清空 `TEST_REDIS_URL` 指向 Redis 的**所有 key**，请勿把生产 Redis 配到这里。

## 2. 运行方式

```bash
# 从仓库根目录
cargo test -p http_service --test http_service_integration_test -- --ignored
# 带日志查看细节
cargo test -p http_service --test http_service_integration_test -- --ignored --nocapture
```

- 测试标记为 `#[ignore]`，普通的 `cargo test --workspace` 不会执行它。
- 首次运行会编译较慢，之后每次约 5~6 秒。
- 可重复运行：每次结束都会删库并清空 Redis，不留下脏数据。

---

## 3. 测试流程

```
初始化 tracing / 加载 .env
        │
        ▼
加载 app_config.toml（从仓库根目录绝对路径，因 cargo 运行测试时 cwd 是包目录）
读取 database.url、redis.test_url
        │
        ▼
连接管理员库 → ensure_database 创建测试库 only_talk_http_test（不存在则 CREATE）
        │
        ▼
连接测试 Redis（redis.test_url）→ PING 验证连通
        │
        ▼
┌─────────────── 测试主体（catch_unwind 包裹，保证 panic 也执行清理）───────────────┐
│  1. 连接测试库，entity::ddl::apply_all_ddl 执行全部建表脚本（幂等）                │
│  2. 校验关键表存在（basic_user / user_info / friend_link / group_info）            │
│  3. FLUSHALL 清空测试 Redis（保证起点干净）                                        │
│  4. 写入种子用户（BasicUser + UserInfo，密码用 Argon2 哈希，与业务一致）            │
│  5. 构造 AppState{core: CoreState{db, redis}, s3, email}，init_service 启动测试应用   │
│  6. 依次执行场景断言（见 §4）                                                       │
└─────────────────────────────────────────────────────────────────────────────────┘
        │
        ▼
清理（无条件执行）：FLUSHALL 清空测试 Redis → drop_test_database 删除测试库
        │
        ▼
返回结果 / 透传 panic
```

## 4. 覆盖的接口与断言

| # | 接口 | 请求 | 期望结果 |
| --- | --- | --- | --- |
| 1 | `POST /user/me`（无 token） | — | 鉴权中间件直接拒绝（`try_call_service` 返回 Err） |
| 2 | `POST /user/sign_in` | `{account, password, platform:"PC"}` | 200，`code=200`，返回 `access_token`/`refresh_token` |
| 3 | `POST /user/me`（带 token） | — | 200，返回种子用户信息，`account` 一致 |
| 4 | `POST /user/get_user_by_account/{account}` | — | 200，返回种子用户 |
| 5 | `POST /user/get_uuid_by_account/{account}` | — | 200，`data` 为种子用户 uuid |
| 6 | `GET /user/get_user_by_uuid/{uuid}` | — | 200，返回种子用户 |
| 7 | `POST /user/get_exit_user_flag/is_exit` | 请求体为纯文本账号 | 响应体 `true` |
| 8 | `POST /user/refresh_token` | `{refresh_token}` | 200，返回新 `access_token` |
| 9 | `POST /user/sign_up`（新邮箱） | 先向 Redis 预置验证码，再提交注册 | 200，`code=204` 注册成功 |
| 10 | `POST /user/sign_up`（同一邮箱重复） | 重复提交 | 400（错误消息未做断言） |
| 11 | `POST /user/sign_up`（验证码错误） | `verification_code` 与 Redis 不一致 | 400（错误消息未做断言） |
| 12 | `POST /user/sign_in`（新注册账号） | 新账号登录 | 200，注册链路全通 |

说明：

- 注册验证码由测试**直接写入测试 Redis**（key 格式与业务一致：`EMAIL:VERIFY:CODE:{邮箱}` 转大写，5 分钟过期），从而绕开真实邮件服务。
- 种子用户直接写库，密码哈希使用与 `user_service` 相同的 `hash_password`（Argon2），保证 `sign_in` 校验通过。
- JWT 签名依赖 `config/jwt/*.key`（不存在时 `get_rsa_keys` 会自动生成，同进程内签名与校验使用同一份密钥）。

## 5. 清理机制

测试主体被 `futures_util::FutureExt::catch_unwind()` 包裹，无论**成功、断言失败 panic、还是返回 Err**，都会执行以下清理：

1. `FLUSHALL`：清空 `TEST_REDIS_URL` 指向 Redis 的所有 key（含测试期间写入的验证码、refresh_token 等）。
2. `drop_test_database`：先 `pg_terminate_backend` 终止测试库残留连接，再 `DROP DATABASE IF EXISTS only_talk_http_test`。

保证测试可重复执行、不留残留。

## 6. 实现细节与注意点

- **cwd 问题**：`cargo test` 运行集成测试时工作目录是包目录 `crates/http_service`，而非仓库根目录。因此：
  - 通过 `dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.env"))` 用绝对路径加载 `.env`；
  - 手动读取 `../../config/app_config.toml` 并调用 `common::substitute_env_vars` + `common::init_global_config!` 填充全局配置（行为与 `common::init_app_config` 一致）。
- **不使用 `serde_json::json!` 宏**：其内部调用 `unwrap()`，违反仓库 `clippy.toml` 的 `disallowed_methods` 规范；改用自写 `json_obj` 构造 JSON。
- **隔离性**：测试库名 `only_talk_http_test` 与 entity 集成测试的 `only_talk_test` 区分，互不干扰；测试 Redis 建议独立 DB index（如 `/15`）。
- **`unwrap()` 禁令**：与仓库规范一致，测试内使用 `?` / `context` / `unwrap_or`（如 `unwrap_or(JsonValue::Null)`，无 `.expect(` 调用）。
- **redis 连接池**：直接用 `deadpool_redis::Config::from_url(...).create_pool()` 创建，不走 `common::init_redis`，便于测试隔离。

## 7. 故障排查

| 现象 | 可能原因 |
| --- | --- |
| `创建 Redis 连接池失败: Redis URL did not parse` | `TEST_REDIS_URL` 为空 → `.env` 未加载或编码非 UTF-8 |
| `未找到 redis.test_url` | `config/app_config.toml` 缺少 `test_url` 项，或 `.env` 未配置 `TEST_REDIS_URL` |
| `未找到 database.url` | `.env` 未加载或缺少 `DATABASE_URL` |
| `测试库创建失败 / 连接超时` | PostgreSQL 未启动、账号无建库权限、端口与 `.env` 不一致 |
| 断言失败但日志显示清理已执行 | `catch_unwind` 保证清理，直接看具体 assert 的 message 即可 |
