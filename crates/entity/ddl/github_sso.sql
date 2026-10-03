-- public.github_sso 表定义

-- 删除表
-- DROP TABLE github_sso;

CREATE TABLE IF NOT EXISTS github_sso (
    uuid uuid NOT NULL, -- 主键，强制关联 basic_user.uuid（1:1）
    github_id int8 NOT NULL, -- GitHub 用户 ID，唯一
    login varchar NOT NULL, -- GitHub 登录名
    name varchar NULL, -- GitHub 显示名（可能为空）
    avatar_url varchar NULL, -- GitHub 头像地址
    email varchar NULL, -- GitHub 公开邮箱（可能为空）
    verified bool NOT NULL DEFAULT true, -- GitHub 身份已验证（OAuth 即视为已认证）
    status int2 NOT NULL DEFAULT 1, -- 渠道状态: 0=未激活/禁用 1=正常 2=已解绑
    last_login_at int8 NULL, -- 最近一次该渠道登录时间
    last_login_ip varchar NULL, -- 最近登录IP
    login_count int8 NOT NULL DEFAULT 0, -- 累计登录次数
    created_at int8 NOT NULL DEFAULT 0, -- 创建时间
    updated_at int8 NOT NULL DEFAULT 0, -- 更新时间
    deleted_at int8 NULL, -- 软删除时间（预留解绑历史保留）
    CONSTRAINT github_sso_pk PRIMARY KEY (uuid),
    CONSTRAINT github_sso_github_id_unique UNIQUE (github_id),
    CONSTRAINT github_sso_uuid_fk FOREIGN KEY (uuid) REFERENCES basic_user (uuid) ON DELETE CASCADE
);

-- 表注释
COMMENT ON TABLE public.github_sso IS 'GitHub 登录渠道表（1:1 关联 basic_user，免密 OAuth）';

-- 列注释
COMMENT ON COLUMN public.github_sso.uuid IS '主键，关联 basic_user.uuid';
COMMENT ON COLUMN public.github_sso.github_id IS 'GitHub 用户 ID，唯一';
COMMENT ON COLUMN public.github_sso.login IS 'GitHub 登录名';
COMMENT ON COLUMN public.github_sso.name IS 'GitHub 显示名';
COMMENT ON COLUMN public.github_sso.avatar_url IS 'GitHub 头像地址';
COMMENT ON COLUMN public.github_sso.email IS 'GitHub 公开邮箱';
COMMENT ON COLUMN public.github_sso.verified IS 'GitHub 身份已验证';
COMMENT ON COLUMN public.github_sso.status IS '渠道状态: 0=未激活/禁用 1=正常 2=已解绑';
COMMENT ON COLUMN public.github_sso.last_login_at IS '最近一次该渠道登录时间';
COMMENT ON COLUMN public.github_sso.last_login_ip IS '最近登录IP';
COMMENT ON COLUMN public.github_sso.login_count IS '累计登录次数';
COMMENT ON COLUMN public.github_sso.created_at IS '创建时间';
COMMENT ON COLUMN public.github_sso.updated_at IS '更新时间';
COMMENT ON COLUMN public.github_sso.deleted_at IS '软删除时间';