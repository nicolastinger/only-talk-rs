# 文件上传 file_path 和 stored_name 不一致问题修复

## 问题描述

在上传聊天文件到 S3 时，数据库 `file_upload_record` 表中的 `file_path` 和 `stored_name` 字段值不一致，导致生成预签名下载 URL 时出现 `NoSuchKey` 错误。

### 具体表现

- 数据库 `file_path` 值：`019b0d88/2026/04/25/abbf65c1-a414-494c-ad18-d45cc6ef67a8.webp`
- 数据库 `stored_name` 值：`019b0d88/2026/04/25/ffeaad94-3f90-43c2-b3e9-2aaddd7c070c.webp`
- S3 实际存储的文件：`ffeaad94-3f90-43c2-b3e9-2aaddd7c070c.webp`（与 stored_name 一致）
- 生成的临时链接使用 `file_path`，导致找不到文件

### 错误信息

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Error>
  <Code>NoSuchKey</Code>
  <Message>The specified key does not exist.</Message>
  <Key>019b0d88/2026/04/25/abbf65c1-a414-494c-ad18-d45cc6ef67a8.webp</Key>
  <BucketName>chat-file-preview</BucketName>
</Error>
```

## 问题根源

在文件去重逻辑中，当检测到相同文件（基于 `file_size`、`file_hash` 和 `bucket`）已存在时：

### 错误的逻辑（修复前）

1. 如果旧文件是本地存储（`is_oss == 0`），会**生成新的 `s3_key`**（包含新生成的 UUID）上传到 S3
2. 更新数据库记录时，将 `file_path` 和 `stored_name` 都设置为新的 `s3_key`
3. 但这导致：**实际 S3 中存储的文件 key 与数据库记录不一致**

### 正确的逻辑（修复后）

1. **文件已存在且是 S3 存储**：直接复用已存在的记录，不做任何修改
2. **文件已存在但是本地存储**：使用已存在记录的 `stored_name` 作为 S3 key 上传，保持 `file_path` 和 `stored_name` 不变
3. **文件不存在**：生成新的 `s3_key`，同时设置 `file_path` 和 `stored_name`

### 受影响的文件

1. `backend/http_service/src/http_service/file_service/service/chat_s3_service.rs`
2. `backend/http_service/src/http_service/file_service/service/avatar_s3_service.rs`
3. `backend/http_service/src/http_service/file_service/service/s3_file_service.rs`

## 修复方案

### 代码修复

文件去重时的正确处理逻辑：

```rust
if !file_upload_record_exist.is_empty() {
    let exist_record = file_upload_record_exist[0].clone();
    let exist_file_path = exist_record.file_path.clone().ok_or(anyhow!("文件路径为空"))?;

    // 已有文件，如果原来存本地则重新上传S3，原来就S3则直接复用
    if exist_record.is_oss.unwrap_or(0) == 0 {
        // 原来是本地文件，上传到S3（使用已有的 stored_name 作为 S3 key）
        let s3_key = exist_record.stored_name.clone().ok_or(anyhow!("stored_name为空"))?;
        let local_data = tokio::fs::read(&exist_file_path).await?;
        let _ = storage.upload(&s3_key, local_data, mime_type.as_deref()).await?;
        
        let mut file_record = exist_record.clone();
        file_record.is_oss = Some(1);
        file_record.oss_type = Some(oss_type);
        // file_path 和 stored_name 保持不变
        FileUploadRecord::update_by_map(rb, &file_record, value! {"uuid": &file_record.uuid}).await?;
        return Ok(file_record);
    }

    // 原来就是S3文件，直接复用
    Ok(exist_record)
}
```

### 已修复的文件

- ✅ `chat_s3_service.rs` - 聊天文件上传
  - 文件已存在且是 S3：直接返回已存在记录
  - 文件已存在但是本地：使用已有的 `stored_name` 上传到 S3，更新 `is_oss` 和 `oss_type`
  
- ✅ `avatar_s3_service.rs` - 头像文件上传
  - 同上逻辑
  
- ✅ `s3_file_service.rs` - 普通文件上传
  - 同上逻辑

### 数据修复

对于已经存在数据库中的错误记录，执行以下 SQL 脚本进行修复：

```sql
-- 修复 file_upload_record 表中 file_path 和 stored_name 不一致的问题
UPDATE file_upload_record
SET file_path = stored_name,
    updated_at = NOW()
WHERE is_oss = 1 
  AND file_path IS NOT NULL 
  AND stored_name IS NOT NULL
  AND file_path != stored_name;
```

完整的 SQL 脚本见：`backend/entity/ddl/fix_file_path_stored_name_inconsistency.sql`

## 验证方法

### 1. 检查数据库中是否还有不一致的记录

```sql
SELECT uuid, file_path, stored_name, bucket, is_oss
FROM file_upload_record
WHERE is_oss = 1 
  AND file_path IS NOT NULL 
  AND stored_name IS NOT NULL
  AND file_path != stored_name;
```

应该返回 0 条记录。

### 2. 测试文件上传和下载

1. 上传一个新文件
2. 检查数据库中 `file_path` 和 `stored_name` 是否一致
3. 生成预签名下载 URL
4. 使用该 URL 下载文件，确认可以正常访问

## 预防措施

1. **代码审查**：确保所有更新 `file_path` 的地方也同时更新 `stored_name`
2. **数据库约束**：考虑添加 CHECK 约束，确保 `file_path = stored_name`（对于 S3 文件）
3. **单元测试**：添加测试用例，验证文件去重逻辑正确更新两个字段

## 影响范围

- **影响版本**：所有在修复前部署的版本
- **影响功能**：S3 文件上传（聊天文件、头像文件、普通文件）的文件去重场景
- **数据影响**：
  - 文件已存在且是 S3 存储：之前会生成新的 S3 key 并更新数据库，导致 S3 中实际不存在该 key
  - 需要通过 SQL 脚本修复已有的错误数据（将 `file_path` 改回 `stored_name` 的值）

## 后续建议

1. 执行数据修复脚本前，**务必备份数据库**
2. 修复后，测试所有文件上传和下载功能
3. 考虑添加监控告警，检测 `file_path` 和 `stored_name` 不一致的情况
4. 在代码中添加注释，说明文件去重时必须沿用已有记录的路径
5. 添加单元测试覆盖文件去重的各种场景：
   - 文件不存在 → 新上传
   - 文件已存在且是 S3 → 直接复用
   - 文件已存在但是本地 → 迁移到 S3
