-- public.chat_biz_record table definition

-- Drop table
-- DROP TABLE chat_biz_record;

CREATE TABLE IF NOT EXISTS chat_biz_record (
    id bigserial PRIMARY KEY, -- 主键ID
    uuid uuid UNIQUE, -- 业务唯一标识符
    biz_name varchar(255), -- 业务名称
    description text, -- 业务描述
    file_ids text, -- 关联的文件UUID
    preview_file_ids text, -- 关联的压缩后的文件UUID
    created_by uuid, -- 创建用户UUID
    receiver uuid, -- 接收用户UUID
    created_at int8, -- 创建时间（Unix时间戳，毫秒）
    updated_at int8, -- 更新时间（Unix时间戳，毫秒）
    status int4 DEFAULT 0, -- 业务状态（0-正常，1-已禁用，2-已删除）
    approve_status int4 DEFAULT 0, -- 审批状态(0-待处理,1-已通过,2-已拒绝)
    biz_type varchar(100), -- 业务类型(私聊、群聊等)
    remark text -- 备注信息
);

-- Table comment
COMMENT ON TABLE public.chat_biz_record IS '聊天文件上传业务表';

-- Column comments
COMMENT ON COLUMN public.chat_biz_record.id IS '主键ID';
COMMENT ON COLUMN public.chat_biz_record.uuid IS '业务唯一标识符';
COMMENT ON COLUMN public.chat_biz_record.biz_name IS '业务名称';
COMMENT ON COLUMN public.chat_biz_record.description IS '业务描述';
COMMENT ON COLUMN public.chat_biz_record.file_ids IS '关联的文件UUID';
COMMENT ON COLUMN public.chat_biz_record.preview_file_ids IS '关联的压缩后的文件UUID';
COMMENT ON COLUMN public.chat_biz_record.created_by IS '创建用户UUID';
COMMENT ON COLUMN public.chat_biz_record.receiver IS '接收用户UUID';
COMMENT ON COLUMN public.chat_biz_record.created_at IS '创建时间（Unix时间戳，毫秒）';
COMMENT ON COLUMN public.chat_biz_record.updated_at IS '更新时间（Unix时间戳，毫秒）';
COMMENT ON COLUMN public.chat_biz_record.status IS '业务状态（0-正常，1-已禁用，2-已删除）';
COMMENT ON COLUMN public.chat_biz_record.approve_status IS '审批状态(0-待处理,1-已通过,2-已拒绝)';
COMMENT ON COLUMN public.chat_biz_record.biz_type IS '业务类型(私聊、群聊等)';
COMMENT ON COLUMN public.chat_biz_record.remark IS '备注信息';

-- Indexes
CREATE INDEX idx_chat_biz_record_uuid ON public.chat_biz_record USING btree (uuid);
CREATE INDEX idx_chat_biz_record_created_by ON public.chat_biz_record USING btree (created_by);
CREATE INDEX idx_chat_biz_record_receiver ON public.chat_biz_record USING btree (receiver);
CREATE INDEX idx_chat_biz_record_created_at ON public.chat_biz_record USING btree (created_at);
CREATE INDEX idx_chat_biz_record_status ON public.chat_biz_record USING btree (status);
CREATE INDEX idx_chat_biz_record_approve_status ON public.chat_biz_record USING btree (approve_status);
CREATE INDEX idx_chat_biz_record_biz_type ON public.chat_biz_record USING btree (biz_type);
