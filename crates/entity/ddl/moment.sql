-- public.moment 表定义

-- 删除表
-- DROP TABLE moment;

CREATE TABLE IF NOT EXISTS moment (
    uuid uuid NOT NULL, -- 动态主键(同时也是图片分组 biz_id)
    author_uuid uuid NOT NULL, -- 作者
    content text NOT NULL, -- 文案内容
    visibility int2 DEFAULT 0 NOT NULL, -- 可见性: 0-公开, 1-仅自己
    is_del bool DEFAULT false NOT NULL, -- 是否删除
    created_at int8 NOT NULL, -- 创建时间
    updated_at int8 NOT NULL, -- 更新时间
    CONSTRAINT moment_pk PRIMARY KEY (uuid)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_moment_visibility_created ON public.moment USING btree (visibility, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_moment_author ON public.moment USING btree (author_uuid);

-- 表注释
COMMENT ON TABLE public.moment IS '动态广场动态表';

-- 列注释
COMMENT ON COLUMN public.moment.uuid IS '动态主键(同时也是图片分组 biz_id)';
COMMENT ON COLUMN public.moment.author_uuid IS '作者用户uuid';
COMMENT ON COLUMN public.moment.content IS '文案内容';
COMMENT ON COLUMN public.moment.visibility IS '可见性: 0-公开, 1-仅自己';
COMMENT ON COLUMN public.moment.is_del IS '是否删除';
COMMENT ON COLUMN public.moment.created_at IS '创建时间 (Unix时间戳，秒)';
COMMENT ON COLUMN public.moment.updated_at IS '更新时间 (Unix时间戳，秒)';
