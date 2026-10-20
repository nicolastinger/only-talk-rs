-- public.announcement 表定义

-- 删除表
-- DROP TABLE announcement;

CREATE TABLE IF NOT EXISTS announcement (
    uuid uuid NOT NULL, -- 公告主键
    title text NOT NULL, -- 公告标题
    content text NOT NULL, -- 公告内容 (markdown/html)
    content_type int2 DEFAULT 0 NOT NULL, -- 内容类型: 0-markdown, 1-html
    start_at int8 NOT NULL, -- 展示开始时间
    end_at int8 NOT NULL, -- 展示结束时间
    is_active bool DEFAULT true NOT NULL, -- 是否启用
    sort_order int2 DEFAULT 0 NOT NULL, -- 排序权重(越小越靠前，同窗口内轮换顺序)
    is_del bool DEFAULT false NOT NULL, -- 是否删除
    created_at int8 NOT NULL, -- 创建时间
    updated_at int8 NOT NULL, -- 更新时间
    CONSTRAINT announcement_pk PRIMARY KEY (uuid)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_announcement_active_window ON public.announcement USING btree (is_active, start_at, end_at);
CREATE INDEX IF NOT EXISTS idx_announcement_sort_created ON public.announcement USING btree (sort_order, created_at DESC);

-- 表注释
COMMENT ON TABLE public.announcement IS '公共公告表';

-- 列注释
COMMENT ON COLUMN public.announcement.uuid IS '公告主键 (uuid)';
COMMENT ON COLUMN public.announcement.title IS '公告标题';
COMMENT ON COLUMN public.announcement.content IS '公告内容 (markdown/html)';
COMMENT ON COLUMN public.announcement.content_type IS '内容类型: 0-markdown, 1-html';
COMMENT ON COLUMN public.announcement.start_at IS '展示开始时间 (Unix时间戳，秒)';
COMMENT ON COLUMN public.announcement.end_at IS '展示结束时间 (Unix时间戳，秒)';
COMMENT ON COLUMN public.announcement.is_active IS '是否启用';
COMMENT ON COLUMN public.announcement.sort_order IS '排序权重(越小越靠前，同窗口内轮换顺序)';
COMMENT ON COLUMN public.announcement.is_del IS '是否删除';
COMMENT ON COLUMN public.announcement.created_at IS '创建时间 (Unix时间戳，秒)';
COMMENT ON COLUMN public.announcement.updated_at IS '更新时间 (Unix时间戳，秒)';
