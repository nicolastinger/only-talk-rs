# 数据库表创建说明

## 概述

本文档说明 `ddl/` 目录下的建表脚本（共 24 个 `.sql`：根目录 22 个 + `migrations/` 2 个）及其执行方式。

**执行顺序**：由文件名（`00_` / `01_` 前缀 + 字母序）决定，与「外键依赖」无关——除群表外，其余表之间**没有任何外键约束**（全目录仅 `01_group_tables.sql` 2 处、`group_invitation.sql` 1 处 `REFERENCES` 指向 `group_info`）。

**自动执行**：`crates/entity/src/ddl.rs::apply_all_ddl` 会按路径字典序自动执行目录内全部 `.sql`（含 `migrations/`，实际顺序为 `00_sequences` → `01_group_tables` → `basic_user` → … → `migrations/*` 最后）。手工执行时可按下方脚本顺序，效果一致。

---

## 建表脚本（按执行顺序）

### 1. 序列：`00_sequences.sql`

**说明**：仅显式创建建表时通过 `nextval(...)` 引用的序列；使用 `serial4/bigserial` 的表由 PostgreSQL 自动创建同名序列，无需在此定义。

**显式创建的序列（2 个）：**
- `chat_message_record_read_status_id_seq`
- `group_message_record_read_id_seq`

（其余如 `chat_list_link_id_seq`、`chat_message_record_id_seq`、`friend_request_info_id_seq` 等由 `serial4`/`bigserial` 隐式创建。）

---

### 2. 基础用户表：`basic_user.sql`

**说明**：最基础的用户表。注意：**无外键约束**，其他表对 `basic_user.uuid` 的引用均为逻辑关联。

**主要字段：**
- `uuid` - 主键
- `username` - 用户名
- `account` - 用户编码
- `password` - 用户密码
- `email` - 邮箱
- `info` - 用户信息
- `icon` - 头像

（无 `created_at` / `updated_at` 列。）

---

### 3. 用户相关表

这些表的 `uuid` 逻辑上关联 `basic_user.uuid`（无外键约束）。

#### 1. 用户详细信息表 `user_info.sql`
- `uuid` - 主键（逻辑关联 basic_user.uuid）
- `gender` - 性别
- `age` - 年龄
- `birthday` - 生日
- `phone` - 手机号
- `email` - 邮箱
- `address` - 地址
- `status` - 状态
- `note` - 用户简介
- `created_at` / `updated_at` - 创建/更新时间

#### 2. 用户缓存表 `user_cache.sql`
- `uuid` - 主键（逻辑关联 basic_user.uuid）
- `created_at` - 创建时间
- `updated_at` - 更新时间
- `version` - 版本号

#### 3. 用户登录审计表 `user_login_log.sql`
- `id` - 主键
- `uuid` - 关联 basic_user.uuid（账号不存在时为空，用户删除后置空保留审计）
- `account` - 登录时提交的账号（失败/不存在也保留，便于审计）
- `login_type` - 登录渠道：account / email / refresh
- `event_type` - 事件类型：success / password_fail / account_not_found / refresh
- `login_at` - 事件时间（Unix 时间戳，毫秒）
- `platform` - 登录平台：PC / MOBILE
- `ipv4` / `ipv6` - 客户端 IP 地址
- `user_agent` - 客户端 User-Agent
- `device` - 设备指纹/名称（预留）
- `result` - 结果补充（如刷新失败原因）
- 索引：`(uuid, login_at DESC)`、`(account, login_at DESC)`、`(login_at DESC)`

---

### 4. 文件上传相关表

#### 1. 文件上传记录表 `file_upload_record.sql`
- `id` - 主键
- `uuid` - 文件唯一标识符
- `upload_user_uuid` - 上传用户UUID（逻辑关联 basic_user.uuid）
- `file_path` - 文件路径
- `file_size` - 文件大小
- `mime_type` - 文件MIME类型
- 另有 `original_name`、`stored_name`、`bucket`、`file_hash`、`upload_time`、`status`、`description`、`download_count`、`last_download_time`、`is_oss`、`oss_type`（共 17 列）

#### 2. 文件上传业务表 `biz_record.sql`
- `id` - 主键
- `uuid` - 业务唯一标识符
- `created_by` - 创建用户UUID（逻辑关联 basic_user.uuid）
- `biz_type` - 业务类型
- `file_ids` - 关联的文件UUID
- `preview_file_ids` - 关联的压缩后的文件UUID
- 另有 `biz_name`、`description`、`created_at`、`updated_at`、`status`、`approve_status`、`remark`（共 13 列）

#### 3. 聊天文件上传业务表 `chat_biz_record.sql`
- `id` - 主键
- `uuid` - 业务唯一标识符
- `created_by` - 创建用户UUID（逻辑关联 basic_user.uuid）
- 另有 `biz_name`、`description`、`file_ids`、`preview_file_ids`、`receiver`、`created_at`、`updated_at`、`status`、`approve_status`、`biz_type`、`remark`（共 14 列）

#### 4. 私密文件上传业务表 `private_biz_record.sql`
- `id` - 主键
- `uuid` - 业务唯一标识符
- `created_by` - 创建用户UUID（逻辑关联 basic_user.uuid）
- 另有 `policy_id`、`biz_name`、`description`、`file_ids`、`preview_file_ids`、`created_at`、`updated_at`、`status`、`approve_status`、`biz_type`、`remark`（共 14 列）

#### 5. 业务文件关联表 `biz_file_link.sql`
- `id` - 主键
- `biz_id` - 业务记录ID
- `file_id` - 文件ID
- `origin_file_id` - 原始文件ID
- `is_del` - 是否删除

---

### 5. 好友相关表

#### 1. 好友关系表 `friend_link.sql`
- `uuid` - 主键
- `request_user` - 请求用户（逻辑关联 basic_user.uuid）
- `accept_user` - 接受用户（逻辑关联 basic_user.uuid）
- `is_del` - 是否删除
- `created_at` - 创建时间
- `updated_at` - 更新时间
- `UNIQUE(request_user, accept_user)` - 唯一约束

#### 2. 好友列表缓存表 `friend_list.sql`
- `uuid` - 用户id（逻辑关联 basic_user.uuid）
- `create_at` - 创建时间
- `update_at` - 更新时间
- `version`（uuid 类型）- 版本id，缓存对比
- `last_query` - 上次查询时间

（**无 `friends` 列**；当前也没有对应的实体模型。）

#### 3. 好友请求表 `friend_request_info.sql`
- `id` - 自增主键
- `uuid` - 请求记录UUID
- `request_user` - 请求用户（逻辑关联 basic_user.uuid）
- `accept_user` - 接受用户（逻辑关联 basic_user.uuid）
- 另有 `created_at`、`updated_at`、`request_message`、`add_type`、`accept_message`、`accept_status`

---

### 6. 聊天相关表

#### 1. 聊天列表表 `chat_list_link.sql`
- `id` - 自增主键（`serial4`）
- `uuid` - 用户id（逻辑关联 basic_user.uuid）
- `friend_uuid` - 朋友id（逻辑关联 basic_user.uuid）
- `enable` - 是否显示
- `created_at` - 创建时间

#### 2. 聊天消息记录表 `chat_message_record.sql`
- `id` - 自增id
- `nano_id` - 消息主键
- `send_user` - 发送人id（逻辑关联 basic_user.uuid）
- `recv_user` - 接收用户id（逻辑关联 basic_user.uuid）
- `text_type` - 消息类型
- `timestamp` - 创建时间
- `raw` - 二进制数据

#### 3. 聊天消息失败记录表 `chat_message_record_fail.sql`
- `id` - 自增主键
- `send_user` - 发送者id（逻辑关联 basic_user.uuid）
- `recv_user` - 接收者id（逻辑关联 basic_user.uuid）
- `reason` - 失败原因
- `created_at`（varchar）- 创建时间
- `nano_id` - 消息ID

#### 4. 聊天消息已读状态表 `chat_message_record_read.sql`
- `id` - 自增id
- `send_user` - 发送人id（逻辑关联 basic_user.uuid）
- `recv_user` - 接收人id（逻辑关联 basic_user.uuid）
- `timestamp` - 创建时间
- `nano_id` - 消息ID

---

### 7. 系统通知表：`system_notification.sql`

**说明**：`user_id` 逻辑关联 `basic_user.uuid`。

**主要字段（共 13 列）：**
- `id` - 主键（**uuid 类型**）
- `user_id` - 接收人（逻辑关联 basic_user.uuid）
- `title` - 通知标题
- `content` - 详细内容
- `is_read` - 是否已读
- `priority` - 通知优先级
- 另有 `created_at`、`content_type`、`level1`~`level4`、`unread_count`

---

### 8. 群聊相关表

**文件：** `01_group_tables.sql`（包含 `group_info`、`group_member`、`group_message_record` 三张表；其中 `group_member`、`group_message_record` 有 `REFERENCES group_info(group_uuid)` 外键）

**文件：** `group_invitation.sql`（群邀请表；`group_uuid` 有 `REFERENCES group_info(group_uuid)` 外键）

**文件：** `group_message_record_read.sql`（群消息已读状态表）

**`group_message_record_read` 主要字段：**
- `id` - 自增id
- `nano_id` - 消息主键
- `send_user` - 发送人id
- `group_uuid` - 群组id
- `read_user` - 已读用户id
- `timestamp` - 消息创建时间

---

### 9. 其他脚本

| 文件 | 说明 |
|------|------|
| `fix_file_path_stored_name_inconsistency.sql` | **数据修复脚本**（SELECT/UPDATE），非建表脚本 |
| `migrations/add_email_to_basic_user.sql` | 增量迁移：为 `basic_user` 添加 `email` 列 |
| `migrations/add_bucket_to_file_upload_record.sql` | 增量迁移：为 `file_upload_record` 添加 `bucket` 列 |

---

## 执行方式

### 方式一：自动执行（推荐）

`crates/entity/src/ddl.rs::apply_all_ddl` 按路径字典序自动执行目录内全部 `.sql`（含 `migrations/`），由 `entity` 集成测试（`crates/entity/tests/ddl_integration_test.rs`）验证。无需手工维护顺序。

### 方式二：逐个执行文件（psql）

```bash
# 1. 序列
psql -U username -d database_name -f 00_sequences.sql
psql -U username -d database_name -f 01_group_tables.sql

# 2. 基础用户表
psql -U username -d database_name -f basic_user.sql

# 3. 用户相关表
psql -U username -d database_name -f user_info.sql
psql -U username -d database_name -f user_cache.sql
psql -U username -d database_name -f user_login_log.sql

# 4. 文件上传相关表
psql -U username -d database_name -f file_upload_record.sql
psql -U username -d database_name -f biz_record.sql
psql -U username -d database_name -f chat_biz_record.sql
psql -U username -d database_name -f private_biz_record.sql
psql -U username -d database_name -f biz_file_link.sql

# 5. 好友相关表
psql -U username -d database_name -f friend_link.sql
psql -U username -d database_name -f friend_list.sql
psql -U username -d database_name -f friend_request_info.sql

# 6. 聊天相关表
psql -U username -d database_name -f chat_list_link.sql
psql -U username -d database_name -f chat_message_record.sql
psql -U username -d database_name -f chat_message_record_fail.sql
psql -U username -d database_name -f chat_message_record_read.sql

# 7. 系统通知表
psql -U username -d database_name -f system_notification.sql

# 8. 群聊相关表
psql -U username -d database_name -f group_invitation.sql
psql -U username -d database_name -f group_message_record_read.sql
```

（数据修复与迁移脚本 `fix_file_path_stored_name_inconsistency.sql`、`migrations/*.sql` 按需执行；修复脚本有 SELECT 确认语句，迁移脚本幂等。）

### 方式三：使用 shell 脚本（Linux/Mac）

```bash
#!/bin/bash

DB_USER="username"
DB_NAME="database_name"
DDL_DIR="crates/entity/ddl"

# 按字典序执行目录内全部 SQL（与 apply_all_ddl 行为一致）
for f in $DDL_DIR/*.sql $DDL_DIR/migrations/*.sql; do
    echo "执行: $f"
    psql -U $DB_USER -d $DB_NAME -f "$f"
done

echo "所有表创建完成！"
```

### 方式四：使用 PowerShell 脚本（Windows）

```powershell
# 设置数据库连接信息
$dbUser = "username"
$dbName = "database_name"
$ddlDir = "crates\entity\ddl"

# 按字典序执行目录内全部 SQL（含 migrations）
$sqlFiles = Get-ChildItem -Path $ddlDir -Filter *.sql -Recurse | Sort-Object FullName
foreach ($file in $sqlFiles) {
    Write-Host "正在执行: $($file.FullName)"
    psql -U $dbUser -d $dbName -f $file.FullName
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ $($file.Name) 执行成功" -ForegroundColor Green
    } else {
        Write-Host "✗ $($file.Name) 执行失败" -ForegroundColor Red
        exit 1
    }
}

Write-Host "所有表创建完成！" -ForegroundColor Green
```

---

## 验证表创建

执行完成后，可以使用以下命令验证：

```sql
-- 查看所有表
\dt

-- 查看特定表结构
\d+ table_name

-- 查看表注释
SELECT 
    table_name,
    obj_description((table_schema||'.'||table_name)::regclass, 'pg_class') as table_comment
FROM information_schema.tables
WHERE table_schema = 'public'
ORDER BY table_name;

-- 查看列注释
SELECT 
    table_name,
    column_name,
    col_description((table_schema||'.'||table_name)::regclass::oid, ordinal_position) as column_comment
FROM information_schema.columns
WHERE table_schema = 'public'
ORDER BY table_name, ordinal_position;
```

---

## 注意事项

1. **幂等性**：建表与迁移脚本均使用 `IF NOT EXISTS`，可安全重复执行；`fix_file_path_stored_name_inconsistency.sql` 是数据修复脚本（UPDATE），不在其列
2. **执行顺序**：由文件名前缀（`00_` / `01_` 及字母序）决定；仅群表之间存在外键约束，其余表无外键依赖
3. **数据库连接**：确保在执行前已创建数据库并有足够的权限
4. **备份数据**：在生产环境执行前，请先备份数据
5. **测试环境**：建议先在测试环境验证，确认无误后再在生产环境执行

---

## 依赖关系图

```
00_sequences.sql (序列)
    ↓
01_group_tables.sql (群组信息/成员/群消息表，含 REFERENCES group_info)
    ↓
basic_user.sql (基础用户表)
    ↓
    ├─→ user_info.sql (用户详细信息表)
    ├─→ user_cache.sql (用户缓存表)
    ├─→ user_login_log.sql (用户登录记录表)
    ├─→ file_upload_record.sql (文件上传记录表)
    ├─→ biz_record.sql (文件上传业务表)
    ├─→ chat_biz_record.sql (聊天文件上传业务表)
    ├─→ private_biz_record.sql (私密文件上传业务表)
    ├─→ biz_file_link.sql (业务文件关联表)
    ├─→ friend_link.sql (好友关系表)
    ├─→ friend_list.sql (好友列表缓存表)
    ├─→ friend_request_info.sql (好友请求表)
    ├─→ chat_list_link.sql (聊天列表表)
    ├─→ chat_message_record.sql (聊天消息记录表)
    ├─→ chat_message_record_fail.sql (聊天消息失败记录表)
    ├─→ chat_message_record_read.sql (聊天消息已读状态表)
    ├─→ system_notification.sql (系统通知表)
    ├─→ group_invitation.sql (群邀请表，REFERENCES group_info)
    └─→ group_message_record_read.sql (群消息已读状态表)
```

> 注：除 `01_group_tables.sql` 与 `group_invitation.sql` 指向 `group_info` 的 3 处外键外，所有「用户/文件/聊天/通知」表的 `uuid` 关联均为**逻辑关联（无外键约束）**。实际执行顺序由文件名字典序决定（`migrations/*.sql` 最后）。

---

## 故障排查

### 问题：外键约束错误

**错误信息：**
```
ERROR:  foreign key constraint "xxx" cannot be implemented
DETAIL:  Key columns "xxx" are not of the same data type as referenced key columns "xxx"
```

**解决方案：**
- 先执行 `01_group_tables.sql`（群表外键指向 `group_info`，同文件内创建）
- 检查被引用的表是否已创建

### 问题：序列不存在

**错误信息：**
```
ERROR:  relation "xxx_seq" does not exist
```

**解决方案：**
- 确保先执行 `00_sequences.sql`

### 问题：表已存在

**错误信息：**
```
ERROR:  relation "xxx" already exists
```

**解决方案：**
- 所有建表脚本都使用了 `IF NOT EXISTS`，不应出现此错误
- 如果出现，可能是之前的执行没有使用 `IF NOT EXISTS`
