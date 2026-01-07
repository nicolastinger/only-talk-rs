-- 文件上传记录表
CREATE TABLE file_upload_record (
    id BIGSERIAL PRIMARY KEY,                    -- 主键ID
    uuid UUID UNIQUE,                            -- 文件唯一标识符
    original_name VARCHAR(500),                  -- 原始文件名
    stored_name VARCHAR(500),                    -- 存储文件名
    file_path VARCHAR(1000),                     -- 文件路径
    file_size BIGINT,                            -- 文件大小（字节）
    mime_type VARCHAR(100),                      -- 文件MIME类型
    file_hash VARCHAR(100) UNIQUE,               -- 文件哈希值（用于去重）
    upload_user_uuid UUID,                       -- 上传用户UUID
    upload_time BIGINT,                          -- 上传时间（Unix时间戳，毫秒）
    status INTEGER DEFAULT 0,                    -- 文件状态（0-正常，1-已删除，2-临时文件）
    description TEXT,                            -- 文件描述
    download_count INTEGER DEFAULT 0,            -- 下载次数
    last_download_time BIGINT,                   -- 最后下载时间
    is_oss INTEGER DEFAULT 0,                    -- 是否为OSS存储（0-否，1-是）
    oss_type INTEGER                             -- OSS类型（0-阿里云，1-腾讯云，2-亚马逊AWS，3-其他）
);

-- 添加字段注释
COMMENT ON TABLE file_upload_record IS '文件上传记录表';
COMMENT ON COLUMN file_upload_record.id IS '主键ID';
COMMENT ON COLUMN file_upload_record.uuid IS '文件唯一标识符';
COMMENT ON COLUMN file_upload_record.original_name IS '原始文件名';
COMMENT ON COLUMN file_upload_record.stored_name IS '存储文件名';
COMMENT ON COLUMN file_upload_record.file_path IS '文件路径';
COMMENT ON COLUMN file_upload_record.file_size IS '文件大小（字节）';
COMMENT ON COLUMN file_upload_record.mime_type IS '文件MIME类型';
COMMENT ON COLUMN file_upload_record.file_hash IS '文件哈希值（用于去重）';
COMMENT ON COLUMN file_upload_record.upload_user_uuid IS '上传用户UUID';
COMMENT ON COLUMN file_upload_record.upload_time IS '上传时间（Unix时间戳，毫秒）';
COMMENT ON COLUMN file_upload_record.status IS '文件状态（0-正常，1-已删除，2-临时文件）';
COMMENT ON COLUMN file_upload_record.description IS '文件描述';
COMMENT ON COLUMN file_upload_record.download_count IS '下载次数';
COMMENT ON COLUMN file_upload_record.last_download_time IS '最后下载时间';
COMMENT ON COLUMN file_upload_record.is_oss IS '是否为OSS存储（0-否，1-是）';
COMMENT ON COLUMN file_upload_record.oss_type IS 'OSS类型（0-阿里云，1-腾讯云，2-亚马逊AWS，3-其他）';

-- 创建索引以提高查询性能
CREATE INDEX idx_file_upload_record_uuid ON file_upload_record (uuid);
CREATE INDEX idx_file_upload_record_upload_user_uuid ON file_upload_record (upload_user_uuid);
CREATE INDEX idx_file_upload_record_file_hash ON file_upload_record (file_hash);
CREATE INDEX idx_file_upload_record_upload_time ON file_upload_record (upload_time);
CREATE INDEX idx_file_upload_record_status ON file_upload_record (status);
