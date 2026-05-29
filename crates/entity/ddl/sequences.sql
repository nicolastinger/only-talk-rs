-- Sequences for v1.0.0

-- DROP SEQUENCE chat_list_link_id_seq;

CREATE SEQUENCE IF NOT EXISTS chat_list_link_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;

-- DROP SEQUENCE chat_message_record_fail_id_seq;

CREATE SEQUENCE IF NOT EXISTS chat_message_record_fail_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;

-- DROP SEQUENCE chat_message_record_id_seq;

CREATE SEQUENCE IF NOT EXISTS chat_message_record_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;

-- DROP SEQUENCE chat_message_record_read_status_id_seq;

CREATE SEQUENCE IF NOT EXISTS chat_message_record_read_status_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;

-- DROP SEQUENCE friend_request_info_id_seq;

CREATE SEQUENCE IF NOT EXISTS friend_request_info_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;

-- DROP SEQUENCE group_message_record_read_id_seq;

CREATE SEQUENCE IF NOT EXISTS group_message_record_read_id_seq
    INCREMENT BY 1
    MINVALUE 1
    MAXVALUE 2147483647
    START 1
    CACHE 1
    NO CYCLE;
