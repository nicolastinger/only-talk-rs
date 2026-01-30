-- public.system_notification table definition

-- Drop table
-- DROP TABLE system_notification;

CREATE TABLE IF NOT EXISTS system_notification (
    id uuid NOT NULL, -- 主键ID
    title varchar(255) NULL, -- 通知标题
    "content" text NULL, -- 详细内容
    created_at int8 NULL, -- 创建时间 (Unix时间戳，毫秒)
    content_type int2 NULL, -- 内容类型，0-纯文本，1-json, 2-xml
    user_id uuid NULL, -- 接收人
    is_read bool NULL, -- 是否已读
    level1 int4 NULL, -- 第一层级，用于定位功能大类
    level2 int4 NULL, -- 第二层级，用于定位子功能模块
    level3 int4 NULL, -- 第三层级，用于定位具体功能组
    level4 int4 NULL, -- 第四层级，用于定位详细功能项
    unread_count int4 NULL, -- 未读数量
    priority int4 NULL, -- 通知优先级
    CONSTRAINT system_notification_pkey PRIMARY KEY (id)
);

-- Indexes
CREATE INDEX idx_system_notification_is_read ON public.system_notification USING btree (is_read);
CREATE INDEX idx_system_notification_user_id_created_at ON public.system_notification USING btree (user_id, created_at);

-- Table comment
COMMENT ON TABLE public.system_notification IS '系统通知表';

-- Column comments
COMMENT ON COLUMN public.system_notification.id IS '主键ID';
COMMENT ON COLUMN public.system_notification.title IS '通知标题';
COMMENT ON COLUMN public.system_notification."content" IS '详细内容';
COMMENT ON COLUMN public.system_notification.created_at IS '创建时间 (Unix时间戳，毫秒)';
COMMENT ON COLUMN public.system_notification.content_type IS '内容类型，0-纯文本，1-json, 2-xml';
COMMENT ON COLUMN public.system_notification.user_id IS '接收人';
COMMENT ON COLUMN public.system_notification.is_read IS '是否已读';
COMMENT ON COLUMN public.system_notification.level1 IS '第一层级，用于定位功能大类';
COMMENT ON COLUMN public.system_notification.level2 IS '第二层级，用于定位子功能模块';
COMMENT ON COLUMN public.system_notification.level3 IS '第三层级，用于定位具体功能组';
COMMENT ON COLUMN public.system_notification.level4 IS '第四层级，用于定位详细功能项';
COMMENT ON COLUMN public.system_notification.unread_count IS '未读数量';
COMMENT ON COLUMN public.system_notification.priority IS '通知优先级';
