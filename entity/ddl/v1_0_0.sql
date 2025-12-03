
-- DROP SEQUENCE chat_list_link_id_seq;

CREATE SEQUENCE chat_list_link_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;
-- DROP SEQUENCE chat_message_record_fail_id_seq;

CREATE SEQUENCE chat_message_record_fail_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;
-- DROP SEQUENCE chat_message_record_id_seq;

CREATE SEQUENCE chat_message_record_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;
-- DROP SEQUENCE chat_message_record_read_status_id_seq;

CREATE SEQUENCE chat_message_record_read_status_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;
-- DROP SEQUENCE friend_request_info_id_seq;

CREATE SEQUENCE friend_request_info_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;-- public.basic_user definition

-- Drop table

-- DROP TABLE basic_user;

CREATE TABLE basic_user (
                            "uuid" uuid NOT NULL, -- 主键
                            username varchar NOT NULL, -- 用户名
                            account varchar NOT NULL, -- 用户编码
                            "password" varchar NOT NULL, -- 用户密码
                            info varchar NULL, -- 个人简介
                            icon varchar NULL, -- 用户头像超链接
                            CONSTRAINT basic_user_pk PRIMARY KEY (uuid)
);

-- Column comments

COMMENT ON COLUMN public.basic_user."uuid" IS '主键';
COMMENT ON COLUMN public.basic_user.username IS '用户名';
COMMENT ON COLUMN public.basic_user.account IS '用户编码';
COMMENT ON COLUMN public.basic_user."password" IS '用户密码';
COMMENT ON COLUMN public.basic_user.info IS '个人简介';
COMMENT ON COLUMN public.basic_user.icon IS '用户头像超链接';


-- public.basic_user_salt definition

-- Drop table

-- DROP TABLE basic_user_salt;

CREATE TABLE basic_user_salt (
                                 "uuid" uuid NOT NULL, -- 主键
                                 sign_up_salt varchar NULL, -- 密码盐
                                 CONSTRAINT basic_user_salt_pk PRIMARY KEY (uuid)
);

-- Column comments

COMMENT ON COLUMN public.basic_user_salt."uuid" IS '主键';
COMMENT ON COLUMN public.basic_user_salt.sign_up_salt IS '密码盐';


-- public.chat_list_link definition

-- Drop table

-- DROP TABLE chat_list_link;

CREATE TABLE chat_list_link (
                                "uuid" uuid NOT NULL, -- 用户id
                                friend_uuid uuid NOT NULL, -- 朋友id
                                "enable" int2 DEFAULT 0 NULL, -- 是否显示
                                created_at int8 NULL, -- 创建时间
                                id serial4 NOT NULL, -- 自增主键
                                CONSTRAINT chat_list_link_pk PRIMARY KEY (id),
                                CONSTRAINT chat_list_link_unique UNIQUE (id)
);
COMMENT ON TABLE public.chat_list_link IS '聊天列表';

-- Column comments

COMMENT ON COLUMN public.chat_list_link."uuid" IS '用户id';
COMMENT ON COLUMN public.chat_list_link.friend_uuid IS '朋友id';
COMMENT ON COLUMN public.chat_list_link."enable" IS '是否显示';
COMMENT ON COLUMN public.chat_list_link.created_at IS '创建时间';
COMMENT ON COLUMN public.chat_list_link.id IS '自增主键';


-- public.chat_message_record definition

-- Drop table

-- DROP TABLE chat_message_record;

CREATE TABLE chat_message_record (
                                     send_user uuid NOT NULL, -- 发送人id
                                     recv_user uuid NOT NULL, -- 接收用户id
                                     text_type int4 DEFAULT 0 NOT NULL, -- 消息类型，0-文本，1-图片
                                     "timestamp" int8 NOT NULL, -- 创建时间
                                     id serial4 NOT NULL, -- 自增id
                                     nano_id varchar NOT NULL, -- 消息主键
                                     raw bytea NULL, -- 二进制数据
                                     CONSTRAINT chat_message_record_unique UNIQUE (id)
);
CREATE INDEX chat_message_record_create_at_idx ON public.chat_message_record USING btree ("timestamp");
CREATE INDEX chat_message_record_send_user_idx ON public.chat_message_record USING btree (send_user, recv_user, "timestamp");
COMMENT ON TABLE public.chat_message_record IS '单对单聊天列表';

-- Column comments

COMMENT ON COLUMN public.chat_message_record.send_user IS '发送人id';
COMMENT ON COLUMN public.chat_message_record.recv_user IS '接收用户id';
COMMENT ON COLUMN public.chat_message_record.text_type IS '消息类型，0-文本，1-图片';
COMMENT ON COLUMN public.chat_message_record."timestamp" IS '创建时间';
COMMENT ON COLUMN public.chat_message_record.id IS '自增id';
COMMENT ON COLUMN public.chat_message_record.nano_id IS '消息主键';
COMMENT ON COLUMN public.chat_message_record.raw IS '二进制数据';


-- public.chat_message_record_fail definition

-- Drop table

-- DROP TABLE chat_message_record_fail;

CREATE TABLE chat_message_record_fail (
                                          id serial4 NOT NULL, -- 自增主键
                                          send_user uuid NOT NULL, -- 发送者id
                                          recv_user uuid NULL, -- 接收者id
                                          reason varchar NULL, -- 失败原因
                                          created_at varchar NULL, -- 创建时间
                                          nano_id varchar NULL, -- 消息主键
                                          CONSTRAINT chat_message_record_fail_pk PRIMARY KEY (id)
);
COMMENT ON TABLE public.chat_message_record_fail IS '发送消息失败表';

-- Column comments

COMMENT ON COLUMN public.chat_message_record_fail.id IS '自增主键';
COMMENT ON COLUMN public.chat_message_record_fail.send_user IS '发送者id';
COMMENT ON COLUMN public.chat_message_record_fail.recv_user IS '接收者id';
COMMENT ON COLUMN public.chat_message_record_fail.reason IS '失败原因';
COMMENT ON COLUMN public.chat_message_record_fail.created_at IS '创建时间';
COMMENT ON COLUMN public.chat_message_record_fail.nano_id IS '消息主键';


-- public.chat_message_record_read definition

-- Drop table

-- DROP TABLE chat_message_record_read;

CREATE TABLE chat_message_record_read (
                                          id int4 DEFAULT nextval('chat_message_record_read_status_id_seq'::regclass) NOT NULL, -- 自增id
                                          send_user uuid NOT NULL, -- 发送人id
                                          recv_user uuid NOT NULL, -- 接收人id
                                          "timestamp" int8 NOT NULL, -- 创建时间
                                          nano_id varchar NOT NULL, -- 消息主键
                                          CONSTRAINT chat_message_record_read_status_pk PRIMARY KEY (id)
);
COMMENT ON TABLE public.chat_message_record_read IS '聊天记录已读状态表';

-- Column comments

COMMENT ON COLUMN public.chat_message_record_read.id IS '自增id';
COMMENT ON COLUMN public.chat_message_record_read.send_user IS '发送人id';
COMMENT ON COLUMN public.chat_message_record_read.recv_user IS '接收人id';
COMMENT ON COLUMN public.chat_message_record_read."timestamp" IS '创建时间';
COMMENT ON COLUMN public.chat_message_record_read.nano_id IS '消息主键';


-- public.friend_link definition

-- Drop table

-- DROP TABLE friend_link;

CREATE TABLE friend_link (
                             "uuid" uuid NOT NULL,
                             request_user uuid NOT NULL,
                             accept_user uuid NOT NULL,
                             is_del bool DEFAULT false NULL,
                             created_at int8 NOT NULL, -- 创建时间
                             updated_at int8 NOT NULL, -- 更新时间
                             "version" int4 DEFAULT 0 NULL,
                             CONSTRAINT friend_link_pk PRIMARY KEY (uuid),
                             CONSTRAINT friend_link_unique UNIQUE (request_user, accept_user)
);

-- Column comments

COMMENT ON COLUMN public.friend_link.created_at IS '创建时间';
COMMENT ON COLUMN public.friend_link.updated_at IS '更新时间';


-- public.friend_list definition

-- Drop table

-- DROP TABLE friend_list;

CREATE TABLE friend_list (
                             "uuid" uuid NOT NULL, -- 用户id
                             create_at int8 NULL, -- 创建时间
                             update_at int8 NULL, -- 更新时间
                             "version" uuid NULL, -- 版本id，缓存对比
                             last_query int8 NULL, -- 最后一次查询时间
                             CONSTRAINT friend_list_pk PRIMARY KEY (uuid)
);

-- Column comments

COMMENT ON COLUMN public.friend_list."uuid" IS '用户id';
COMMENT ON COLUMN public.friend_list.create_at IS '创建时间';
COMMENT ON COLUMN public.friend_list.update_at IS '更新时间';
COMMENT ON COLUMN public.friend_list."version" IS '版本id，缓存对比';
COMMENT ON COLUMN public.friend_list.last_query IS '最后一次查询时间';


-- public.friend_request_info definition

-- Drop table

-- DROP TABLE friend_request_info;

CREATE TABLE friend_request_info (
                                     "uuid" uuid NOT NULL, -- 主键
                                     created_at int8 NOT NULL, -- 创建时间
                                     updated_at int8 NOT NULL, -- 更新时间
                                     request_message varchar NULL, -- 加好友的文本
                                     add_type varchar NULL, -- 哪种方式添加
                                     request_user uuid NULL, -- 请求方
                                     accept_user uuid NULL, -- 接收方
                                     accept_message varchar NULL, -- 接收方信息
                                     accept_status int2 NULL, -- 加好友状态
                                     id serial4 NOT NULL, -- 自增id
                                     "version" int4 NULL,
                                     CONSTRAINT friend_request_info_pk PRIMARY KEY (id)
);
COMMENT ON TABLE public.friend_request_info IS '好友请求表';

-- Column comments

COMMENT ON COLUMN public.friend_request_info."uuid" IS '主键';
COMMENT ON COLUMN public.friend_request_info.created_at IS '创建时间';
COMMENT ON COLUMN public.friend_request_info.updated_at IS '更新时间';
COMMENT ON COLUMN public.friend_request_info.request_message IS '加好友的文本';
COMMENT ON COLUMN public.friend_request_info.add_type IS '哪种方式添加';
COMMENT ON COLUMN public.friend_request_info.request_user IS '请求方';
COMMENT ON COLUMN public.friend_request_info.accept_user IS '接收方';
COMMENT ON COLUMN public.friend_request_info.accept_message IS '接收方信息';
COMMENT ON COLUMN public.friend_request_info.accept_status IS '加好友状态';
COMMENT ON COLUMN public.friend_request_info.id IS '自增id';


-- public.system_notification definition

-- Drop table

-- DROP TABLE system_notification;

CREATE TABLE system_notification (
                                     id uuid NOT NULL, -- 主键ID
                                     title varchar(255) NULL, -- 通知标题
                                     "content" text NULL, -- 详细内容
                                     created_at int8 NULL, -- 创建时间 (Unix时间戳，毫秒)
                                     content_type int2 NULL, -- 内容类型，0-纯文本，1-json, 2-xml
                                     user_id uuid NULL, -- 接收人
                                     is_read bool NULL, -- 是否已读
                                     level1 int4 NULL, -- 第一层级，用于定位功能大类
                                     level2 int4 NULL, -- 第二层级，用于定位子功能模块
                                     level3 int4 NULL, -- 第三层级，用于定位具体功能组
                                     level4 int4 NULL, -- 第四层级，用于定位详细功能项
                                     unread_count int4 NULL, -- 未读数量
                                     priority int4 NULL, -- 通知优先级
                                     CONSTRAINT system_notification_pkey PRIMARY KEY (id)
);
CREATE INDEX idx_system_notification_is_read ON public.system_notification USING btree (is_read);
CREATE INDEX idx_system_notification_user_id_created_at ON public.system_notification USING btree (user_id, created_at);
COMMENT ON TABLE public.system_notification IS '系统通知表';

-- Column comments

COMMENT ON COLUMN public.system_notification.id IS '主键ID';
COMMENT ON COLUMN public.system_notification.title IS '通知标题';
COMMENT ON COLUMN public.system_notification."content" IS '详细内容';
COMMENT ON COLUMN public.system_notification.created_at IS '创建时间 (Unix时间戳，毫秒)';
COMMENT ON COLUMN public.system_notification.content_type IS '内容类型，0-纯文本，1-json, 2-xml';
COMMENT ON COLUMN public.system_notification.user_id IS '接收人';
COMMENT ON COLUMN public.system_notification.is_read IS '是否已读';
COMMENT ON COLUMN public.system_notification.level1 IS '第一层级，用于定位功能大类';
COMMENT ON COLUMN public.system_notification.level2 IS '第二层级，用于定位子功能模块';
COMMENT ON COLUMN public.system_notification.level3 IS '第三层级，用于定位具体功能组';
COMMENT ON COLUMN public.system_notification.level4 IS '第四层级，用于定位详细功能项';
COMMENT ON COLUMN public.system_notification.unread_count IS '未读数量';
COMMENT ON COLUMN public.system_notification.priority IS '通知优先级';


-- public.user_cache definition

-- Drop table

-- DROP TABLE user_cache;

CREATE TABLE user_cache (
                            "uuid" uuid NOT NULL, -- 用户主键
                            created_at int8 NOT NULL, -- 创建时间
                            updated_at int8 NOT NULL, -- 更新时间
                            "text" json NULL, -- 缓存json文本
                            "version" int4 NULL, -- 版本
                            CONSTRAINT user_cache_pk PRIMARY KEY (uuid)
);

-- Column comments

COMMENT ON COLUMN public.user_cache."uuid" IS '用户主键';
COMMENT ON COLUMN public.user_cache.created_at IS '创建时间';
COMMENT ON COLUMN public.user_cache.updated_at IS '更新时间';
COMMENT ON COLUMN public.user_cache."text" IS '缓存json文本';
COMMENT ON COLUMN public.user_cache."version" IS '版本';


-- public.user_info definition

-- Drop table

-- DROP TABLE user_info;

CREATE TABLE user_info (
                           "uuid" uuid NOT NULL, -- 主键
                           gender int4 DEFAULT 0 NULL, -- 性别
                           age int2 NULL, -- 年龄
                           birthday int8 DEFAULT 0 NULL,
                           create_at int8 NULL, -- 创建时间
                           update_at int8 NULL, -- 更新时间
                           last_login_at int8 NULL, -- 最后一次登录时间
                           last_login_equipment varchar NULL, -- 最后一次登陆设备
                           last_login_ipv4 varchar NULL, -- 最后一次登录ip
                           last_login_ipv6 varchar NULL, -- 最后一次登录ip
                           phone varchar NULL, -- 手机号
                           email varchar NULL, -- 邮箱
                           address varchar NULL, -- 地址
                           status int2 NULL, -- 状态
                           note varchar NULL, -- 用户简介
                           CONSTRAINT user_info_pk PRIMARY KEY (uuid)
);

-- Column comments

COMMENT ON COLUMN public.user_info."uuid" IS '主键';
COMMENT ON COLUMN public.user_info.gender IS '性别';
COMMENT ON COLUMN public.user_info.age IS '年龄';
COMMENT ON COLUMN public.user_info.create_at IS '创建时间';
COMMENT ON COLUMN public.user_info.update_at IS '更新时间';
COMMENT ON COLUMN public.user_info.last_login_at IS '最后一次登录时间';
COMMENT ON COLUMN public.user_info.last_login_equipment IS '最后一次登陆设备';
COMMENT ON COLUMN public.user_info.last_login_ipv4 IS '最后一次登录ip';
COMMENT ON COLUMN public.user_info.last_login_ipv6 IS '最后一次登录ip';
COMMENT ON COLUMN public.user_info.phone IS '手机号';
COMMENT ON COLUMN public.user_info.email IS '邮箱';
COMMENT ON COLUMN public.user_info.address IS '地址';
COMMENT ON COLUMN public.user_info.status IS '状态';
COMMENT ON COLUMN public.user_info.note IS '用户简介';

-- Drop table

-- DROP TABLE basic_user;

CREATE TABLE basic_user (
                            "uuid" uuid NOT NULL, -- 主键
                            username varchar NOT NULL, -- 用户名
                            account varchar NOT NULL, -- 用户编码
                            "password" varchar NOT NULL, -- 用户密码
                            info varchar NULL, -- 个人简介
                            icon varchar NULL, -- 用户头像超链接
                            CONSTRAINT basic_user_pk PRIMARY KEY (uuid)
);

-- Column comments

COMMENT ON COLUMN public.basic_user."uuid" IS '主键';
COMMENT ON COLUMN public.basic_user.username IS '用户名';
COMMENT ON COLUMN public.basic_user.account IS '用户编码';
COMMENT ON COLUMN public.basic_user."password" IS '用户密码';
COMMENT ON COLUMN public.basic_user.info IS '个人简介';
COMMENT ON COLUMN public.basic_user.icon IS '用户头像超链接';

-- Drop table

-- DROP TABLE basic_user_salt;

CREATE TABLE basic_user_salt (
                                 "uuid" uuid NOT NULL, -- 主键
                                 sign_up_salt varchar NULL, -- 密码盐
                                 CONSTRAINT basic_user_salt_pk PRIMARY KEY (uuid)
);

-- Column comments

COMMENT ON COLUMN public.basic_user_salt."uuid" IS '主键';
COMMENT ON COLUMN public.basic_user_salt.sign_up_salt IS '密码盐';

-- Drop table

-- DROP TABLE chat_list_link;

CREATE TABLE chat_list_link (
                                "uuid" uuid NOT NULL, -- 用户id
                                friend_uuid uuid NOT NULL, -- 朋友id
                                "enable" int2 DEFAULT 0 NULL, -- 是否显示
                                created_at int8 NULL, -- 创建时间
                                id serial4 NOT NULL, -- 自增主键
                                CONSTRAINT chat_list_link_pk PRIMARY KEY (id),
                                CONSTRAINT chat_list_link_unique UNIQUE (id)
);
COMMENT ON TABLE public.chat_list_link IS '聊天列表';

-- Column comments

COMMENT ON COLUMN public.chat_list_link."uuid" IS '用户id';
COMMENT ON COLUMN public.chat_list_link.friend_uuid IS '朋友id';
COMMENT ON COLUMN public.chat_list_link."enable" IS '是否显示';
COMMENT ON COLUMN public.chat_list_link.created_at IS '创建时间';
COMMENT ON COLUMN public.chat_list_link.id IS '自增主键';

-- Drop table

-- DROP TABLE chat_message_record;

CREATE TABLE chat_message_record (
                                     send_user uuid NOT NULL, -- 发送人id
                                     recv_user uuid NOT NULL, -- 接收用户id
                                     text_type int4 DEFAULT 0 NOT NULL, -- 消息类型，0-文本，1-图片
                                     "timestamp" int8 NOT NULL, -- 创建时间
                                     id serial4 NOT NULL, -- 自增id
                                     nano_id varchar NOT NULL, -- 消息主键
                                     raw bytea NULL, -- 二进制数据
                                     CONSTRAINT chat_message_record_unique UNIQUE (id)
);
CREATE INDEX chat_message_record_create_at_idx ON public.chat_message_record USING btree ("timestamp");
CREATE INDEX chat_message_record_send_user_idx ON public.chat_message_record USING btree (send_user, recv_user, "timestamp");
COMMENT ON TABLE public.chat_message_record IS '单对单聊天列表';

-- Column comments

COMMENT ON COLUMN public.chat_message_record.send_user IS '发送人id';
COMMENT ON COLUMN public.chat_message_record.recv_user IS '接收用户id';
COMMENT ON COLUMN public.chat_message_record.text_type IS '消息类型，0-文本，1-图片';
COMMENT ON COLUMN public.chat_message_record."timestamp" IS '创建时间';
COMMENT ON COLUMN public.chat_message_record.id IS '自增id';
COMMENT ON COLUMN public.chat_message_record.nano_id IS '消息主键';
COMMENT ON COLUMN public.chat_message_record.raw IS '二进制数据';

-- Drop table

-- DROP TABLE chat_message_record_fail;

CREATE TABLE chat_message_record_fail (
                                          id serial4 NOT NULL, -- 自增主键
                                          send_user uuid NOT NULL, -- 发送者id
                                          recv_user uuid NULL, -- 接收者id
                                          reason varchar NULL, -- 失败原因
                                          created_at varchar NULL, -- 创建时间
                                          nano_id varchar NULL, -- 消息主键
                                          CONSTRAINT chat_message_record_fail_pk PRIMARY KEY (id)
);
COMMENT ON TABLE public.chat_message_record_fail IS '发送消息失败表';

-- Column comments

COMMENT ON COLUMN public.chat_message_record_fail.id IS '自增主键';
COMMENT ON COLUMN public.chat_message_record_fail.send_user IS '发送者id';
COMMENT ON COLUMN public.chat_message_record_fail.recv_user IS '接收者id';
COMMENT ON COLUMN public.chat_message_record_fail.reason IS '失败原因';
COMMENT ON COLUMN public.chat_message_record_fail.created_at IS '创建时间';
COMMENT ON COLUMN public.chat_message_record_fail.nano_id IS '消息主键';

-- Drop table

-- DROP TABLE chat_message_record_read;

CREATE TABLE chat_message_record_read (
                                          id int4 DEFAULT nextval('chat_message_record_read_status_id_seq'::regclass) NOT NULL, -- 自增id
                                          send_user uuid NOT NULL, -- 发送人id
                                          recv_user uuid NOT NULL, -- 接收人id
                                          "timestamp" int8 NOT NULL, -- 创建时间
                                          nano_id varchar NOT NULL, -- 消息主键
                                          CONSTRAINT chat_message_record_read_status_pk PRIMARY KEY (id)
);
COMMENT ON TABLE public.chat_message_record_read IS '聊天记录已读状态表';

-- Column comments

COMMENT ON COLUMN public.chat_message_record_read.id IS '自增id';
COMMENT ON COLUMN public.chat_message_record_read.send_user IS '发送人id';
COMMENT ON COLUMN public.chat_message_record_read.recv_user IS '接收人id';
COMMENT ON COLUMN public.chat_message_record_read."timestamp" IS '创建时间';
COMMENT ON COLUMN public.chat_message_record_read.nano_id IS '消息主键';

-- Drop table

-- DROP TABLE friend_link;

CREATE TABLE friend_link (
                             "uuid" uuid NOT NULL,
                             request_user uuid NOT NULL,
                             accept_user uuid NOT NULL,
                             is_del bool DEFAULT false NULL,
                             created_at int8 NOT NULL, -- 创建时间
                             updated_at int8 NOT NULL, -- 更新时间
                             "version" int4 DEFAULT 0 NULL,
                             CONSTRAINT friend_link_pk PRIMARY KEY (uuid),
                             CONSTRAINT friend_link_unique UNIQUE (request_user, accept_user)
);

-- Column comments

COMMENT ON COLUMN public.friend_link.created_at IS '创建时间';
COMMENT ON COLUMN public.friend_link.updated_at IS '更新时间';

-- Drop table

-- DROP TABLE friend_list;

CREATE TABLE friend_list (
                             "uuid" uuid NOT NULL, -- 用户id
                             create_at int8 NULL, -- 创建时间
                             update_at int8 NULL, -- 更新时间
                             "version" uuid NULL, -- 版本id，缓存对比
                             last_query int8 NULL, -- 最后一次查询时间
                             CONSTRAINT friend_list_pk PRIMARY KEY (uuid)
);

-- Column comments

COMMENT ON COLUMN public.friend_list."uuid" IS '用户id';
COMMENT ON COLUMN public.friend_list.create_at IS '创建时间';
COMMENT ON COLUMN public.friend_list.update_at IS '更新时间';
COMMENT ON COLUMN public.friend_list."version" IS '版本id，缓存对比';
COMMENT ON COLUMN public.friend_list.last_query IS '最后一次查询时间';

-- Drop table

-- DROP TABLE friend_request_info;

CREATE TABLE friend_request_info (
                                     "uuid" uuid NOT NULL, -- 主键
                                     created_at int8 NOT NULL, -- 创建时间
                                     updated_at int8 NOT NULL, -- 更新时间
                                     request_message varchar NULL, -- 加好友的文本
                                     add_type varchar NULL, -- 哪种方式添加
                                     request_user uuid NULL, -- 请求方
                                     accept_user uuid NULL, -- 接收方
                                     accept_message varchar NULL, -- 接收方信息
                                     accept_status int2 NULL, -- 加好友状态
                                     id serial4 NOT NULL, -- 自增id
                                     "version" int4 NULL,
                                     CONSTRAINT friend_request_info_pk PRIMARY KEY (id)
);
COMMENT ON TABLE public.friend_request_info IS '好友请求表';

-- Column comments

COMMENT ON COLUMN public.friend_request_info."uuid" IS '主键';
COMMENT ON COLUMN public.friend_request_info.created_at IS '创建时间';
COMMENT ON COLUMN public.friend_request_info.updated_at IS '更新时间';
COMMENT ON COLUMN public.friend_request_info.request_message IS '加好友的文本';
COMMENT ON COLUMN public.friend_request_info.add_type IS '哪种方式添加';
COMMENT ON COLUMN public.friend_request_info.request_user IS '请求方';
COMMENT ON COLUMN public.friend_request_info.accept_user IS '接收方';
COMMENT ON COLUMN public.friend_request_info.accept_message IS '接收方信息';
COMMENT ON COLUMN public.friend_request_info.accept_status IS '加好友状态';
COMMENT ON COLUMN public.friend_request_info.id IS '自增id';

-- Drop table

-- DROP TABLE system_notification;

CREATE TABLE system_notification (
                                     id uuid NOT NULL, -- 主键ID
                                     title varchar(255) NULL, -- 通知标题
                                     "content" text NULL, -- 详细内容
                                     created_at int8 NULL, -- 创建时间 (Unix时间戳，毫秒)
                                     content_type int2 NULL, -- 内容类型，0-纯文本，1-json, 2-xml
                                     user_id uuid NULL, -- 接收人
                                     is_read bool NULL, -- 是否已读
                                     level1 int4 NULL, -- 第一层级，用于定位功能大类
                                     level2 int4 NULL, -- 第二层级，用于定位子功能模块
                                     level3 int4 NULL, -- 第三层级，用于定位具体功能组
                                     level4 int4 NULL, -- 第四层级，用于定位详细功能项
                                     unread_count int4 NULL, -- 未读数量
                                     priority int4 NULL, -- 通知优先级
                                     biz_id varchar(255) NULL,
                                     CONSTRAINT system_notification_pkey PRIMARY KEY (id)
);
CREATE INDEX idx_system_notification_is_read ON public.system_notification USING btree (is_read);
CREATE INDEX idx_system_notification_user_id_created_at ON public.system_notification USING btree (user_id, created_at);
COMMENT ON TABLE public.system_notification IS '系统通知表';

-- Column comments

COMMENT ON COLUMN public.system_notification.id IS '主键ID';
COMMENT ON COLUMN public.system_notification.title IS '通知标题';
COMMENT ON COLUMN public.system_notification."content" IS '详细内容';
COMMENT ON COLUMN public.system_notification.created_at IS '创建时间 (Unix时间戳，毫秒)';
COMMENT ON COLUMN public.system_notification.content_type IS '内容类型，0-纯文本，1-json, 2-xml';
COMMENT ON COLUMN public.system_notification.user_id IS '接收人';
COMMENT ON COLUMN public.system_notification.is_read IS '是否已读';
COMMENT ON COLUMN public.system_notification.level1 IS '第一层级，用于定位功能大类';
COMMENT ON COLUMN public.system_notification.level2 IS '第二层级，用于定位子功能模块';
COMMENT ON COLUMN public.system_notification.level3 IS '第三层级，用于定位具体功能组';
COMMENT ON COLUMN public.system_notification.level4 IS '第四层级，用于定位详细功能项';
COMMENT ON COLUMN public.system_notification.unread_count IS '未读数量';
COMMENT ON COLUMN public.system_notification.biz_id IS '业务id';
COMMENT ON COLUMN public.system_notification.priority IS '通知优先级';

-- Drop table

-- DROP TABLE user_cache;

CREATE TABLE user_cache (
                            "uuid" uuid NOT NULL, -- 用户主键
                            created_at int8 NOT NULL, -- 创建时间
                            updated_at int8 NOT NULL, -- 更新时间
                            "text" json NULL, -- 缓存json文本
                            "version" int4 NULL, -- 版本
                            CONSTRAINT user_cache_pk PRIMARY KEY (uuid)
);

-- Column comments

COMMENT ON COLUMN public.user_cache."uuid" IS '用户主键';
COMMENT ON COLUMN public.user_cache.created_at IS '创建时间';
COMMENT ON COLUMN public.user_cache.updated_at IS '更新时间';
COMMENT ON COLUMN public.user_cache."text" IS '缓存json文本';
COMMENT ON COLUMN public.user_cache."version" IS '版本';

-- Drop table

-- DROP TABLE user_info;

CREATE TABLE user_info (
                           "uuid" uuid NOT NULL, -- 主键
                           gender int4 DEFAULT 0 NULL, -- 性别
                           age int2 NULL, -- 年龄
                           birthday int8 DEFAULT 0 NULL,
                           create_at int8 NULL, -- 创建时间
                           update_at int8 NULL, -- 更新时间
                           last_login_at int8 NULL, -- 最后一次登录时间
                           last_login_equipment varchar NULL, -- 最后一次登陆设备
                           last_login_ipv4 varchar NULL, -- 最后一次登录ip
                           last_login_ipv6 varchar NULL, -- 最后一次登录ip
                           phone varchar NULL, -- 手机号
                           email varchar NULL, -- 邮箱
                           address varchar NULL, -- 地址
                           status int2 NULL, -- 状态
                           note varchar NULL, -- 用户简介
                           CONSTRAINT user_info_pk PRIMARY KEY (uuid)
);

-- Column comments

COMMENT ON COLUMN public.user_info."uuid" IS '主键';
COMMENT ON COLUMN public.user_info.gender IS '性别';
COMMENT ON COLUMN public.user_info.age IS '年龄';
COMMENT ON COLUMN public.user_info.create_at IS '创建时间';
COMMENT ON COLUMN public.user_info.update_at IS '更新时间';
COMMENT ON COLUMN public.user_info.last_login_at IS '最后一次登录时间';
COMMENT ON COLUMN public.user_info.last_login_equipment IS '最后一次登陆设备';
COMMENT ON COLUMN public.user_info.last_login_ipv4 IS '最后一次登录ip';
COMMENT ON COLUMN public.user_info.last_login_ipv6 IS '最后一次登录ip';
COMMENT ON COLUMN public.user_info.phone IS '手机号';
COMMENT ON COLUMN public.user_info.email IS '邮箱';
COMMENT ON COLUMN public.user_info.address IS '地址';
COMMENT ON COLUMN public.user_info.status IS '状态';
COMMENT ON COLUMN public.user_info.note IS '用户简介';

-- Drop table

-- DROP TABLE file_upload_record;

CREATE TABLE file_upload_record (
                                   id bigserial NOT NULL, -- 主键ID
                                   uuid uuid NOT NULL, -- 文件唯一标识符
                                   original_name varchar(255) NULL, -- 原始文件名
                                   stored_name varchar(255) NULL, -- 存储文件名
                                   file_path varchar(512) NOT NULL, -- 文件路径
                                   file_size int8 NOT NULL, -- 文件大小（字节）
                                   mime_type varchar(128) NULL, -- 文件MIME类型
                                   file_hash varchar(128) NULL, -- 文件哈希值（用于去重）
                                   upload_user_uuid uuid NOT NULL, -- 上传用户UUID
                                   upload_time int8 NOT NULL, -- 上传时间（Unix时间戳，毫秒）
                                   status int4 DEFAULT 0 NOT NULL, -- 文件状态（0-正常，1-已删除，2-临时文件）
                                   description text NULL, -- 文件描述
                                   download_count int4 DEFAULT 0 NOT NULL, -- 下载次数
                                   last_download_time int8 NULL, -- 最后下载时间
                                   is_oss int4 DEFAULT 0 NOT NULL, -- 是否为OSS存储（0-否，1-是）
                                   oss_type int4 DEFAULT 3 NOT NULL, -- OSS类型（0-阿里云，1-腾讯云，2-亚马逊AWS，3-其他）
                                   CONSTRAINT file_upload_record_pkey PRIMARY KEY (id)
);
CREATE INDEX idx_file_upload_record_uuid ON public.file_upload_record USING btree (uuid);
CREATE INDEX idx_file_upload_record_user_uuid ON public.file_upload_record USING btree (upload_user_uuid);
CREATE INDEX idx_file_upload_record_upload_time ON public.file_upload_record USING btree (upload_time);
COMMENT ON TABLE public.file_upload_record IS '文件上传记录表';

-- Column comments

COMMENT ON COLUMN public.file_upload_record.id IS '主键ID';
COMMENT ON COLUMN public.file_upload_record.uuid IS '文件唯一标识符';
COMMENT ON COLUMN public.file_upload_record.original_name IS '原始文件名';
COMMENT ON COLUMN public.file_upload_record.stored_name IS '存储文件名';
COMMENT ON COLUMN public.file_upload_record.file_path IS '文件路径';
COMMENT ON COLUMN public.file_upload_record.file_size IS '文件大小（字节）';
COMMENT ON COLUMN public.file_upload_record.mime_type IS '文件MIME类型';
COMMENT ON COLUMN public.file_upload_record.file_hash IS '文件哈希值（用于去重）';
COMMENT ON COLUMN public.file_upload_record.upload_user_uuid IS '上传用户UUID';
COMMENT ON COLUMN public.file_upload_record.upload_time IS '上传时间（Unix时间戳，毫秒）';
COMMENT ON COLUMN public.file_upload_record.status IS '文件状态（0-正常，1-已删除，2-临时文件）';
COMMENT ON COLUMN public.file_upload_record.description IS '文件描述';
COMMENT ON COLUMN public.file_upload_record.download_count IS '下载次数';
COMMENT ON COLUMN public.file_upload_record.last_download_time IS '最后下载时间';
COMMENT ON COLUMN public.file_upload_record.is_oss IS '是否为OSS存储（0-否，1-是）';
COMMENT ON COLUMN public.file_upload_record.oss_type IS 'OSS类型（0-阿里云，1-腾讯云，2-亚马逊AWS，3-其他）';
