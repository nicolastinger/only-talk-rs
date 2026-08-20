// 从 common crate 重新导出（共享消息类型的标准位置）
pub use common::utils::text_msg::{
    HeadMsg, MessageType, TextMsg, TextQuicMsg, X25, build_text_msg, generate_text_msg,
    generate_text_msg_with_id, generate_text_msg_with_time,
};
