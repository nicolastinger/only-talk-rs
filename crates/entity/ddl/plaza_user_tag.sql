-- public.plaza_user_tag 表定义

-- 删除表
-- DROP TABLE plaza_user_tag;

CREATE TABLE IF NOT EXISTS plaza_user_tag (
    id uuid NOT NULL, -- 主键
    user_uuid uuid NOT NULL, -- 所属用户
    tag varchar(32) NOT NULL, -- 标签内容
    sort int4 DEFAULT 0 NOT NULL, -- 展示顺序
    created_at int8 NOT NULL, -- 创建时间
    CONSTRAINT plaza_user_tag_pk PRIMARY KEY (id),
    CONSTRAINT plaza_user_tag_unique UNIQUE (user_uuid, tag)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_plaza_user_tag_user ON public.plaza_user_tag USING btree (user_uuid);
CREATE INDEX IF NOT EXISTS idx_plaza_user_tag_tag ON public.plaza_user_tag USING btree (tag);

-- 表注释
COMMENT ON TABLE public.plaza_user_tag IS '用户交友广场标签表';

-- 列注释
COMMENT ON COLUMN public.plaza_user_tag.id IS '主键';
COMMENT ON COLUMN public.plaza_user_tag.user_uuid IS '所属用户';
COMMENT ON COLUMN public.plaza_user_tag.tag IS '标签内容 (1-32字符)';
COMMENT ON COLUMN public.plaza_user_tag.sort IS '展示顺序';
COMMENT ON COLUMN public.plaza_user_tag.created_at IS '创建时间 (Unix时间戳，秒)';
