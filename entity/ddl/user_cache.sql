-- public.user_cache table definition

-- Drop table
-- DROP TABLE user_cache;

CREATE TABLE IF NOT EXISTS user_cache (
    uuid uuid NOT NULL, -- 用户主键
    created_at int8 NOT NULL, -- 创建时间
    updated_at int8 NOT NULL, -- 更新时间
    "text" json NULL, -- 缓存json文本
    "version" int4 NULL, -- 版本
    CONSTRAINT user_cache_pk PRIMARY KEY (uuid)
);

-- Table comment
COMMENT ON TABLE public.user_cache IS '用户缓存表';

-- Column comments
COMMENT ON COLUMN public.user_cache.uuid IS '用户主键';
COMMENT ON COLUMN public.user_cache.created_at IS '创建时间';
COMMENT ON COLUMN public.user_cache.updated_at IS '更新时间';
COMMENT ON COLUMN public.user_cache."text" IS '缓存json文本';
COMMENT ON COLUMN public.user_cache."version" IS '版本';
