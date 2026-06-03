use tracing::error;

/// Log a fatal error, wait 5 seconds, then panic.
/// Used for unrecoverable startup errors, ensuring logs are flushed before termination.
#[inline]
pub fn fatal_panic(msg: &str) -> ! {
    error!("FATAL: {}", msg);
    std::thread::sleep(std::time::Duration::from_secs(5));
    panic!("{}", msg);
}

/// Async version of fatal_panic
#[inline]
pub async fn fatal_panic_async(msg: &str) -> ! {
    error!("FATAL: {}", msg);
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    panic!("{}", msg);
}
