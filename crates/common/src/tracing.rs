use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter, Registry, prelude::*};
use tracing_subscriber::fmt::time::LocalTime;

/// 初始化 tracing：文件 + 标准输出双通道，返回 guard 须在 main 中持有
pub fn init_tracing() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::never("log", "rust_im.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::new("info");

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

    tracing::subscriber::set_global_default(subscriber)
        .expect("设置全局 tracing subscriber 失败");

    guard
}
