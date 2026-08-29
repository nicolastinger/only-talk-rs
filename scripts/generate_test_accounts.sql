-- ============================================================================
-- generate_test_accounts.sql
--
-- 批量生成可登录测试账号。
--
-- 登录方式: account(账号) + password(密码)，密码以 Argon2id 哈希存入 basic_user。
-- 所有账号共用同一个密码： Test@12345678
--
-- 写入的表:
--   basic_user      登录凭证  (registration_status = 1 才可登录)
--   user_info       用户详情
--   email_sso       邮箱登录渠道 (FK -> basic_user.uuid, email_normalized 唯一)
--   plaza_user_info 交友广场信息
--   user_cache      用户缓存
--
-- 账号命名: test_001 ... test_200
--   用户名: 测试用户001 ... 测试用户200
--   邮箱:   test_001@test.local ...
--
-- 依赖: PostgreSQL 13+ (gen_random_uuid 为内置函数)
-- 用法: psql -U <user> -d <database> -f generate_test_accounts.sql
--
-- 注意:
--   1. 本脚本**不要**放进 crates/entity/ddl/ 目录 —— apply_all_ddl 会递归执行
--      该目录下全部 .sql，INSERT 脚本会被集成测试自动运行。
--   2. email_sso.email_normalized 有唯一约束, 同一分库上重复运行本脚本会报错；
--      需要重新生成时请先清空相关测试表(或重建测试库)。
-- ============================================================================

DO $$
DECLARE
    i           int;
    v_uuid      uuid;
    v_account   text;
    v_username  text;
    v_email     text;
    v_now       int8;
    -- 预生成的 Argon2id 哈希，对应明文密码 Ab123456789011 (所有账号共用同一份哈希)
    v_pwd       text := '$argon2id$v=19$m=19456,t=2,p=1$sM4ZuDttXb+J2I4IDf2MXQ$/Ki6hbDmCnDLaKG7HUC5phluVj63kMIn4zjEqZcDyAY';
BEGIN
    v_now := (EXTRACT(EPOCH FROM now()) * 1000)::int8;

    FOR i IN 1..200 LOOP
        v_uuid     := gen_random_uuid();
        v_account  := 'test_' || lpad(i::text, 3, '0');
        v_username := '测试用户' || lpad(i::text, 3, '0');
        v_email    := 'test_' || lpad(i::text, 3, '0') || '@test.local';

        -- 1. 登录凭证
        INSERT INTO basic_user (uuid, username, account, password, info, icon, registration_status)
        VALUES (v_uuid, v_username, v_account, v_pwd, '批量生成的测试账号', NULL, 1);

        -- 2. 用户详情
        INSERT INTO user_info
            (uuid, gender, age, birthday, created_at, updated_at, phone, email, address, status, note)
        VALUES
            (v_uuid, 0, 0, 0, v_now, v_now, NULL, v_email, NULL, 0, '批量生成的测试账号');

        -- 3. 邮箱登录渠道 (email_normalized 唯一)
        INSERT INTO email_sso
            (uuid, email, email_normalized, verified, verified_at, verify_code_issued_at,
             is_primary, status, last_login_at, last_login_ip, login_count, fail_count,
             locked_until, created_at, updated_at, deleted_at)
        VALUES
            (v_uuid, v_email, lower(v_email), true, v_now, v_now,
             true, 1, NULL, NULL, 0, 0, NULL, v_now, v_now, NULL);

        -- 4. 交友广场信息
        INSERT INTO plaza_user_info
            (uuid, allow_discover, motto, status, created_at, updated_at)
        VALUES
            (v_uuid, true, '这是测试账号 ' || i, 0, v_now, v_now);

        -- 5. 用户缓存
        INSERT INTO user_cache
            (uuid, created_at, updated_at, text, version)
        VALUES
            (v_uuid, v_now, v_now, NULL, 1);
    END LOOP;
END $$;
