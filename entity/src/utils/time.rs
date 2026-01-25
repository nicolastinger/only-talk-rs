use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

use log::error;

pub fn get_now_time_stamp_as_millis() -> Result<i64, io::Error> {
    // 获取当前时间
    let start = SystemTime::now();

    // 将当前时间转换为自 UNIX_EPOCH 以来的持续时间
    match start.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            // 获取毫秒数
            let timestamp_ms = duration.as_millis();
            // 将毫秒数转换为 i64
            let timestamp_long: i64 = timestamp_ms as i64;
            Ok(timestamp_long)
        }
        Err(e) => {
            error!("时间计算错误: {}", e);
            // 创建一个 io::Error 并返回
            Err(io::Error::other(format!("时间计算错误: {}", e)))
        }
    }
}
