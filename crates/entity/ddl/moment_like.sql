-- public.moment_like 表定义

-- 删除表
-- DROP TABLE moment_like;

CREATE TABLE IF NOT EXISTS moment_like (
    id uuid NOT NULL, -- 主键ID
    moment_uuid uuid NOT NULL, -- 动态key
    user_uuid uuid NOT NULL, -- 点赞用户
    is_del bool DEFAULT false NOT NULL, -- 是否删除(取消点赞=软删)
    created_at int8 NOT NULL, -- 创建时间
    CONSTRAINT moment_like_pk PRIMARY KEY (id),
    CONSTRAINT moment_like_unique UNIQUE (moment_uuid, user_uuid)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_moment_like_moment ON public.moment_like USING btree (moment_uuid);
CREATE INDEX IF NOT EXISTS idx_moment_like_user ON public.moment_like USING btree (user_uuid);

-- 表注释
COMMENT ON TABLE public.moment_like IS '动态点赞表';

-- 列注释
COMMENT ON COLUMN public.moment_like.id IS '主键ID (uuid)';
COMMENT ON COLUMN public.moment_like.moment_uuid IS '动态key';
COMMENT ON COLUMN public.moment_like.user_uuid IS '点赞用户uuid';
COMMENT ON COLUMN public.moment_like.is_del IS '是否删除(取消点赞=软删)';
COMMENT ON COLUMN public.moment_like.created_at IS '创建时间 (Unix时间戳，秒)';
