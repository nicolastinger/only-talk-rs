-- public.system_notification 表定义

-- 删除表
-- DROP TABLE system_notification;

CREATE TABLE IF NOT EXISTS system_notification (
    id uuid NOT NULL, -- 主键ID
    title varchar(255) NULL, -- 通知标题
    "content" text NULL, -- 详细内容
    created_at int8 NULL, -- 创建时间 (Unix时间戳，毫秒)
    content_type int2 NULL, -- 内容类型，0-纯文本，1-json, 2-xml
    user_id uuid NULL, -- 接收人
    is_read bool NULL, -- 是否已读
    biz_id varchar NULL, -- 业务ID（如好友请求/群邀请记录ID）
    level1 int4 NULL, -- 第一层级，用于定位功能大类
    level2 int4 NULL, -- 第二层级，用于定位子功能模块
    level3 int4 NULL, -- 第三层级，用于定位具体功能组
    level4 int4 NULL, -- 第四层级，用于定位详细功能项
    unread_count int4 NULL, -- 未读数量
    priority int4 NULL, -- 通知优先级
    CONSTRAINT system_notification_pkey PRIMARY KEY (id)
);

-- 兼容旧库: biz_id 列在 52a05c9 引入, 此前创建的库无此列;
-- CREATE TABLE IF NOT EXISTS 对已存在表静默跳过, 必须幂等补列后再执行列注释。
-- 幂等: 列已存在时跳过, 可安全重复执行。
ALTER TABLE system_notification ADD COLUMN IF NOT EXISTS biz_id varchar NULL;

-- 索引(任务13 通知中心优化):
-- 列表/未读窗口查询形态: `user_id=? AND is_read=? AND created_at>=? ORDER BY created_at DESC, id DESC`,
-- 复合索引覆盖过滤 + 排序 + keyset 游标,避免回表排序。
CREATE INDEX IF NOT EXISTS idx_system_notification_user_read_created
    ON public.system_notification USING btree (user_id, is_read, created_at DESC, id DESC);
-- 全量列表(不按 is_read 过滤)按时间倒序
CREATE INDEX IF NOT EXISTS idx_system_notification_user_created
    ON public.system_notification USING btree (user_id, created_at DESC, id DESC);

-- 旧索引清理(任务13): is_read 单列选择性极低,无查询价值; (user_id, created_at) 升序索引被上面的降序索引覆盖。
DROP INDEX IF EXISTS idx_system_notification_is_read;
DROP INDEX IF EXISTS idx_system_notification_user_id_created_at;

-- 表注释
COMMENT ON TABLE public.system_notification IS '系统通知表';

-- 列注释
COMMENT ON COLUMN public.system_notification.id IS '主键ID';
COMMENT ON COLUMN public.system_notification.title IS '通知标题';
COMMENT ON COLUMN public.system_notification."content" IS '详细内容';
COMMENT ON COLUMN public.system_notification.created_at IS '创建时间 (Unix时间戳，毫秒)';
COMMENT ON COLUMN public.system_notification.content_type IS '内容类型，0-纯文本，1-json, 2-xml';
COMMENT ON COLUMN public.system_notification.user_id IS '接收人';
COMMENT ON COLUMN public.system_notification.is_read IS '是否已读';
COMMENT ON COLUMN public.system_notification.biz_id IS '业务ID（如好友请求/群邀请记录ID）';
COMMENT ON COLUMN public.system_notification.level1 IS '第一层级，用于定位功能大类';
COMMENT ON COLUMN public.system_notification.level2 IS '第二层级，用于定位子功能模块';
COMMENT ON COLUMN public.system_notification.level3 IS '第三层级，用于定位具体功能组';
COMMENT ON COLUMN public.system_notification.level4 IS '第四层级，用于定位详细功能项';
COMMENT ON COLUMN public.system_notification.unread_count IS '未读数量';
COMMENT ON COLUMN public.system_notification.priority IS '通知优先级';
