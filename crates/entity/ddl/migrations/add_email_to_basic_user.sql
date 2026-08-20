-- basic_user 表增加邮箱列(注册必填,唯一)
ALTER TABLE basic_user ADD COLUMN IF NOT EXISTS email varchar NULL;

COMMENT ON COLUMN public.basic_user.email IS '邮箱';

-- 邮箱唯一索引(每个邮箱地址只能注册一个账号)
CREATE UNIQUE INDEX IF NOT EXISTS basic_user_email_unique_idx ON basic_user (email)
    WHERE email IS NOT NULL;
