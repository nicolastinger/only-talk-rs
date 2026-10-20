-- public.enterprise_info 表定义

-- 删除表
-- DROP TABLE enterprise_info;

CREATE TABLE IF NOT EXISTS enterprise_info (
    uuid uuid NOT NULL, -- 主键(逻辑关联 basic_user.uuid, user_type=2)
    company_name varchar NULL, -- 企业名称
    credit_code varchar NULL, -- 统一社会信用代码
    legal_person varchar NULL, -- 法人代表
    contact_name varchar NULL, -- 联系人
    contact_phone varchar NULL, -- 联系电话
    contact_email varchar NULL, -- 联系邮箱
    address varchar NULL, -- 企业地址
    verified int2 NOT NULL DEFAULT 0, -- 是否认证: 0=未认证 1=已认证
    status int2 NOT NULL DEFAULT 0, -- 状态: 0=正常 1=禁用 2=注销等
    note varchar NULL, -- 备注
    created_at int8 NULL, -- 创建时间
    updated_at int8 NULL, -- 更新时间
    CONSTRAINT enterprise_info_pk PRIMARY KEY (uuid)
);

-- 表注释
COMMENT ON TABLE public.enterprise_info IS '企业用户扩展信息表';

-- 列注释
COMMENT ON COLUMN public.enterprise_info.uuid IS '主键(逻辑关联 basic_user.uuid, user_type=2)';
COMMENT ON COLUMN public.enterprise_info.company_name IS '企业名称';
COMMENT ON COLUMN public.enterprise_info.credit_code IS '统一社会信用代码';
COMMENT ON COLUMN public.enterprise_info.legal_person IS '法人代表';
COMMENT ON COLUMN public.enterprise_info.contact_name IS '联系人';
COMMENT ON COLUMN public.enterprise_info.contact_phone IS '联系电话';
COMMENT ON COLUMN public.enterprise_info.contact_email IS '联系邮箱';
COMMENT ON COLUMN public.enterprise_info.address IS '企业地址';
COMMENT ON COLUMN public.enterprise_info.verified IS '是否认证: 0=未认证 1=已认证';
COMMENT ON COLUMN public.enterprise_info.status IS '状态: 0=正常 1=禁用 2=注销等';
COMMENT ON COLUMN public.enterprise_info.note IS '备注';
COMMENT ON COLUMN public.enterprise_info.created_at IS '创建时间';
COMMENT ON COLUMN public.enterprise_info.updated_at IS '更新时间';
