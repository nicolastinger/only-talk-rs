-- public.user_follow 表定义 (动态广场关注)

-- 删除表
-- DROP TABLE user_follow;

CREATE TABLE IF NOT EXISTS user_follow (
    id uuid NOT NULL, -- 主键ID
    follow_user_uuid uuid NOT NULL, -- 关注者uuid
    target_user_uuid uuid NOT NULL, -- 被关注者uuid
    is_del bool DEFAULT false NOT NULL, -- 是否删除(取消关注=软删)
    created_at int8 NOT NULL, -- 创建时间
    CONSTRAINT user_follow_pk PRIMARY KEY (id),
    CONSTRAINT user_follow_unique UNIQUE (follow_user_uuid, target_user_uuid)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_user_follow_follow ON public.user_follow USING btree (follow_user_uuid);
CREATE INDEX IF NOT EXISTS idx_user_follow_target ON public.user_follow USING btree (target_user_uuid);

-- 表注释
COMMENT ON TABLE public.user_follow IS '动态广场用户关注表';

-- 列注释
COMMENT ON COLUMN public.user_follow.id IS '主键ID';
COMMENT ON COLUMN public.user_follow.follow_user_uuid IS '关注者用户uuid';
COMMENT ON COLUMN public.user_follow.target_user_uuid IS '被关注者用户uuid';
COMMENT ON COLUMN public.user_follow.is_del IS '是否删除(取消关注=软删)';
COMMENT ON COLUMN public.user_follow.created_at IS '创建时间 (Unix时间戳，秒)';
