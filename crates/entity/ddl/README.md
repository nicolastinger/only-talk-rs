# 数据库表创建顺序说明

## 概述

本文档说明了数据库表的创建顺序，确保按照正确的依赖关系依次创建表。

## 创建顺序

### 第一阶段：序列（SEQUENCE）

**文件：** `sequences.sql`

**说明：** 序列必须在表之前创建，因为某些表的字段依赖序列。

**包含的序列：**
- `chat_list_link_id_seq`
- `chat_message_record_fail_id_seq`
- `chat_message_record_id_seq`
- `chat_message_record_read_status_id_seq`
- `friend_request_info_id_seq`
- `group_message_record_read_id_seq`

---

### 第二阶段：基础用户表

**文件：** `basic_user.sql`

**说明：** 这是最基础的用户表，其他表都依赖它的 `uuid` 字段。

**主要字段：**
- `uuid` - 主键
- `username` - 用户名
- `account` - 用户编码
- `password` - 用户密码
- `created_at` - 创建时间
- `updated_at` - 更新时间

---

### 第三阶段：用户相关表

这些表都依赖 `basic_user` 表的 `uuid` 字段。

#### 1. 用户详细信息表
**文件：** `user_info.sql`
- `uuid` - 主键（外键，关联 basic_user.uuid）
- `gender` - 性别
- `age` - 年龄
- `birthday` - 生日
- `phone` - 手机号
- `email` - 邮箱
- `address` - 地址
- `status` - 状态
- `note` - 用户简介

#### 2. 用户缓存表
**文件：** `user_cache.sql`
- `uuid` - 主键（外键，关联 basic_user.uuid）
- `text` - 缓存 json 文本

#### 3. 用户登录记录表
**文件：** `user_login_log.sql`
- `id` - 主键
- `last_login_at` - 最后登录时间
- `last_login_equipment` - 最后登录的设备
- `last_login_ipv4` - 最后登录的IPv4地址
- `last_login_ipv6` - 最后登录的IPv6地址

---

### 第四阶段：文件上传相关表

这些表都依赖 `basic_user` 表的 `uuid` 字段。

#### 1. 文件上传记录表
**文件：** `file_upload_record.sql`
- `id` - 主键
- `uuid` - 文件唯一标识符
- `upload_user_uuid` - 上传用户UUID（外键，关联 basic_user.uuid）
- `file_path` - 文件路径
- `file_size` - 文件大小
- `mime_type` - 文件MIME类型

#### 2. 文件上传业务表
**文件：** `biz_record.sql`
- `id` - 主键
- `uuid` - 业务唯一标识符
- `created_by` - 创建用户UUID（外键，关联 basic_user.uuid）
- `biz_type` - 业务类型
- `file_ids` - 关联的文件UUID
- `preview_file_ids` - 关联的压缩后的文件UUID

#### 3. 聊天文件上传业务表
**文件：** `chat_biz_record.sql`
- `id` - 主键
- `uuid` - 业务唯一标识符
- `created_by` - 创建用户UUID（外键，关联 basic_user.uuid）

#### 4. 私密文件上传业务表
**文件：** `private_biz_record.sql`
- `id` - 主键
- `uuid` - 业务唯一标识符
- `created_by` - 创建用户UUID（外键，关联 basic_user.uuid）

---

### 第五阶段：好友相关表

这些表都依赖 `basic_user` 表的 `uuid` 字段。

#### 1. 好友关系表
**文件：** `friend_link.sql`
- `uuid` - 主键
- `request_user` - 请求用户（外键，关联 basic_user.uuid）
- `accept_user` - 接受用户（外键，关联 basic_user.uuid）

#### 2. 好友列表缓存表
**文件：** `friend_list.sql`
- `uuid` - 用户id（外键，关联 basic_user.uuid）
- `friends` - 好友列表（JSON数组）

#### 3. 好友请求表
**文件：** `friend_request_info.sql`
- `uuid` - 主键
- `request_user` - 请求用户（外键，关联 basic_user.uuid）
- `accept_user` - 接受用户（外键，关联 basic_user.uuid）

---

### 第六阶段：聊天相关表

这些表都依赖 `basic_user` 表的 `uuid` 字段。

#### 1. 聊天列表表
**文件：** `chat_list_link.sql`
- `uuid` - 用户id（外键，关联 basic_user.uuid）
- `friend_uuid` - 朋友id（外键，关联 basic_user.uuid）

#### 2. 聊天消息记录表
**文件：** `chat_message_record.sql`
- `id` - 自增id
- `nano_id` - 消息主键
- `send_user` - 发送人id（外键，关联 basic_user.uuid）
- `recv_user` - 接收用户id（外键，关联 basic_user.uuid）
- `text_type` - 消息类型
- `timestamp` - 创建时间
- `raw` - 二进制数据

#### 3. 聊天消息失败记录表
**文件：** `chat_message_record_fail.sql`
- `id` - 自增主键
- `send_user` - 发送者id（外键，关联 basic_user.uuid）
- `recv_user` - 接收者id（外键，关联 basic_user.uuid）
- `reason` - 失败原因

#### 4. 聊天消息已读状态表
**文件：** `chat_message_record_read.sql`
- `id` - 自增id
- `send_user` - 发送人id（外键，关联 basic_user.uuid）
- `recv_user` - 接收人id（外键，关联 basic_user.uuid）
- `timestamp` - 创建时间

---

### 第七阶段：系统通知表

**文件：** `system_notification.sql`

**说明：** 系统通知表依赖 `basic_user` 表的 `uuid` 字段。

**主要字段：**
- `id` - 主键
- `user_id` - 接收人（外键，关联 basic_user.uuid）
- `title` - 通知标题
- `content` - 详细内容
- `is_read` - 是否已读
- `priority` - 通知优先级

---

### 第八阶段：群聊相关表

**文件：** `group_tables.sql`（包含 `group_info`、`group_member`、`group_message_record`）

**说明：** 群聊相关表，包含群组信息、成员列表和消息记录。

**文件：** `group_invitation.sql`（群邀请表）

**文件：** `group_message_record_read.sql`（群消息已读状态表）

**主要字段：**
- `id` - 自增id
- `nano_id` - 消息主键
- `send_user` - 发送人id
- `group_uuid` - 群组id
- `read_user` - 已读用户id
- `timestamp` - 消息创建时间

---

## 执行方式

### 方式一：使用 psql 命令行

```bash
# 连接到数据库
psql -U username -d database_name

# 执行创建脚本
\i 00_create_all_tables.sql
```

### 方式二：逐个执行文件

```bash
# 1. 创建序列
psql -U username -d database_name -f sequences.sql

# 2. 创建基础用户表
psql -U username -d database_name -f basic_user.sql

# 3. 创建用户相关表
psql -U username -d database_name -f user_info.sql
psql -U username -d database_name -f user_cache.sql
psql -U username -d database_name -f user_login_log.sql

# 4. 创建文件上传相关表
psql -U username -d database_name -f file_upload_record.sql
psql -U username -d database_name -f biz_record.sql
psql -U username -d database_name -f chat_biz_record.sql
psql -U username -d database_name -f private_biz_record.sql

# 5. 创建好友相关表
psql -U username -d database_name -f friend_link.sql
psql -U username -d database_name -f friend_list.sql
psql -U username -d database_name -f friend_request_info.sql

# 6. 创建聊天相关表
psql -U username -d database_name -f chat_list_link.sql
psql -U username -d database_name -f chat_message_record.sql
psql -U username -d database_name -f chat_message_record_fail.sql
psql -U username -d database_name -f chat_message_record_read.sql

# 7. 创建系统通知表
psql -U username -d database_name -f system_notification.sql

# 8. 创建群聊相关表
psql -U username -d database_name -f group_tables.sql
psql -U username -d database_name -f group_invitation.sql
psql -U username -d database_name -f group_message_record_read.sql
```

### 方式三：使用 shell 脚本（Linux/Mac）

```bash
#!/bin/bash

DB_USER="username"
DB_NAME="database_name"
DDL_DIR="entity/ddl"

# 按顺序执行 SQL 文件
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/sequences.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/basic_user.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/user_info.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/user_cache.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/user_login_log.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/file_upload_record.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/biz_record.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/chat_biz_record.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/private_biz_record.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/friend_link.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/friend_list.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/friend_request_info.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/chat_list_link.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/chat_message_record.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/chat_message_record_fail.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/chat_message_record_read.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/system_notification.sql

# 8. 创建群聊相关表
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/group_tables.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/group_invitation.sql
psql -U $DB_USER -d $DB_NAME -f $DDL_DIR/group_message_record_read.sql

echo "所有表创建完成！"
```

### 方式四：使用 PowerShell 脚本（Windows）

```powershell
# 设置数据库连接信息
$dbUser = "username"
$dbName = "database_name"
$ddlDir = "entity\ddl"

# 定义 SQL 文件执行顺序
$sqlFiles = @(
    "sequences.sql",
    "basic_user.sql",
    "user_info.sql",
    "user_cache.sql",
    "user_login_log.sql",
    "file_upload_record.sql",
    "biz_record.sql",
    "chat_biz_record.sql",
    "private_biz_record.sql",
    "friend_link.sql",
    "friend_list.sql",
    "friend_request_info.sql",
    "chat_list_link.sql",
    "chat_message_record.sql",
    "chat_message_record_fail.sql",
    "chat_message_record_read.sql",
    "system_notification.sql",
    "group_tables.sql",
    "group_invitation.sql",
    "group_message_record_read.sql"
)

# 逐个执行 SQL 文件
foreach ($file in $sqlFiles) {
    $filePath = Join-Path $ddlDir $file
    Write-Host "正在执行: $file"
    psql -U $dbUser -d $dbName -f $filePath
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ $file 执行成功" -ForegroundColor Green
    } else {
        Write-Host "✗ $file 执行失败" -ForegroundColor Red
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

1. **幂等性**：所有 SQL 文件都使用了 `IF NOT EXISTS`，可以安全地重复执行
2. **依赖关系**：必须按照上述顺序执行，否则会因为外键约束而失败
3. **数据库连接**：确保在执行前已创建数据库并有足够的权限
4. **备份数据**：在生产环境执行前，请先备份数据
5. **测试环境**：建议先在测试环境验证，确认无误后再在生产环境执行

---

## 依赖关系图

```
sequences.sql (序列)
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
    ├─→ friend_link.sql (好友关系表)
    ├─→ friend_list.sql (好友列表缓存表)
    ├─→ friend_request_info.sql (好友请求表)
    ├─→ chat_list_link.sql (聊天列表表)
    ├─→ chat_message_record.sql (聊天消息记录表)
    ├─→ chat_message_record_fail.sql (聊天消息失败记录表)
    ├─→ chat_message_record_read.sql (聊天消息已读状态表)
    ├─→ system_notification.sql (系统通知表)
    ├─→ group_tables.sql (群组信息/成员/消息表)
    ├─→ group_invitation.sql (群邀请表)
    └─→ group_message_record_read.sql (群消息已读状态表)
```

---

## 故障排查

### 问题：外键约束错误

**错误信息：**
```
ERROR:  foreign key constraint "xxx" cannot be implemented
DETAIL:  Key columns "xxx" are not of the same data type as referenced key columns "xxx"
```

**解决方案：**
- 确保按照正确的顺序执行 SQL 文件
- 检查被引用的表是否已创建

### 问题：序列不存在

**错误信息：**
```
ERROR:  relation "xxx_seq" does not exist
```

**解决方案：**
- 确保先执行 `sequences.sql`

### 问题：表已存在

**错误信息：**
```
ERROR:  relation "xxx" already exists
```

**解决方案：**
- 所有 SQL 文件都使用了 `IF NOT EXISTS`，不应出现此错误
- 如果出现，可能是之前的执行没有使用 `IF NOT EXISTS`
