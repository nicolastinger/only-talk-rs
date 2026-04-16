# 数据库表一键创建脚本使用说明

## 概述

本项目提供了三个一键创建数据库表的脚本，分别适用于不同的操作系统：

- **create_tables.sh** - Linux/Mac 系统（Bash 脚本）
- **create_tables.ps1** - Windows 系统（PowerShell 脚本）
- **create_tables.bat** - Windows 系统（CMD 批处理脚本）

---

## 快速开始

### Linux/Mac 系统

```bash
# 1. 给脚本添加执行权限
chmod +x entity/ddl/create_tables.sh

# 2. 设置环境变量（可选）
export DB_HOST=localhost
export DB_PORT=5432
export DB_NAME=your_database_name
export DB_USER=postgres
export DB_PASSWORD=your_password

# 3. 执行脚本
./entity/ddl/create_tables.sh
```

### Windows 系统（PowerShell）

```powershell
# 1. 设置环境变量（可选）
$env:DB_HOST = "localhost"
$env:DB_PORT = "5432"
$env:DB_NAME = "your_database_name"
$env:DB_USER = "postgres"
$env:DB_PASSWORD = "your_password"

# 2. 执行脚本（可能需要管理员权限）
.\entity\ddl\create_tables.ps1

# 如果遇到执行策略限制，先运行：
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Windows 系统（CMD）

```cmd
REM 1. 设置环境变量（可选）
set DB_HOST=localhost
set DB_PORT=5432
set DB_NAME=your_database_name
set DB_USER=postgres
set DB_PASSWORD=your_password

REM 2. 双击执行或在 CMD 中运行
entity\ddl\create_tables.bat
```

---

## 配置说明

### 环境变量配置

所有脚本都支持通过环境变量配置数据库连接信息：

| 环境变量 | 说明 | 默认值 |
|---------|------|--------|
| `DB_HOST` | 数据库主机地址 | `localhost` |
| `DB_PORT` | 数据库端口 | `5432` |
| `DB_NAME` | 数据库名称 | `your_database_name` |
| `DB_USER` | 数据库用户名 | `postgres` |
| `DB_PASSWORD` | 数据库密码 | （必须设置） |

### 修改脚本配置

你也可以直接修改脚本文件中的配置：

**Linux/Mac (create_tables.sh):**
```bash
# 数据库连接配置
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-your_database_name}"
DB_USER="${DB_USER:-postgres}"
```

**Windows (create_tables.ps1):**
```powershell
# 数据库连接配置
$DB_HOST = if ($env:DB_HOST) { $env:DB_HOST } else { "localhost" }
$DB_PORT = if ($env:DB_PORT) { $env:DB_PORT } else { "5432" }
$DB_NAME = if ($env:DB_NAME) { $env:DB_NAME } else { "your_database_name" }
$DB_USER = if ($env:DB_USER) { $env:DB_USER } else { "postgres" }
```

**Windows (create_tables.bat):**
```cmd
REM 数据库连接配置
if "%DB_HOST%"=="" set DB_HOST=localhost
if "%DB_PORT%"=="" set DB_PORT=5432
if "%DB_NAME%"=="" set DB_NAME=your_database_name
if "%DB_USER%"=="" set DB_USER=postgres
```

---

## 脚本功能

### 主要功能

1. **环境检查**
   - 检查 psql 客户端是否安装
   - 检查数据库连接是否正常
   - 检查所有 SQL 文件是否存在

2. **表创建**
   - 按照正确的依赖关系顺序创建表
   - 显示执行进度和结果
   - 记录详细的执行日志

3. **结果验证**
   - 验证所有表是否创建成功
   - 显示已创建的表列表
   - 统计执行时间和成功率

### 日志文件

所有脚本都会生成日志文件，格式为：`create_tables_YYYYMMDD_HHMMSS.log`

日志文件包含：
- 执行时间戳
- 每个步骤的详细信息
- SQL 执行输出
- 错误信息（如果有）

---

## 执行顺序

脚本会按照以下顺序执行 SQL 文件：

1. `sequences.sql` - 序列定义
2. `basic_user.sql` - 基础用户表
3. `basic_user_salt.sql` - 用户密码盐表
4. `user_info.sql` - 用户详细信息表
5. `user_cache.sql` - 用户缓存表
6. `user_login_log.sql` - 用户登录记录表
7. `file_upload_record.sql` - 文件上传记录表
8. `biz_record.sql` - 文件上传业务表
9. `chat_biz_record.sql` - 聊天文件上传业务表
10. `private_biz_record.sql` - 私密文件上传业务表
11. `friend_link.sql` - 好友关系表
12. `friend_list.sql` - 好友列表缓存表
13. `friend_request_info.sql` - 好友请求表
14. `chat_list_link.sql` - 聊天列表表
15. `chat_message_record.sql` - 聊天消息记录表
16. `chat_message_record_fail.sql` - 聊天消息失败记录表
17. `chat_message_record_read.sql` - 聊天消息已读状态表
18. `system_notification.sql` - 系统通知表

---

## 常见问题

### 1. psql 未安装

**错误信息：**
```
[ERROR] psql 未安装，请先安装 PostgreSQL 客户端
```

**解决方案：**

**Windows:**
- 下载 PostgreSQL 安装包：https://www.postgresql.org/download/windows/
- 安装时确保勾选 "Command Line Tools"

**Linux (Ubuntu/Debian):**
```bash
sudo apt-get update
sudo apt-get install postgresql-client
```

**Linux (CentOS/RHEL):**
```bash
sudo yum install postgresql
```

**Mac (Homebrew):**
```bash
brew install postgresql
```

### 2. 数据库连接失败

**错误信息：**
```
[ERROR] 无法连接到数据库
```

**解决方案：**
- 检查数据库服务是否启动
- 检查主机地址、端口、数据库名称、用户名是否正确
- 检查密码是否正确（通过环境变量 `DB_PASSWORD` 设置）
- 检查防火墙设置
- 检查数据库用户权限

### 3. SQL 文件不存在

**错误信息：**
```
[ERROR] 文件不存在: xxx.sql
```

**解决方案：**
- 确保在正确的目录下执行脚本
- 检查 SQL 文件是否在 `entity/ddl/` 目录下
- 确保所有 SQL 文件都已创建

### 4. PowerShell 执行策略限制

**错误信息：**
```
无法加载文件 create_tables.ps1，因为在此系统上禁止运行脚本
```

**解决方案：**
```powershell
# 临时允许脚本执行
Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process

# 或者为当前用户设置执行策略
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### 5. 外键约束错误

**错误信息：**
```
ERROR:  foreign key constraint "xxx" cannot be implemented
```

**解决方案：**
- 脚本已经按照正确的顺序执行，不应出现此错误
- 如果出现，可能是手动执行了部分 SQL 文件，导致表创建顺序混乱
- 建议删除所有表后重新执行脚本

---

## 高级用法

### 在 CI/CD 中使用

**GitHub Actions 示例：**
```yaml
name: Create Database Tables

on:
  push:
    branches: [ main ]

jobs:
  create-tables:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v2
    
    - name: Set up PostgreSQL
      run: |
        sudo apt-get update
        sudo apt-get install postgresql-client
    
    - name: Create tables
      env:
        DB_HOST: ${{ secrets.DB_HOST }}
        DB_PORT: ${{ secrets.DB_PORT }}
        DB_NAME: ${{ secrets.DB_NAME }}
        DB_USER: ${{ secrets.DB_USER }}
        DB_PASSWORD: ${{ secrets.DB_PASSWORD }}
      run: |
        chmod +x entity/ddl/create_tables.sh
        ./entity/ddl/create_tables.sh
```

### 在 Docker 中使用

**Dockerfile 示例：**
```dockerfile
FROM postgres:latest

# 复制 SQL 文件
COPY entity/ddl/ /docker-entrypoint-initdb.d/

# 设置环境变量
ENV POSTGRES_DB=your_database_name
ENV POSTGRES_USER=postgres
ENV POSTGRES_PASSWORD=your_password
```

### 定时执行

**Linux Cron 任务：**
```bash
# 每天凌晨 2 点执行
0 2 * * * /path/to/create_tables.sh >> /var/log/create_tables.log 2>&1
```

**Windows 任务计划程序：**
1. 打开"任务计划程序"
2. 创建基本任务
3. 设置触发器（如每天执行）
4. 设置操作为启动程序：`powershell.exe`
5. 参数：`-ExecutionPolicy Bypass -File "C:\path\to\create_tables.ps1"`

---

## 故障排查

### 查看日志文件

如果脚本执行失败，请查看日志文件：

```bash
# Linux/Mac
cat entity/ddl/create_tables_*.log

# Windows PowerShell
Get-Content entity\ddl\create_tables_*.log

# Windows CMD
type entity\ddl\create_tables_*.log
```

### 手动验证表创建

```sql
-- 连接到数据库
psql -U username -d database_name

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
```

### 删除所有表（重新开始）

```sql
-- 连接到数据库
psql -U username -d database_name

-- 删除所有表（按依赖关系逆序）
DROP TABLE IF EXISTS system_notification CASCADE;
DROP TABLE IF EXISTS chat_message_record_read CASCADE;
DROP TABLE IF EXISTS chat_message_record_fail CASCADE;
DROP TABLE IF EXISTS chat_message_record CASCADE;
DROP TABLE IF EXISTS chat_list_link CASCADE;
DROP TABLE IF EXISTS friend_request_info CASCADE;
DROP TABLE IF EXISTS friend_list CASCADE;
DROP TABLE IF EXISTS friend_link CASCADE;
DROP TABLE IF EXISTS private_biz_record CASCADE;
DROP TABLE IF EXISTS chat_biz_record CASCADE;
DROP TABLE IF EXISTS biz_record CASCADE;
DROP TABLE IF EXISTS file_upload_record CASCADE;
DROP TABLE IF EXISTS user_login_log CASCADE;
DROP TABLE IF EXISTS user_cache CASCADE;
DROP TABLE IF EXISTS user_info CASCADE;
DROP TABLE IF EXISTS basic_user_salt CASCADE;
DROP TABLE IF EXISTS basic_user CASCADE;

-- 删除序列
DROP SEQUENCE IF EXISTS friend_request_info_id_seq CASCADE;
DROP SEQUENCE IF EXISTS chat_message_record_read_status_id_seq CASCADE;
DROP SEQUENCE IF EXISTS chat_message_record_id_seq CASCADE;
DROP SEQUENCE IF EXISTS chat_message_record_fail_id_seq CASCADE;
DROP SEQUENCE IF EXISTS chat_list_link_id_seq CASCADE;
```

---

## 注意事项

1. **备份数据**：在生产环境执行前，请先备份数据
2. **测试环境**：建议先在测试环境验证，确认无误后再在生产环境执行
3. **权限要求**：确保数据库用户有创建表、序列、索引的权限
4. **幂等性**：所有 SQL 文件都使用了 `IF NOT EXISTS`，可以安全地重复执行
5. **日志文件**：执行完成后，请检查日志文件，确认没有错误

---

## 支持

如果遇到问题，请：

1. 查看日志文件获取详细错误信息
2. 检查数据库连接配置
3. 确认 PostgreSQL 客户端已正确安装
4. 验证数据库用户权限
5. 参考本文档的故障排查部分

---

## 更新日志

- **v1.0.0** - 初始版本
  - 支持 Linux/Mac (Bash)
  - 支持 Windows (PowerShell)
  - 支持 Windows (CMD)
  - 环境检查功能
  - 日志记录功能
  - 表创建验证功能
