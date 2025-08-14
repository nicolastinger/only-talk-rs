use std::sync::Arc;
use anyhow::anyhow;
use log::{error, info};
use nanoid::nanoid;
use tokio::sync::{Mutex, MutexGuard};
use crate::common::quic_network_service::models::text_msg::{HeadMsg, MessageType, TextMsg, TextQuicMsg};
use crate::utils::time::get_now_time_stamp_as_millis;
use crate::X25;

//生成文本消息
pub fn generate_text_msg(
    text_type: u16,
    raw: Vec<u8>,
    recv_user: String,
    send_user: String,
) -> anyhow::Result<Vec<u8>> {
    let now = get_now_time_stamp_as_millis().unwrap_or_else(|_| -99999999999);
    let text_quic_msg = TextQuicMsg {
        nano_id: nanoid!(),
        text_type,
        raw,
        recv_user,
        send_user,
        timestamp: now
    };
    build_text(text_quic_msg)
}

//生成文本消息
pub fn generate_text_msg_with_id(
    nano_id: String,
    text_type: u16,
    raw: Vec<u8>,
    recv_user: String,
    send_user: String,
) -> anyhow::Result<Vec<u8>> {
    let now = get_now_time_stamp_as_millis().unwrap_or_else(|_| -99999999999);
    let text_quic_msg = TextQuicMsg {
        nano_id,
        text_type,
        raw,
        recv_user,
        send_user,
        timestamp: now
    };
    build_text(text_quic_msg)
}

//生成文本消息
pub fn generate_text_msg_with_time(
    nano_id: String,
    text_type: u16,
    raw: Vec<u8>,
    recv_user: String,
    send_user: String,
    timestamp: i64
) -> anyhow::Result<Vec<u8>> {
    let text_quic_msg = TextQuicMsg {
        nano_id,
        text_type,
        raw,
        recv_user,
        send_user,
        timestamp
    };
    build_text(text_quic_msg)
}

fn build_text(text_quic_msg: TextQuicMsg) -> anyhow::Result<Vec<u8>> {
    let meta_data = text_quic_msg.get_bytes()?;
    let crc = X25.checksum(&meta_data);
    let head_msg = HeadMsg {
        version: 1,
        crc,
        body_len: meta_data.len() as u32, // 消息体长度
        message_type: MessageType::Text as u16  // 消息类型
    };

    build_text_msg(&head_msg, &text_quic_msg)
}

//组装头部+消息体
pub fn build_text_msg<H: TextMsg, G: TextMsg>(text_head: &H, text_msg: &G) -> anyhow::Result<Vec<u8>> {
    let mut head_byte = text_head.get_bytes()?;
    let mut msg_byte = text_msg.get_bytes()?;
    head_byte.append(&mut msg_byte);
    Ok(head_byte)
}

//解析文本信息
pub async fn get_text_msg(buffer: &mut Vec<u8>,
                          mut length: usize,
                          buffer_msg: Arc<Mutex<Vec<u8>>>,
                          head_length: usize) -> anyhow::Result<Vec<TextQuicMsg>> {
    let mut result_vec:Vec<TextQuicMsg> = Vec::new();
    {
        // 获取锁并访问 Arc 中的数据
        let mut buffer_vec: MutexGuard<Vec<u8>> = buffer_msg.lock().await;

        // 如果 buffer_vec 中有数据，将其与 buffer 合并
        if !buffer_vec.is_empty() {
            // 创建一个新的 Vec<u8>，将 buffer_vec 和 buffer 的数据合并
            let mut combined_buffer = buffer_vec.clone(); // 复制 buffer_vec 的数据
            length += combined_buffer.len();
            combined_buffer.extend_from_slice(buffer); // 将 buffer 的数据追加到 combined_buffer
            *buffer = combined_buffer; // 将合并后的数据赋值给 buffer
            buffer_vec.clear();
        }
    } // buffer_vec 离开作用域，锁被释放

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
        let head_msg: HeadMsg = match bincode::deserialize(&head_msg_vec){
            Ok(msg) => msg,
            Err(error) => {
                error!("序列化粘包数据失败! {}",error);
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
        let body_msg: TextQuicMsg = match bincode::deserialize(&body_msg_vec) {
            Ok(msg) => msg,
            Err(error) => {
                error!("序列化粘包数据失败! {}",error);
                buffer_msg.lock().await.append(&mut buffer[round..length].to_vec());
                return Ok(result_vec);
            }
        };

        let crc = X25.checksum(body_msg_vec);
        if crc != head_msg.crc { Err(anyhow!("解析错误码失败!"))? }
        result_vec.push(body_msg);
        i += head_msg.body_len as usize;
    };
    Ok(result_vec)
}