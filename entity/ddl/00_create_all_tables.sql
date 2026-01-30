-- ========================================
-- 数据库表创建顺序脚本
-- ========================================
-- 执行说明：
-- 1. 按照顺序依次执行以下 SQL 文件
-- 2. 每个文件都包含 IF NOT EXISTS，可以安全重复执行
-- 3. 确保在执行前已创建数据库
-- ========================================

-- ========================================
-- 第一步：创建序列（SEQUENCE）
-- ========================================
-- 说明：序列必须在表之前创建，因为某些表的字段依赖序列
-- 文件：sequences.sql
\ir sequences.sql

-- ========================================
-- 第二步：创建基础用户表
-- ========================================
-- 说明：这是最基础的用户表，其他表都依赖它
-- 文件：basic_user.sql
\ir basic_user.sql

-- ========================================
-- 第三步：创建用户相关表
-- ========================================
-- 说明：这些表都依赖 basic_user 表的 uuid 字段

-- 用户密码盐表
-- 文件：basic_user_salt.sql
\ir basic_user_salt.sql

-- 用户详细信息表
-- 文件：user_info.sql
\ir user_info.sql

-- 用户缓存表
-- 文件：user_cache.sql
\ir user_cache.sql

-- 用户登录记录表
-- 文件：user_login_log.sql
\ir user_login_log.sql

-- ========================================
-- 第四步：创建文件上传相关表
-- ========================================
-- 说明：文件上传记录表依赖 basic_user 表

-- 文件上传记录表
-- 文件：file_upload_record.sql
\ir file_upload_record.sql

-- 文件上传业务表
-- 文件：biz_record.sql
\ir biz_record.sql

-- 聊天文件上传业务表
-- 文件：chat_biz_record.sql
\ir chat_biz_record.sql

-- 私密文件上传业务表
-- 文件：private_biz_record.sql
\ir private_biz_record.sql

-- ========================================
-- 第五步：创建好友相关表
-- ========================================
-- 说明：好友相关表都依赖 basic_user 表

-- 好友关系表
-- 文件：friend_link.sql
\ir friend_link.sql

-- 好友列表缓存表
-- 文件：friend_list.sql
\ir friend_list.sql

-- 好友请求表
-- 文件：friend_request_info.sql
\ir friend_request_info.sql

-- ========================================
-- 第六步：创建聊天相关表
-- ========================================
-- 说明：聊天相关表都依赖 basic_user 表

-- 聊天列表表
-- 文件：chat_list_link.sql
\ir chat_list_link.sql

-- 聊天消息记录表
-- 文件：chat_message_record.sql
\ir chat_message_record.sql

-- 聊天消息失败记录表
-- 文件：chat_message_record_fail.sql
\ir chat_message_record_fail.sql

-- 聊天消息已读状态表
-- 文件：chat_message_record_read.sql
\ir chat_message_record_read.sql

-- ========================================
-- 第七步：创建系统通知表
-- ========================================
-- 说明：系统通知表依赖 basic_user 表
-- 文件：system_notification.sql
\ir system_notification.sql

-- ========================================
-- 创建完成
-- ========================================
-- 所有表已按照正确的依赖关系创建完成
-- 可以通过以下命令验证：
-- \dt
-- \d+ table_name
