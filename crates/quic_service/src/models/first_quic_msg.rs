use serde::{Deserialize, Serialize};

use crate::models::quic_connection::ConnectionType;

// QUIC initial packet
#[derive(Debug, Serialize, Deserialize)]
pub struct FirstQuicMsg {
    pub token: String,             // User token
    pub uuid: String,              // User account
    pub msg_type: ConnectionType,  // Stream data type: text, image, video, other
    pub text_serde_struct: String, // Text type serialized struct
    pub dyn_buffer_size: usize,    // Buffer size
    pub dyn_header_size: usize,    // Header size
}

impl FirstQuicMsg {
    pub(crate) fn new() -> FirstQuicMsg {
        FirstQuicMsg {
            token: "".to_string(),
            uuid: "".to_string(),
            msg_type: ConnectionType::Text,
            text_serde_struct: "".to_string(),
            dyn_buffer_size: 0,
            dyn_header_size: 0,
        }
    }
}
