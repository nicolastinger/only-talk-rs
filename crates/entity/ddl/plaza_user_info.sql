-- public.plaza_user_info 表定义

-- 删除表
-- DROP TABLE plaza_user_info;

CREATE TABLE IF NOT EXISTS plaza_user_info (
    uuid uuid NOT NULL, -- 用户主键
    allow_discover bool DEFAULT false NOT NULL, -- 是否允许被发现
    motto varchar(255) NULL, -- 交友宣言
    status int2 DEFAULT 0 NOT NULL, -- 状态
    created_at int8 NOT NULL, -- 创建时间
    updated_at int8 NOT NULL, -- 更新时间
    CONSTRAINT plaza_user_info_pk PRIMARY KEY (uuid)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_plaza_user_info_allow_updated ON public.plaza_user_info USING btree (allow_discover, updated_at);

-- 表注释
COMMENT ON TABLE public.plaza_user_info IS '用户交友广场信息表';

-- 列注释
COMMENT ON COLUMN public.plaza_user_info.uuid IS '用户主键';
COMMENT ON COLUMN public.plaza_user_info.allow_discover IS '是否允许被发现(默认关闭)';
COMMENT ON COLUMN public.plaza_user_info.motto IS '交友宣言';
COMMENT ON COLUMN public.plaza_user_info.status IS '状态: 0-正常, 1-隐藏, 2-封禁';
COMMENT ON COLUMN public.plaza_user_info.created_at IS '创建时间 (Unix时间戳，秒)';
COMMENT ON COLUMN public.plaza_user_info.updated_at IS '更新时间 (Unix时间戳，秒)';
