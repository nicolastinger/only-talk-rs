// Re-export from common crate (canonical location for shared message types)
pub use common::utils::text_msg::{
    HeadMsg, MessageType, TextMsg, TextQuicMsg, X25, build_text_msg, generate_text_msg,
    generate_text_msg_with_id, generate_text_msg_with_time,
};
