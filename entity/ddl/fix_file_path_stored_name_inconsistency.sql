-- 修复 file_upload_record 表中 file_path 和 stored_name 不一致的问题
-- 问题描述：某些记录的 file_path 和 stored_name 字段值不同，导致生成预签名 URL 时找不到文件
-- 修复策略：将 stored_name 的值作为正确的 S3 key，更新 file_path 字段
-- 原因：文件去重时应该沿用已存在文件的路径，而不是生成新的 S3 key

-- 查看不一致的记录
SELECT 
    uuid,
    file_path,
    stored_name,
    bucket,
    is_oss,
    created_at
FROM file_upload_record
WHERE is_oss = 1 
  AND file_path IS NOT NULL 
  AND stored_name IS NOT NULL
  AND file_path != stored_name
ORDER BY created_at DESC;

-- 修复不一致的记录（使用 stored_name 作为正确的 S3 key）
-- 注意：执行前请先备份数据库！
UPDATE file_upload_record
SET file_path = stored_name,
    updated_at = NOW()
WHERE is_oss = 1 
  AND file_path IS NOT NULL 
  AND stored_name IS NOT NULL
  AND file_path != stored_name;

-- 验证修复结果
SELECT 
    uuid,
    file_path,
    stored_name,
    bucket,
    is_oss
FROM file_upload_record
WHERE is_oss = 1 
  AND file_path IS NOT NULL 
  AND stored_name IS NOT NULL
  AND file_path != stored_name;
-- 应该返回 0 条记录
