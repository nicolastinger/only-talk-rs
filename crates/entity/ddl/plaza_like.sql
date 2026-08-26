-- public.plaza_like 表定义

-- 删除表
-- DROP TABLE plaza_like;

CREATE TABLE IF NOT EXISTS plaza_like (
    id uuid NOT NULL, -- 主键ID
    target_uuid uuid NOT NULL, -- 被心动用户uuid
    user_uuid uuid NOT NULL, -- 心动用户uuid
    is_del bool DEFAULT false NOT NULL, -- 是否删除(取消心动=软删)
    created_at int8 NOT NULL, -- 创建时间
    CONSTRAINT plaza_like_pk PRIMARY KEY (id),
    CONSTRAINT plaza_like_unique UNIQUE (target_uuid, user_uuid)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_plaza_like_target ON public.plaza_like USING btree (target_uuid);
CREATE INDEX IF NOT EXISTS idx_plaza_like_user ON public.plaza_like USING btree (user_uuid);

-- 表注释
COMMENT ON TABLE public.plaza_like IS '交友广场心动表';

-- 列注释
COMMENT ON COLUMN public.plaza_like.id IS '主键ID (uuid)';
COMMENT ON COLUMN public.plaza_like.target_uuid IS '被心动用户uuid';
COMMENT ON COLUMN public.plaza_like.user_uuid IS '心动用户uuid';
COMMENT ON COLUMN public.plaza_like.is_del IS '是否删除(取消心动=软删)';
COMMENT ON COLUMN public.plaza_like.created_at IS '创建时间 (Unix时间戳，秒)';
