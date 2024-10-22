use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct QuicMsg {
    pub id: String,
    pub connection_id: String,
    pub connection_type: String,  //文字,二进制数据，视频二进制
    pub connection_address: String,  //连接ip地址
    pub msg_buffer: Vec<u8>,  //缓冲区源数据
}