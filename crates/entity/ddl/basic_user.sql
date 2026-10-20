-- public.basic_user 表定义

-- 删除表
-- DROP TABLE basic_user;

CREATE TABLE IF NOT EXISTS basic_user (
    uuid uuid NOT NULL, -- 主键
    username varchar NOT NULL, -- 用户名
    account varchar NOT NULL, -- 用户编码
    "password" varchar NOT NULL, -- 用户密码
    info varchar NULL, -- 个人简介
    icon varchar NULL, -- 用户头像超链接
    registration_status int2 NOT NULL DEFAULT 0, -- 注册状态: 0=占位未完成 1=已完成
    user_type int2 NOT NULL DEFAULT 0, -- 用户类型: 0=普通用户 1=机器人 2=企业用户 (其他待补充)
    CONSTRAINT basic_user_pk PRIMARY KEY (uuid)
);

-- 兼容已存在的旧库: CREATE TABLE IF NOT EXISTS 不会补列, 这里幂等补上 user_type(旧数据默认 0)。
-- 用 DO 块先查系统目录, 仅在缺列时才 ALTER: ADD COLUMN IF NOT EXISTS 即使跳过也会申请
-- ACCESS EXCLUSIVE 锁, 而 CD 会重放全部 DDL, 会阻塞在线应用(见 .github/workflows/cd.yml)。
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'basic_user'
          AND column_name = 'user_type'
    ) THEN
        ALTER TABLE public.basic_user ADD COLUMN user_type int2 NOT NULL DEFAULT 0;
    END IF;
END $$;

-- 表注释
COMMENT ON TABLE public.basic_user IS '基础用户表';

-- 列注释
COMMENT ON COLUMN public.basic_user.uuid IS '主键';
COMMENT ON COLUMN public.basic_user.username IS '用户名';
COMMENT ON COLUMN public.basic_user.account IS '用户编码';
COMMENT ON COLUMN public.basic_user."password" IS '用户密码';
COMMENT ON COLUMN public.basic_user.info IS '个人简介';
COMMENT ON COLUMN public.basic_user.icon IS '用户头像超链接';
COMMENT ON COLUMN public.basic_user.registration_status IS '注册状态: 0=占位未完成 1=已完成';
COMMENT ON COLUMN public.basic_user.user_type IS '用户类型: 0=普通用户 1=机器人 2=企业用户 (其他待补充)';
