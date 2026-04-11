-- Migration: Add bucket column to file_upload_record table
-- Date: 2026-04-11
-- Description: 添加S3存储桶名称字段，用于记录文件实际存储的桶

-- Add bucket column if not exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'file_upload_record' AND column_name = 'bucket'
    ) THEN
        ALTER TABLE file_upload_record ADD COLUMN bucket varchar(255) NULL;
        COMMENT ON COLUMN file_upload_record.bucket IS 'S3存储桶名称（仅S3存储时有效）';
    END IF;
END $$;
