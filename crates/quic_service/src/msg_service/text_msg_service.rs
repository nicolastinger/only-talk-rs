use std::sync::Arc;

use anyhow::anyhow;
use common::utils::message_types::MSG_TYPE_TEXT;
use common::utils::time::get_now_time_stamp_as_millis;
use nanoid::nanoid;
use tokio::sync::{Mutex, MutexGuard};
use tracing::error;

use crate::X25;
use crate::models::text_msg::{HeadMsg, TextMsg, TextQuicMsg};

// Generate text message
pub fn generate_text_msg(
    text_type: u16,
    raw: Vec<u8>,
    recv_user: String,
    send_user: String,
) -> anyhow::Result<Vec<u8>> {
    let now = get_now_time_stamp_as_millis().unwrap_or(-99999999999);
    let text_quic_msg =
        TextQuicMsg { nano_id: nanoid!(), text_type, raw, recv_user, send_user, timestamp: now };
    build_text(text_quic_msg)
}

// Generate text message
pub fn generate_text_msg_with_id(
    nano_id: String,
    text_type: u16,
    raw: Vec<u8>,
    recv_user: String,
    send_user: String,
) -> anyhow::Result<Vec<u8>> {
    let now = get_now_time_stamp_as_millis().unwrap_or(-99999999999);
    let text_quic_msg =
        TextQuicMsg { nano_id, text_type, raw, recv_user, send_user, timestamp: now };
    build_text(text_quic_msg)
}

// Generate text message
pub fn generate_text_msg_with_time(
    nano_id: String,
    text_type: u16,
    raw: Vec<u8>,
    recv_user: String,
    send_user: String,
    timestamp: i64,
) -> anyhow::Result<Vec<u8>> {
    let text_quic_msg = TextQuicMsg { nano_id, text_type, raw, recv_user, send_user, timestamp };
    build_text(text_quic_msg)
}

fn build_text(text_quic_msg: TextQuicMsg) -> anyhow::Result<Vec<u8>> {
    let meta_data = text_quic_msg.get_bytes()?;
    let crc = X25.checksum(&meta_data);
    let head_msg = HeadMsg {
        version: 1,
        crc,
        body_len: meta_data.len() as u32, // Message body length
        message_type: MSG_TYPE_TEXT,      // Message type
    };

    build_text_msg(&head_msg, &text_quic_msg)
}

// Assemble header + message body
pub fn build_text_msg<H: TextMsg, G: TextMsg>(
    text_head: &H,
    text_msg: &G,
) -> anyhow::Result<Vec<u8>> {
    let mut head_byte = text_head.get_bytes()?;
    let mut msg_byte = text_msg.get_bytes()?;
    head_byte.append(&mut msg_byte);
    Ok(head_byte)
}

// Parse text message
pub async fn get_text_msg(
    buffer: &mut Vec<u8>,
    mut length: usize,
    buffer_msg: Arc<Mutex<Vec<u8>>>,
    head_length: usize,
) -> anyhow::Result<Vec<TextQuicMsg>> {
    let mut result_vec: Vec<TextQuicMsg> = Vec::new();
    {
        // Get lock and access data in Arc
        let mut buffer_vec: MutexGuard<Vec<u8>> = buffer_msg.lock().await;

        // If buffer_vec has data, merge it with buffer
        if !buffer_vec.is_empty() {
            // Create a new Vec<u8>, merge buffer_vec and buffer data
            let mut combined_buffer = buffer_vec.clone(); // Copy buffer_vec data
            length += combined_buffer.len();
            combined_buffer.extend_from_slice(buffer); // Append buffer data to combined_buffer
            *buffer = combined_buffer; // Assign merged data back to buffer
            buffer_vec.clear();
        }
    } // buffer_vec goes out of scope, lock is released

    let mut i = 0;
    for j in 0..length {
        if j != 0 && i >= length {
            break;
        }

        let round = i;
        let head_length_right = head_length + i;
        if head_length_right >= length {
            buffer_msg.lock().await.append(&mut buffer[round..length].to_vec());
            return Ok(result_vec);
        }
        let head_msg_vec = &buffer[i..head_length_right];
        let head_msg: HeadMsg = match bincode::deserialize(head_msg_vec) {
            Ok(msg) => msg,
            Err(error) => {
                error!("failed to serialize sticky packet data! {}", error);
                buffer_msg.lock().await.append(&mut buffer[round..length].to_vec());
                return Ok(result_vec);
            }
        };
        i += head_length;

        let body_size = head_msg.body_len as usize + head_length_right;
        if body_size > length {
            buffer_msg.lock().await.append(&mut buffer[round..length].to_vec());
            return Ok(result_vec);
        }

        let body_msg_vec = &buffer[i..length.min(body_size)];
        let body_msg: TextQuicMsg = match bincode::deserialize(body_msg_vec) {
            Ok(msg) => msg,
            Err(error) => {
                error!("failed to serialize sticky packet data! {}", error);
                buffer_msg.lock().await.append(&mut buffer[round..length].to_vec());
                return Ok(result_vec);
            }
        };

        let crc = X25.checksum(body_msg_vec);
        if crc != head_msg.crc {
            Err(anyhow!("Failed to parse checksum"))?
        }
        result_vec.push(body_msg);
        i += head_msg.body_len as usize;
    }
    Ok(result_vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::X25;
    use crate::models::text_msg::HeadMsg;
    use common::utils::message_types::MSG_TYPE_TEXT;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn head_size() -> usize {
        let head = HeadMsg { version: 1, crc: 0, body_len: 0, message_type: MSG_TYPE_TEXT };
        bincode::serialize(&head).unwrap().len()
    }

    fn make_msg(text_type: u16, raw: &[u8], recv_user: &str, send_user: &str) -> Vec<u8> {
        generate_text_msg(text_type, raw.to_vec(), recv_user.to_string(), send_user.to_string())
            .unwrap()
    }

    fn new_buffer_msg() -> Arc<Mutex<Vec<u8>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    // ========== Single message ==========

    #[tokio::test]
    async fn test_single_complete_message() {
        let msg = make_msg(MSG_TYPE_TEXT, b"hello", "user_b", "user_a");
        let len = msg.len();
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        let result = get_text_msg(&mut msg.clone(), len, buf_msg, head_len).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw, b"hello");
        assert_eq!(result[0].send_user, "user_a");
        assert_eq!(result[0].recv_user, "user_b");
        assert_eq!(result[0].text_type, MSG_TYPE_TEXT);
    }

    // ========== Sticky packets: multiple complete messages ==========

    #[tokio::test]
    async fn test_two_sticky_messages() {
        let msg1 = make_msg(MSG_TYPE_TEXT, b"msg1", "user_b", "user_a");
        let msg2 = make_msg(MSG_TYPE_TEXT, b"msg2", "user_c", "user_a");

        let mut combined = msg1.clone();
        combined.extend_from_slice(&msg2);
        let total_len = combined.len();
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        let result = get_text_msg(&mut combined, total_len, buf_msg, head_len).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].raw, b"msg1");
        assert_eq!(result[0].recv_user, "user_b");
        assert_eq!(result[1].raw, b"msg2");
        assert_eq!(result[1].recv_user, "user_c");
        assert_ne!(result[0].nano_id, result[1].nano_id);
    }

    #[tokio::test]
    async fn test_many_sticky_messages() {
        const N: usize = 10;
        let mut combined = Vec::new();
        for i in 0..N {
            combined.extend_from_slice(&make_msg(
                MSG_TYPE_TEXT,
                format!("msg_{i}").as_bytes(),
                &format!("user_{}", i + 1),
                "sender",
            ));
        }
        let total_len = combined.len();
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        let result = get_text_msg(&mut combined, total_len, buf_msg, head_len).await.unwrap();

        assert_eq!(result.len(), N);
        for (i, msg) in result.iter().enumerate() {
            assert_eq!(msg.raw, format!("msg_{i}").as_bytes());
        }
    }

    // ========== Sticky + incomplete tail ==========

    #[tokio::test]
    async fn test_complete_plus_incomplete_body() {
        let complete = make_msg(MSG_TYPE_TEXT, b"complete", "user_b", "user_a");
        let incomplete_full = make_msg(MSG_TYPE_TEXT, b"incomplete_data_blah", "user_c", "user_a");
        let complete_len = complete.len();
        let head_len = head_size();

        let mut combined = complete.clone();
        // Append second message header + 5 bytes body after complete message, simulating incomplete sticky packet
        combined.extend_from_slice(&incomplete_full[..head_len + 5]);
        let total_len = combined.len();
        let buf_msg = new_buffer_msg();

        let result =
            get_text_msg(&mut combined, total_len, buf_msg.clone(), head_len).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw, b"complete");

        let saved = buf_msg.lock().await;
        assert!(!saved.is_empty(), "Incomplete remaining data should be saved to buffer_msg");
        assert_eq!(saved.len(), total_len - complete_len);
    }

    #[tokio::test]
    async fn test_complete_plus_head_only() {
        let complete = make_msg(MSG_TYPE_TEXT, b"payload", "user_b", "user_a");
        let incomplete = make_msg(MSG_TYPE_TEXT, b"big_payload_here", "user_c", "user_a");
        let head_len = head_size();

        // Complete message + header only (body completely missing)
        let mut combined = complete.clone();
        combined.extend_from_slice(&incomplete[..head_len]);
        let total_len = combined.len();
        let buf_msg = new_buffer_msg();

        let result =
            get_text_msg(&mut combined, total_len, buf_msg.clone(), head_len).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw, b"payload");

        let saved = buf_msg.lock().await;
        assert_eq!(saved.len(), head_len, "Header-only data should be saved to buffer_msg");
    }

    // ========== Cross-call recovery (buffer_msg sticky) ==========

    #[tokio::test]
    async fn test_carryover_completes_in_next_call() {
        let full_msg = make_msg(MSG_TYPE_TEXT, b"carryover_test_data", "user_b", "user_a");
        let head_len = head_size();
        let split_at = head_len + 5;

        // First call: header + partial body
        let mut first_buf = full_msg[..split_at].to_vec();
        let first_len = first_buf.len();
        let buf_msg = new_buffer_msg();

        let result1 =
            get_text_msg(&mut first_buf, first_len, buf_msg.clone(), head_len).await.unwrap();
        assert!(result1.is_empty(), "Incomplete message should not be parsed");

        // Second call: remaining body arrives, should拼接 with buffer_msg to complete
        let mut second_buf = full_msg[split_at..].to_vec();
        let second_len = second_buf.len();

        let result2 = get_text_msg(&mut second_buf, second_len, buf_msg, head_len).await.unwrap();
        assert_eq!(result2.len(), 1);
        assert_eq!(result2[0].raw, b"carryover_test_data");
        assert_eq!(result2[0].send_user, "user_a");
        assert_eq!(result2[0].recv_user, "user_b");
    }

    #[tokio::test]
    async fn test_carryover_multiple_fragments() {
        let full_msg = make_msg(MSG_TYPE_TEXT, b"multi_fragment_payload", "user_b", "user_a");
        let head_len = head_size();

        // Arrive in three parts: header / partial body / remaining body
        let split1 = head_len;
        let split2 = head_len + 7;
        let buf_msg = new_buffer_msg();

        let mut buf1 = full_msg[..split1].to_vec();
        let len1 = buf1.len();
        let r1 = get_text_msg(&mut buf1, len1, buf_msg.clone(), head_len).await.unwrap();
        assert!(r1.is_empty());

        let mut buf2 = full_msg[split1..split2].to_vec();
        let len2 = buf2.len();
        let r2 = get_text_msg(&mut buf2, len2, buf_msg.clone(), head_len).await.unwrap();
        assert!(r2.is_empty());

        let mut buf3 = full_msg[split2..].to_vec();
        let len3 = buf3.len();
        let r3 = get_text_msg(&mut buf3, len3, buf_msg.clone(), head_len).await.unwrap();
        assert_eq!(r3.len(), 1);
        assert_eq!(r3[0].raw, b"multi_fragment_payload");
    }

    #[tokio::test]
    async fn test_carryover_with_new_complete_messages() {
        let partial_full = make_msg(MSG_TYPE_TEXT, b"partial_message_xxxxx", "user_b", "user_a");
        let new_complete = make_msg(MSG_TYPE_TEXT, b"new_complete", "user_c", "user_a");
        let head_len = head_size();
        let split_at = head_len + 3;

        let buf_msg = new_buffer_msg();

        // Store incomplete data first
        let mut buf1 = partial_full[..split_at].to_vec();
        let len1 = buf1.len();
        let r1 = get_text_msg(&mut buf1, len1, buf_msg.clone(), head_len).await.unwrap();
        assert!(r1.is_empty());

        // Remaining part + one complete new message (sticky + recovery)
        let mut buf2 = partial_full[split_at..].to_vec();
        buf2.extend_from_slice(&new_complete);
        let len2 = buf2.len();

        let r2 = get_text_msg(&mut buf2, len2, buf_msg.clone(), head_len).await.unwrap();
        assert_eq!(r2.len(), 2);
        assert_eq!(r2[0].raw, b"partial_message_xxxxx");
        assert_eq!(r2[1].raw, b"new_complete");
    }

    // ========== Boundary conditions ==========

    #[tokio::test]
    async fn test_empty_buffer() {
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        let result = get_text_msg(&mut vec![], 0, buf_msg, head_len).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_buffer_shorter_than_head() {
        let head_len = head_size();
        let buf_msg = new_buffer_msg();
        let len = head_len - 1;
        let mut buf = vec![0u8; len];

        let result = get_text_msg(&mut buf, len, buf_msg.clone(), head_len).await.unwrap();
        assert!(result.is_empty());

        let saved = buf_msg.lock().await;
        assert_eq!(saved.len(), head_len - 1);
    }

    #[tokio::test]
    async fn test_exactly_head_size_buffer() {
        let msg = make_msg(MSG_TYPE_TEXT, b"hello_world", "user_b", "user_a");
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        // Exact header size -- but body_len > 0, body incomplete
        let len = head_len;
        let mut buf = msg[..head_len].to_vec();
        let result = get_text_msg(&mut buf, len, buf_msg.clone(), head_len).await.unwrap();
        assert!(result.is_empty());
        assert!(!buf_msg.lock().await.is_empty());
    }

    #[tokio::test]
    async fn test_exact_fit_message() {
        let msg = make_msg(MSG_TYPE_TEXT, b"exact_fit", "user_b", "user_a");
        let len = msg.len();
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        let mut buf = msg.clone();
        let result = get_text_msg(&mut buf, len, buf_msg.clone(), head_len).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw, b"exact_fit");
        assert!(buf_msg.lock().await.is_empty());
    }

    #[tokio::test]
    async fn test_exact_fit_two_messages() {
        let msg1 = make_msg(MSG_TYPE_TEXT, b"first", "user_b", "user_a");
        let msg2 = make_msg(MSG_TYPE_TEXT, b"second", "user_c", "user_a");
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        let mut combined = msg1.clone();
        combined.extend_from_slice(&msg2);
        let total = combined.len();

        let result = get_text_msg(&mut combined, total, buf_msg.clone(), head_len).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(buf_msg.lock().await.is_empty());
    }

    // ========== CRC check failure ==========

    #[tokio::test]
    async fn test_crc_mismatch_should_error() {
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        // Construct a valid body
        let body = TextQuicMsg {
            nano_id: "crc_test".to_string(),
            text_type: MSG_TYPE_TEXT,
            raw: b"payload".to_vec(),
            recv_user: "user_b".to_string(),
            send_user: "user_a".to_string(),
            timestamp: 123456789,
        };
        let body_bytes = bincode::serialize(&body).unwrap();

        // Header uses wrong CRC, body remains valid
        let head = HeadMsg {
            version: 1,
            crc: 0, // Intentionally wrong CRC
            body_len: body_bytes.len() as u32,
            message_type: MSG_TYPE_TEXT,
        };
        let mut buf = bincode::serialize(&head).unwrap();
        buf.extend_from_slice(&body_bytes);
        let len = buf.len();

        let result = get_text_msg(&mut buf, len, buf_msg, head_len).await;
        assert!(result.is_err(), "CRC mismatch should return error");
    }

    // ========== Corrupted data: header/body deserialization failure ==========

    #[tokio::test]
    async fn test_garbage_head_deserialization_fails() {
        let head_len = head_size();
        let buf_msg = new_buffer_msg();
        // All 0xFF data cannot be deserialized as HeadMsg
        let len = head_len + 10;
        let mut garbage: Vec<u8> = vec![0xFF; len];

        let result = get_text_msg(&mut garbage, len, buf_msg.clone(), head_len).await.unwrap();
        assert!(result.is_empty());

        let saved = buf_msg.lock().await;
        assert!(!saved.is_empty(), "Unparseable header data should be saved");
    }

    #[tokio::test]
    async fn test_valid_head_garbage_body_deserialization_fails() {
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        let fake_body_len: u32 = 50;
        let head =
            HeadMsg { version: 1, crc: 0, body_len: fake_body_len, message_type: MSG_TYPE_TEXT };
        let mut buf = bincode::serialize(&head).unwrap();
        buf.extend_from_slice(&vec![0xFF; fake_body_len as usize]);
        let len = buf.len();

        let result = get_text_msg(&mut buf, len, buf_msg.clone(), head_len).await.unwrap();
        assert!(result.is_empty(), "body 反序列化失败应返回空");
        assert!(!buf_msg.lock().await.is_empty());
    }

    // ========== 极端 body_len 值 ==========

    #[tokio::test]
    async fn test_body_len_exceeds_buffer() {
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        // 头部声称 body 很大，但实际缓冲区不够
        let head = HeadMsg { version: 1, crc: 0, body_len: 99999, message_type: MSG_TYPE_TEXT };
        let mut buf = bincode::serialize(&head).unwrap();
        buf.extend_from_slice(b"short");
        let len = buf.len();

        let result = get_text_msg(&mut buf, len, buf_msg.clone(), head_len).await.unwrap();
        assert!(result.is_empty());
        assert!(!buf_msg.lock().await.is_empty());
    }

    #[tokio::test]
    async fn test_body_len_with_body_empty_vec() {
        let head_len = head_size();

        // 构造 body_len 对应一个合法的空 raw 消息
        let body = TextQuicMsg {
            nano_id: "test_id".to_string(),
            text_type: MSG_TYPE_TEXT,
            raw: vec![],
            recv_user: "user_b".to_string(),
            send_user: "user_a".to_string(),
            timestamp: 0,
        };
        let body_bytes = bincode::serialize(&body).unwrap();
        let crc = X25.checksum(&body_bytes);

        let head = HeadMsg {
            version: 1,
            crc,
            body_len: body_bytes.len() as u32,
            message_type: MSG_TYPE_TEXT,
        };
        let mut buf = bincode::serialize(&head).unwrap();
        buf.extend_from_slice(&body_bytes);
        let len = buf.len();
        let buf_msg = new_buffer_msg();

        let result = get_text_msg(&mut buf, len, buf_msg, head_len).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw.len(), 0);
        assert_eq!(result[0].nano_id, "test_id");
    }

    // ========== buffer_msg 预存数据 + 新数据 ==========

    #[tokio::test]
    async fn test_buffer_msg_has_data_before_call() {
        let msg = make_msg(MSG_TYPE_TEXT, b"buffered", "user_b", "user_a");
        let head_len = head_size();
        let split_at = head_len + 4;

        let buf_msg = new_buffer_msg();
        // 预先存入不完整数据
        {
            let mut guard = buf_msg.lock().await;
            *guard = msg[..split_at].to_vec();
        }

        // 剩余部分到达
        let mut new_data = msg[split_at..].to_vec();
        let new_len = new_data.len();

        let result = get_text_msg(&mut new_data, new_len, buf_msg.clone(), head_len).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw, b"buffered");
        assert!(buf_msg.lock().await.is_empty());
    }

    // ========== 单 buffer 内：完整 → 不完整 → 完整 ==========

    #[tokio::test]
    async fn test_complete_incomplete_complete() {
        let complete1 = make_msg(MSG_TYPE_TEXT, b"first", "user_b", "user_a");
        let complete2 = make_msg(MSG_TYPE_TEXT, b"last", "user_d", "user_a");
        let head_len = head_size();

        // 构造一个"不完整消息"的头，其 body_len 极大，保证 body_size > length
        let fake_head = HeadMsg {
            version: 1,
            crc: 0,
            body_len: 99999, // 比整个 buffer 还大
            message_type: MSG_TYPE_TEXT,
        };
        let fake_head_bytes = bincode::serialize(&fake_head).unwrap();
        let partial_body_fragment: &[u8] = b"xx"; // 仅 2 字节假 body

        let mut combined = complete1.clone();
        combined.extend_from_slice(&fake_head_bytes);
        combined.extend_from_slice(partial_body_fragment);
        combined.extend_from_slice(&complete2);
        let total_len = combined.len();
        let buf_msg = new_buffer_msg();

        let result =
            get_text_msg(&mut combined, total_len, buf_msg.clone(), head_len).await.unwrap();

        // 应该只解析出第 1 条，第 2 条（假装不完整）和第 3 条一起被保存到 buffer_msg
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw, b"first");

        // complete1 之后的所有数据（fake_head + fragment + complete2）应保存在 buffer_msg
        let saved = buf_msg.lock().await;
        let complete1_len = complete1.len();
        assert!(!saved.is_empty());
        assert_eq!(saved.len(), total_len - complete1_len);
    }

    // ========== length < buffer 长度 ==========

    #[tokio::test]
    async fn test_length_smaller_than_buffer() {
        let msg = make_msg(MSG_TYPE_TEXT, b"test", "user_b", "user_a");
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        // buffer 比 length 大，只处理 length 范围内的数据
        let mut buf = msg.clone();
        buf.extend_from_slice(b"extra_garbage");
        let len = msg.len();

        let result = get_text_msg(&mut buf, len, buf_msg.clone(), head_len).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw, b"test");
    }

    // ========== 空 body ==========

    #[tokio::test]
    async fn test_empty_raw_body() {
        let msg = make_msg(MSG_TYPE_TEXT, b"", "user_b", "user_a");
        let len = msg.len();
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        let result = get_text_msg(&mut msg.clone(), len, buf_msg, head_len).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw, b"");
        assert_eq!(result[0].send_user, "user_a");
    }

    // ========== 大消息体 ==========

    #[tokio::test]
    async fn test_large_body() {
        let big_raw = vec![b'X'; 65535];
        let msg = make_msg(MSG_TYPE_TEXT, &big_raw, "user_b", "user_a");
        let len = msg.len();
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        let result = get_text_msg(&mut msg.clone(), len, buf_msg, head_len).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].raw.len(), 65535);
    }

    // ========== 不同消息类型粘包 ==========

    #[tokio::test]
    async fn test_sticky_different_message_types() {
        let msg_text = make_msg(MSG_TYPE_TEXT, b"text", "user_b", "user_a");
        let msg_ping =
            make_msg(common::utils::message_types::MSG_TYPE_PING, b"ping", "system", "user_a");
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        let mut combined = msg_text.clone();
        combined.extend_from_slice(&msg_ping);
        let total = combined.len();

        let result = get_text_msg(&mut combined, total, buf_msg, head_len).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text_type, MSG_TYPE_TEXT);
        assert_eq!(result[1].text_type, common::utils::message_types::MSG_TYPE_PING);
    }

    // ========== 多次分片到达，buffer_msg 逐次累加 ==========

    #[tokio::test]
    async fn test_carryover_accumulates_multiple_times() {
        let full_msg = make_msg(MSG_TYPE_TEXT, b"accumulated_payload", "user_b", "user_a");
        let head_len = head_size();
        let buf_msg = new_buffer_msg();

        // 分 5 次到达
        let chunk_size = full_msg.len() / 5;
        for i in 0..4 {
            let start = i * chunk_size;
            let end = ((i + 1) * chunk_size).min(full_msg.len());
            let mut chunk = full_msg[start..end].to_vec();
            let chunk_len = chunk.len();
            let r = get_text_msg(&mut chunk, chunk_len, buf_msg.clone(), head_len).await.unwrap();
            assert!(r.is_empty(), "第 {i} 次不应有完整消息");
        }

        // 最后一片完成
        let start = 4 * chunk_size;
        let mut last_chunk = full_msg[start..].to_vec();
        let last_len = last_chunk.len();
        let r = get_text_msg(&mut last_chunk, last_len, buf_msg.clone(), head_len).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].raw, b"accumulated_payload");
    }

    // ========== buffer_msg 消费后清空 ==========

    #[tokio::test]
    async fn test_carryover_cleared_after_consumption() {
        let msg = make_msg(MSG_TYPE_TEXT, b"clean_test", "user_b", "user_a");
        let head_len = head_size();
        let split_at = head_len + 2;
        let buf_msg = new_buffer_msg();

        // 不完整
        let mut buf1 = msg[..split_at].to_vec();
        let len1 = buf1.len();
        let _ = get_text_msg(&mut buf1, len1, buf_msg.clone(), head_len).await.unwrap();

        // 完成
        let mut buf2 = msg[split_at..].to_vec();
        let len2 = buf2.len();
        let _ = get_text_msg(&mut buf2, len2, buf_msg.clone(), head_len).await.unwrap();

        assert!(buf_msg.lock().await.is_empty());
    }
}
