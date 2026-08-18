-- public.biz_file_link 表定义

-- 删除表
-- DROP TABLE biz_file_link;

CREATE TABLE IF NOT EXISTS biz_file_link (
    id bigserial NOT NULL, -- 主键ID
    biz_id uuid NULL, -- 业务唯一标识符
    file_id uuid NULL, -- 文件唯一标识符
    origin_file_id uuid NULL, -- 预览文件唯一标识符
    is_del bool DEFAULT false NOT NULL, -- 是否删除
    CONSTRAINT biz_file_link_pkey PRIMARY KEY (id),
    CONSTRAINT biz_file_link_biz_preview_unique UNIQUE (biz_id, file_id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_biz_file_link_biz_id ON public.biz_file_link USING btree (biz_id);
CREATE INDEX IF NOT EXISTS idx_biz_file_link_file_id ON public.biz_file_link USING btree (file_id);

-- 表注释
COMMENT ON TABLE public.biz_file_link IS '业务文件关联表';

-- 列注释
COMMENT ON COLUMN public.biz_file_link.id IS '主键ID';
COMMENT ON COLUMN public.biz_file_link.biz_id IS '业务唯一标识符';
COMMENT ON COLUMN public.biz_file_link.origin_file_id IS '原文件唯一标识符';
COMMENT ON COLUMN public.biz_file_link.file_id IS '预览文件唯一标识符';
COMMENT ON COLUMN public.biz_file_link.is_del IS '是否删除';
