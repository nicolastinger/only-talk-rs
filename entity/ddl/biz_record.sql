-- 文件上传业务表
CREATE TABLE biz_record (
    id BIGSERIAL PRIMARY KEY,                    -- 主键ID
    uuid UUID UNIQUE,                            -- 业务唯一标识符
    biz_name VARCHAR(255),                       -- 业务名称
    description TEXT,                            -- 业务描述
    file_ids TEXT,                               -- 关联的文件UUID
    created_by UUID,                             -- 创建用户UUID
    created_at BIGINT,                           -- 创建时间（Unix时间戳，毫秒）
    updated_at BIGINT,                           -- 更新时间（Unix时间戳，毫秒）
    status INTEGER DEFAULT 0,                    -- 业务状态（0-正常，1-已禁用，2-已删除）
    approve_status INTEGER DEFAULT 0,            -- 审批状态(0-待处理,1-已通过,2-已拒绝)
    biz_type VARCHAR(100),                       -- 业务类型(头像、身份证、用户背景)
    remark TEXT                                  -- 备注信息
);

-- 添加字段注释
COMMENT ON TABLE biz_record IS '文件上传业务表';
COMMENT ON COLUMN biz_record.id IS '主键ID';
COMMENT ON COLUMN biz_record.uuid IS '业务唯一标识符';
COMMENT ON COLUMN biz_record.biz_name IS '业务名称';
COMMENT ON COLUMN biz_record.description IS '业务描述';
COMMENT ON COLUMN biz_record.file_ids IS '关联的文件UUID';
COMMENT ON COLUMN biz_record.created_by IS '创建用户UUID';
COMMENT ON COLUMN biz_record.created_at IS '创建时间（Unix时间戳，毫秒）';
COMMENT ON COLUMN biz_record.updated_at IS '更新时间（Unix时间戳，毫秒）';
COMMENT ON COLUMN biz_record.status IS '业务状态（0-正常，1-已禁用，2-已删除）';
COMMENT ON COLUMN biz_record.approve_status IS '审批状态(0-待处理,1-已通过,2-已拒绝)';
COMMENT ON COLUMN biz_record.biz_type IS '业务类型(头像、身份证、用户背景)';
COMMENT ON COLUMN biz_record.remark IS '备注信息';

-- 创建索引以提高查询性能
CREATE INDEX idx_biz_record_uuid ON biz_record (uuid);
CREATE INDEX idx_biz_record_created_by ON biz_record (created_by);
CREATE INDEX idx_biz_record_created_at ON biz_record (created_at);
CREATE INDEX idx_biz_record_status ON biz_record (status);
CREATE INDEX idx_biz_record_approve_status ON biz_record (approve_status);
CREATE INDEX idx_biz_record_biz_type ON biz_record (biz_type);