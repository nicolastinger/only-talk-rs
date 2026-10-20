-- public.black_list 表定义

-- 删除表
-- DROP TABLE black_list;

CREATE TABLE IF NOT EXISTS black_list (
    uuid uuid NOT NULL,
    me_user uuid NOT NULL,
    block_user uuid NOT NULL,
    created_at int8 NOT NULL, -- 创建时间
    "version" int4 DEFAULT 0 NOT NULL,
    CONSTRAINT black_list_pk PRIMARY KEY (uuid),
    CONSTRAINT black_list_unique UNIQUE (me_user, block_user)
);

-- 表注释
COMMENT ON TABLE public.black_list IS '黑名单表';

-- 列注释
COMMENT ON COLUMN public.black_list.uuid IS '主键';
COMMENT ON COLUMN public.black_list.me_user IS '发起拉黑用户';
COMMENT ON COLUMN public.black_list.block_user IS '被拉黑用户';
COMMENT ON COLUMN public.black_list.created_at IS '创建时间';
COMMENT ON COLUMN public.black_list."version" IS '版本号';
