-- public.file_upload_record table definition

-- Drop table
-- DROP TABLE file_upload_record;

CREATE TABLE IF NOT EXISTS file_upload_record (
    id bigserial NOT NULL, -- 主键ID
    uuid uuid NOT NULL, -- 文件唯一标识符
    original_name varchar(255) NULL, -- 原始文件名
    stored_name varchar(255) NULL, -- 存储文件名
    file_path varchar(512) NOT NULL, -- 文件路径
    bucket varchar(255) NULL, -- S3存储桶名称（仅S3存储时有效）
    file_size int8 NOT NULL, -- 文件大小（字节）
    mime_type varchar(128) NULL, -- 文件MIME类型
    file_hash varchar(128) NULL, -- 文件哈希值（用于去重）
    upload_user_uuid uuid NOT NULL, -- 上传用户UUID
    upload_time int8 NOT NULL, -- 上传时间（Unix时间戳，毫秒）
    status int4 DEFAULT 0 NOT NULL, -- 文件状态（0-正常，1-已删除，2-临时文件）
    description text NULL, -- 文件描述
    download_count int4 DEFAULT 0 NOT NULL, -- 下载次数
    last_download_time int8 NULL, -- 最后下载时间
    is_oss int4 DEFAULT 0 NOT NULL, -- 是否为OSS存储（0-否，1-是）
    oss_type int4 DEFAULT 3 NOT NULL, -- OSS类型（0-阿里云，1-腾讯云，2-亚马逊AWS，3-其他）
    CONSTRAINT file_upload_record_pkey PRIMARY KEY (id)
);

-- Indexes
CREATE INDEX idx_file_upload_record_uuid ON public.file_upload_record USING btree (uuid);
CREATE INDEX idx_file_upload_record_user_uuid ON public.file_upload_record USING btree (upload_user_uuid);
CREATE INDEX idx_file_upload_record_upload_time ON public.file_upload_record USING btree (upload_time);

-- Table comment
COMMENT ON TABLE public.file_upload_record IS '文件上传记录表';

-- Column comments
COMMENT ON COLUMN public.file_upload_record.id IS '主键ID';
COMMENT ON COLUMN public.file_upload_record.uuid IS '文件唯一标识符';
COMMENT ON COLUMN public.file_upload_record.original_name IS '原始文件名';
COMMENT ON COLUMN public.file_upload_record.stored_name IS '存储文件名';
COMMENT ON COLUMN public.file_upload_record.file_path IS '文件路径';
COMMENT ON COLUMN public.file_upload_record.file_size IS '文件大小（字节）';
COMMENT ON COLUMN public.file_upload_record.mime_type IS '文件MIME类型';
COMMENT ON COLUMN public.file_upload_record.file_hash IS '文件哈希值（用于去重）';
COMMENT ON COLUMN public.file_upload_record.upload_user_uuid IS '上传用户UUID';
COMMENT ON COLUMN public.file_upload_record.upload_time IS '上传时间（Unix时间戳，毫秒）';
COMMENT ON COLUMN public.file_upload_record.status IS '文件状态（0-正常，1-已删除，2-临时文件）';
COMMENT ON COLUMN public.file_upload_record.description IS '文件描述';
COMMENT ON COLUMN public.file_upload_record.download_count IS '下载次数';
COMMENT ON COLUMN public.file_upload_record.last_download_time IS '最后下载时间';
COMMENT ON COLUMN public.file_upload_record.is_oss IS '是否为OSS存储（0-否，1-是）';
COMMENT ON COLUMN public.file_upload_record.oss_type IS 'OSS类型（0-阿里云，1-腾讯云，2-亚马逊AWS，3-其他）';
