use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::error;

pub fn get_now_time_stamp_as_secs() -> Result<i64, io::Error> {
    let start = SystemTime::now();
    match start.duration_since(UNIX_EPOCH) {
        Ok(duration) => Ok(duration.as_secs() as i64),
        Err(e) => {
            error!("time calculation error: {}", e);
            Err(io::Error::other(format!("time calculation error: {}", e)))
        }
    }
}

pub fn get_now_time_stamp_as_millis() -> Result<i64, io::Error> {
    // Get current time
    let start = SystemTime::now();

    // Convert current time to duration since UNIX_EPOCH
    match start.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            // Get milliseconds
            let timestamp_ms = duration.as_millis();
            // Convert milliseconds to i64
            let timestamp_long: i64 = timestamp_ms as i64;
            Ok(timestamp_long)
        }
        Err(e) => {
            error!("time calculation error: {}", e);
            // create an io::Error and return
            Err(io::Error::other(format!("time calculation error: {}", e)))
        }
    }
}
