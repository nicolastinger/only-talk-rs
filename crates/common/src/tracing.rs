use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::time::LocalTime;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, Registry, fmt};

fn read_log_level_from_config() -> String {
    std::fs::read_to_string("./config/app_config.toml")
        .ok()
        .and_then(|content| {
            content.parse::<toml::Value>().ok().and_then(|v| {
                v.get("server")
                    .and_then(|s| s.get("log_level"))
                    .and_then(|l| l.as_str())
                    .map(|s| s.to_string())
            })
        })
        .unwrap_or_else(|| "info".to_string())
}

/// 初始化 tracing: 双通道(文件 + stdout),返回的 guard 必须在 main 中持有
pub fn init_tracing() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::never("log", "rust_im.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let log_level = read_log_level_from_config();
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&log_level));

    let timer = LocalTime::rfc_3339();

    let subscriber = Registry::default()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_target(true)
                .with_thread_ids(true)
                .with_timer(timer.clone()),
        )
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_timer(timer),
        );

    tracing::subscriber::set_global_default(subscriber).unwrap_or_else(|e| {
        let msg = format!("设置全局 tracing 订阅器失败: {}", e);
        tracing::error!("致命错误: {}", msg);
        std::thread::sleep(std::time::Duration::from_secs(5));
        panic!("{}", msg);
    });

    guard
}
