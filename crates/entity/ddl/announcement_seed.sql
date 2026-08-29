-- public.announcement 示例种子数据
-- 幂等(ON CONFLICT DO NOTHING)，可反复应用。
-- 通过 apply_all_ddl 随其它 ddl 自动执行，或用 psql 手动执行。
-- time: start_at/end_at 为 Unix 秒，这里用当前时间到 +30 天，始终处于展示窗口内。

INSERT INTO public.announcement (uuid, title, content, content_type, start_at, end_at, is_active, sort_order, is_del, created_at, updated_at)
VALUES
  (
    '11111111-1111-4111-8111-111111111111',
    '系统升级公告',
    E'## 系统升级公告\n\n我们将在本周六凌晨进行系统维护，届时**部分功能**可能短暂不可用。\n\n- 维护时间：周六 02:00 - 04:00\n- 影响范围：登录、消息同步\n\n感谢您的理解与配合。',
    0,
    EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,
    EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint + 86400 * 30,
    true,
    0,
    false,
    EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,
    EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint
  ),
  (
    '22222222-2222-4222-8222-222222222222',
    '用户协议更新',
    E'<h2>用户协议更新</h2><p>我们更新了<a href="#">《在线服务协议》</a>，请及时查阅。</p><p>如有疑问，欢迎联系客服。</p>',
    1,
    EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,
    EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint + 86400 * 30,
    true,
    1,
    false,
    EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,
    EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint
  )
ON CONFLICT (uuid) DO NOTHING;
