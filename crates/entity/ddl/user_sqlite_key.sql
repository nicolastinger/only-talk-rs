-- public.user_sqlite_key 表定义
-- 用户本地加密数据库密钥托管表: 以(用户+设备指纹)记录密钥下发, 同一用户的各设备/历史指纹
-- 复用同一把底层密钥(新设备沿用已签发密钥), 服务端主密钥 AES-256-GCM 加密后落库,
-- 鉴权通过后向对应设备解密返回

-- 删除表
-- DROP TABLE user_sqlite_key;

CREATE TABLE IF NOT EXISTS user_sqlite_key (
    id bigserial PRIMARY KEY, -- 主键ID
    user_id uuid NOT NULL, -- 关联 basic_user.uuid
    device_fingerprint varchar(255) NOT NULL, -- 设备指纹(SHA-256 稳定指纹)
    key_version int NOT NULL DEFAULT 1, -- 密钥版本(预留轮换/重签发)
    encrypted_key text NOT NULL, -- AES-256-GCM 加密后的密钥(hex: nonce(12)+密文含tag)
    created_at int8 NOT NULL DEFAULT 0, -- 创建时间(Unix毫秒)
    updated_at int8 NOT NULL DEFAULT 0, -- 更新时间(Unix毫秒)
    CONSTRAINT user_sqlite_key_user_fp_unique UNIQUE (user_id, device_fingerprint),
    CONSTRAINT user_sqlite_key_uuid_fk FOREIGN KEY (user_id) REFERENCES basic_user (uuid) ON DELETE CASCADE
);

-- 表注释
COMMENT ON TABLE public.user_sqlite_key IS '用户本地加密数据库密钥托管表(同用户各设备/指纹复用同一把底层密钥)';

-- 列注释
COMMENT ON COLUMN public.user_sqlite_key.id IS '主键ID';
COMMENT ON COLUMN public.user_sqlite_key.user_id IS '关联 basic_user.uuid';
COMMENT ON COLUMN public.user_sqlite_key.device_fingerprint IS '设备指纹(SHA-256 稳定指纹)';
COMMENT ON COLUMN public.user_sqlite_key.key_version IS '密钥版本(预留轮换/重签发)';
COMMENT ON COLUMN public.user_sqlite_key.encrypted_key IS 'AES-256-GCM 加密后的密钥(hex: nonce(12)+密文含tag)';
COMMENT ON COLUMN public.user_sqlite_key.created_at IS '创建时间(Unix毫秒)';
COMMENT ON COLUMN public.user_sqlite_key.updated_at IS '更新时间(Unix毫秒)';
