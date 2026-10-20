-- public.announcement_read 表定义

-- 删除表
-- DROP TABLE announcement_read;

CREATE TABLE IF NOT EXISTS announcement_read (
    id uuid NOT NULL, -- 主键ID
    announcement_uuid uuid NOT NULL, -- 公告key
    user_uuid uuid NOT NULL, -- 已读用户
    created_at int8 NOT NULL, -- 创建时间
    CONSTRAINT announcement_read_pk PRIMARY KEY (id),
    CONSTRAINT announcement_read_unique UNIQUE (announcement_uuid, user_uuid)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_announcement_read_announcement ON public.announcement_read USING btree (announcement_uuid);
CREATE INDEX IF NOT EXISTS idx_announcement_read_user ON public.announcement_read USING btree (user_uuid);

-- 表注释
COMMENT ON TABLE public.announcement_read IS '公告已读记录表';

-- 列注释
COMMENT ON COLUMN public.announcement_read.id IS '主键ID (uuid)';
COMMENT ON COLUMN public.announcement_read.announcement_uuid IS '公告key';
COMMENT ON COLUMN public.announcement_read.user_uuid IS '已读用户uuid';
COMMENT ON COLUMN public.announcement_read.created_at IS '创建时间 (Unix时间戳，秒)';
