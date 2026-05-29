use tracing::error;

/// 记录致命错误，等待 5 秒后 panic。
/// 用于不可恢复的启动期错误，确保日志被刷盘后程序才终止。
#[inline]
pub fn fatal_panic(msg: &str) -> ! {
    error!("FATAL: {}", msg);
    std::thread::sleep(std::time::Duration::from_secs(5));
    panic!("{}", msg);
}

/// 异步版本的 fatal_panic
#[inline]
pub async fn fatal_panic_async(msg: &str) -> ! {
    error!("FATAL: {}", msg);
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    panic!("{}", msg);
}
