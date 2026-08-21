-- 迁移: 邮箱登录渠道从 basic_user 迁出至 email_sso 表
-- 日期: 2026-08-21
-- 描述: 新增 email_sso 表承载邮箱登录渠道（1:1 关联 basic_user），并从 basic_user 移除 email 列

-- 1. 创建 email_sso 表
CREATE TABLE IF NOT EXISTS email_sso (
    uuid uuid NOT NULL, -- 主键，强制关联 basic_user.uuid（1:1）
    email varchar NOT NULL, -- 邮箱原始值（展示/发件）
    email_normalized varchar NOT NULL, -- 规范化邮箱（小写），唯一
    verified bool NOT NULL DEFAULT false, -- 是否已验证
    verified_at int8 NULL, -- 验证通过时间
    verify_code_issued_at int8 NULL, -- 最近一次验证码下发时间（防刷/审计）
    is_primary bool NOT NULL DEFAULT true, -- 是否主邮箱（预留多邮箱演进）
    status int2 NOT NULL DEFAULT 1, -- 渠道状态: 0=未激活/禁用 1=正常 2=已解绑
    last_login_at int8 NULL, -- 最近一次该邮箱登录时间（预留邮箱登录）
    last_login_ip varchar NULL, -- 最近登录IP（预留）
    login_count int8 NOT NULL DEFAULT 0, -- 累计登录次数（预留）
    fail_count int8 NOT NULL DEFAULT 0, -- 连续验证失败次数（预留防爆破）
    locked_until int8 NULL, -- 锁定截止时间（预留防爆破）
    created_at int8 NOT NULL DEFAULT 0, -- 创建时间
    updated_at int8 NOT NULL DEFAULT 0, -- 更新时间
    deleted_at int8 NULL, -- 软删除时间（预留换绑/解绑历史保留）
    CONSTRAINT email_sso_pk PRIMARY KEY (uuid),
    CONSTRAINT email_sso_normalized_unique UNIQUE (email_normalized),
    CONSTRAINT email_sso_uuid_fk FOREIGN KEY (uuid) REFERENCES basic_user (uuid) ON DELETE CASCADE
);

-- 表注释
COMMENT ON TABLE public.email_sso IS '邮箱登录渠道表（1:1 关联 basic_user）';

-- 列注释
COMMENT ON COLUMN public.email_sso.uuid IS '主键，关联 basic_user.uuid';
COMMENT ON COLUMN public.email_sso.email IS '邮箱原始值（展示/发件）';
COMMENT ON COLUMN public.email_sso.email_normalized IS '规范化邮箱（小写），唯一';
COMMENT ON COLUMN public.email_sso.verified IS '是否已验证';
COMMENT ON COLUMN public.email_sso.verified_at IS '验证通过时间';
COMMENT ON COLUMN public.email_sso.verify_code_issued_at IS '最近一次验证码下发时间';
COMMENT ON COLUMN public.email_sso.is_primary IS '是否主邮箱';
COMMENT ON COLUMN public.email_sso.status IS '渠道状态: 0=未激活/禁用 1=正常 2=已解绑';
COMMENT ON COLUMN public.email_sso.last_login_at IS '最近一次该邮箱登录时间';
COMMENT ON COLUMN public.email_sso.last_login_ip IS '最近登录IP';
COMMENT ON COLUMN public.email_sso.login_count IS '累计登录次数';
COMMENT ON COLUMN public.email_sso.fail_count IS '连续验证失败次数';
COMMENT ON COLUMN public.email_sso.locked_until IS '锁定截止时间';
COMMENT ON COLUMN public.email_sso.created_at IS '创建时间';
COMMENT ON COLUMN public.email_sso.updated_at IS '更新时间';
COMMENT ON COLUMN public.email_sso.deleted_at IS '软删除时间';

-- 2. 从 basic_user 移除 email 列及其唯一索引
DROP INDEX IF EXISTS basic_user_email_unique_idx;
ALTER TABLE basic_user DROP COLUMN IF EXISTS email;
