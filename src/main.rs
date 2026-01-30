#![deny(clippy::unwrap_used)]
use api::init_server;
use port_service::p2p_service::p2p_udp_service::run_udp_server;
use quic_service::init_server::start_server;
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter, Registry, prelude::*};

fn init_tracing() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::never("log", "rust_im.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = Registry::default()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_target(true)
                .with_thread_ids(true)
        )
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
        );

    tracing::subscriber::set_global_default(subscriber)
        .expect("设置全局 tracing subscriber 失败");

    guard
}

/// 主函数入口点，使用Tokio异步运行时
#[actix_web::main]
async fn main() {
    let _guard = init_tracing();

    info!("启动应用");

    run_udp_server().await.expect("启动UDP服务器失败");
    start_server().await.expect("启动quic服务失败");
    init_server::start_server()
        .await
        .unwrap_or_else(|err| error!("启动http服务失败 {}, 堆栈信息 {:?}", err, err.backtrace()));
}
