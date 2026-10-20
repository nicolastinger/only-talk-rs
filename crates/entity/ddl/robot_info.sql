-- public.robot_info 表定义

-- 删除表
-- DROP TABLE robot_info;

CREATE TABLE IF NOT EXISTS robot_info (
    uuid uuid NOT NULL, -- 主键(逻辑关联 basic_user.uuid, user_type=1)
    owner_uuid uuid NULL, -- 归属用户(谁创建的机器人, 逻辑关联 basic_user.uuid)
    model varchar NULL, -- 模型标识(如 gpt-4)
    prompt varchar NULL, -- 人设/系统提示词
    enabled int2 NOT NULL DEFAULT 1, -- 是否启用: 0=禁用 1=启用
    status int2 NOT NULL DEFAULT 0, -- 状态: 0=正常 1=禁用 2=注销等
    note varchar NULL, -- 备注
    created_at int8 NULL, -- 创建时间
    updated_at int8 NULL, -- 更新时间
    CONSTRAINT robot_info_pk PRIMARY KEY (uuid)
);

-- 表注释
COMMENT ON TABLE public.robot_info IS '机器人扩展信息表';

-- 列注释
COMMENT ON COLUMN public.robot_info.uuid IS '主键(逻辑关联 basic_user.uuid, user_type=1)';
COMMENT ON COLUMN public.robot_info.owner_uuid IS '归属用户(谁创建的机器人)';
COMMENT ON COLUMN public.robot_info.model IS '模型标识(如 gpt-4)';
COMMENT ON COLUMN public.robot_info.prompt IS '人设/系统提示词';
COMMENT ON COLUMN public.robot_info.enabled IS '是否启用: 0=禁用 1=启用';
COMMENT ON COLUMN public.robot_info.status IS '状态: 0=正常 1=禁用 2=注销等';
COMMENT ON COLUMN public.robot_info.note IS '备注';
COMMENT ON COLUMN public.robot_info.created_at IS '创建时间';
COMMENT ON COLUMN public.robot_info.updated_at IS '更新时间';
