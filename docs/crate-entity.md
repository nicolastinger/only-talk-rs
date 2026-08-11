# entity crate

数据访问层（rbatis ORM），定义数据库实体模型与 DDL 脚本，是整个项目唯一直接操作 PostgreSQL 的地方。

## 职责

- 用 `rbatis` 定义与数据库表一一对应的实体 struct（`#[derive(Serialize, Deserialize)]`）
- 通过 `crud!` / `impl_select!` 宏生成通用增删改查与自定义查询
- 维护表结构的 DDL 脚本（`ddl/` 目录），作为数据库初始化与变更的依据
- 模型字段约定：主键 `id`（自增），业务主键 `nano_id` / `uuid`（字符串/UUID），时间戳 `timestamp` / `created_at`

## 依赖

- `rbatis` + `rbdc-pg`（PostgreSQL 驱动）
- 不依赖 `common`、`http_service` 等上层 crate，被所有服务 crate 引用

## 结构

```
entity/src/
├── lib.rs                 # 模块声明
└── models/
    ├── user_entity/       # 用户：basic_user（账号密码）、user_info（资料）、friend_link/friend_list、friend_request_info
    ├── chat_entity/       # 单聊：chat_message_record（消息）、chat_message_record_read（已读）、add_read_chat_record
    ├── group_entity/      # 群聊：group_info、group_member、group_invitation、group_message_record
    ├── file_entity/       # 文件：file_upload_record、biz_file_link、biz_record
    └── notify_entity/     # 通知：system_notification

entity/ddl/                # 22 个 .sql 建表脚本（含 migrations/ 下的增量修复脚本）
```

## 注意事项

- DDL 是手工维护的 SQL 文件，**没有迁移工具**。改动表结构需同时更新 DDL 与实体模型，两者不一致会导致运行时 SQL 错误。
- `common` crate 会 `pub use entity::models` 重新导出模型，因此大部分上层 crate 通过 `common::models::*` 访问实体，不必直接依赖 `entity`。
- 查询偏好显式 `impl_select!`（如 `select_chat_by_limit`、`select_unread_by_time`）而非裸字符串 SQL。
