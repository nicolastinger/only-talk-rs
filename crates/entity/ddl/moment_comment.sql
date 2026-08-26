-- public.moment_comment 表定义

-- 删除表
-- DROP TABLE moment_comment;

CREATE TABLE IF NOT EXISTS moment_comment (
    id uuid NOT NULL, -- 主键ID
    moment_uuid uuid NOT NULL, -- 动态key
    author_uuid uuid NOT NULL, -- 评论作者
    content text NOT NULL, -- 评论内容
    is_del bool DEFAULT false NOT NULL, -- 是否删除
    created_at int8 NOT NULL, -- 创建时间
    CONSTRAINT moment_comment_pk PRIMARY KEY (id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_moment_comment_moment ON public.moment_comment USING btree (moment_uuid, created_at);

-- 表注释
COMMENT ON TABLE public.moment_comment IS '动态评论表';

-- 列注释
COMMENT ON COLUMN public.moment_comment.id IS '主键ID (uuid)';
COMMENT ON COLUMN public.moment_comment.moment_uuid IS '动态key';
COMMENT ON COLUMN public.moment_comment.author_uuid IS '评论作者uuid';
COMMENT ON COLUMN public.moment_comment.content IS '评论内容';
COMMENT ON COLUMN public.moment_comment.is_del IS '是否删除';
COMMENT ON COLUMN public.moment_comment.created_at IS '创建时间 (Unix时间戳，秒)';
