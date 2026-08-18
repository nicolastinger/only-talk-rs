use tracing::error;

/// 记录致命错误,等待 5 秒后 panic。
/// 用于无法恢复的启动错误,确保日志在进程结束前被刷新。
#[inline]
pub fn fatal_panic(msg: &str) -> ! {
    error!("致命错误: {}", msg);
    std::thread::sleep(std::time::Duration::from_secs(5));
    panic!("{}", msg);
}

/// fatal_panic 的异步版本
#[inline]
pub async fn fatal_panic_async(msg: &str) -> ! {
    error!("致命错误: {}", msg);
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    panic!("{}", msg);
}
