-- public.user_auth_factor 表定义
-- 二次认证因素表: 与 basic_user 关联(1 用户多因素, 同类型至多一条), 支持 email/phone/other
-- 三类因素; 当前仅 email 渠道接入(带邮箱验证码校验), phone/other 预留。

-- 删除表
-- DROP TABLE user_auth_factor;

CREATE TABLE IF NOT EXISTS user_auth_factor (
    id bigserial PRIMARY KEY, -- 主键ID
    user_id uuid NOT NULL, -- 关联 basic_user.uuid
    factor_type int2 NOT NULL, -- 因素类型: 0=email 1=phone 2=other
    factor_value varchar NOT NULL, -- 因素值(邮箱/手机号/其他标识)
    verified bool NOT NULL DEFAULT false, -- 是否已验证
    enabled bool NOT NULL DEFAULT true, -- 是否启用
    is_primary bool NOT NULL DEFAULT false, -- 是否主因素
    status int2 NOT NULL DEFAULT 1, -- 因素状态: 0=禁用 1=正常 2=已解绑
    verified_at int8 NULL, -- 验证通过时间(Unix毫秒)
    last_used_at int8 NULL, -- 最近使用时间(Unix毫秒)
    created_at int8 NOT NULL DEFAULT 0, -- 创建时间(Unix毫秒)
    updated_at int8 NOT NULL DEFAULT 0, -- 更新时间(Unix毫秒)
    deleted_at int8 NULL, -- 软删除时间(预留, 当前 CRUD 为物理删除)
    CONSTRAINT user_auth_factor_user_type_unique UNIQUE (user_id, factor_type),
    CONSTRAINT user_auth_factor_user_fk FOREIGN KEY (user_id) REFERENCES basic_user (uuid) ON DELETE CASCADE
);

-- 表注释
COMMENT ON TABLE public.user_auth_factor IS '二次认证因素表(1 用户多因素, 关联 basic_user.uuid)';

-- 列注释
COMMENT ON COLUMN public.user_auth_factor.id IS '主键ID';
COMMENT ON COLUMN public.user_auth_factor.user_id IS '关联 basic_user.uuid';
COMMENT ON COLUMN public.user_auth_factor.factor_type IS '因素类型: 0=email 1=phone 2=other';
COMMENT ON COLUMN public.user_auth_factor.factor_value IS '因素值(邮箱/手机号/其他标识)';
COMMENT ON COLUMN public.user_auth_factor.verified IS '是否已验证';
COMMENT ON COLUMN public.user_auth_factor.enabled IS '是否启用';
COMMENT ON COLUMN public.user_auth_factor.is_primary IS '是否主因素';
COMMENT ON COLUMN public.user_auth_factor.status IS '因素状态: 0=禁用 1=正常 2=已解绑';
COMMENT ON COLUMN public.user_auth_factor.verified_at IS '验证通过时间(Unix毫秒)';
COMMENT ON COLUMN public.user_auth_factor.last_used_at IS '最近使用时间(Unix毫秒)';
COMMENT ON COLUMN public.user_auth_factor.created_at IS '创建时间(Unix毫秒)';
COMMENT ON COLUMN public.user_auth_factor.updated_at IS '更新时间(Unix毫秒)';
COMMENT ON COLUMN public.user_auth_factor.deleted_at IS '软删除时间(预留)';
