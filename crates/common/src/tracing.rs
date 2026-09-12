use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::Local;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::time::LocalTime;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, Registry, fmt};

const LOG_DIR: &str = "log";
const LOG_FILE_NAME: &str = "rust_im.log";
/// 保留最近多少天的日志
const KEEP_DAYS: usize = 30;

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

/// 按本地日期滚动的文件 writer
///
/// `tracing_appender::rolling` 只支持 UTC 分割, 这里按本地日期分割,
/// 与日志时间戳(LocalTime)保持一致。该 writer 仅被 tracing 的非阻塞线程持有,
/// 无需额外的锁。
struct LocalDailyFile {
    dir: PathBuf,
    prefix: String,
    date: String,
    file: File,
}

impl LocalDailyFile {
    fn open(dir: &str, prefix: &str) -> io::Result<Self> {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir)?;
        let date = Local::now().format("%Y-%m-%d").to_string();
        let file = open_dated(&dir, prefix, &date)?;
        prune_old_logs(&dir, prefix, KEEP_DAYS);
        Ok(Self { dir, prefix: prefix.to_string(), date, file })
    }

    fn rotate_if_needed(&mut self) {
        let today = Local::now().format("%Y-%m-%d").to_string();
        if today == self.date {
            return;
        }
        if let Ok(file) = open_dated(&self.dir, &self.prefix, &today) {
            let _ = self.file.flush();
            self.file = file;
            self.date = today;
            prune_old_logs(&self.dir, &self.prefix, KEEP_DAYS);
        }
    }
}

fn open_dated(dir: &Path, prefix: &str, date: &str) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(dir.join(format!("{}.{}", prefix, date)))
}

impl Write for LocalDailyFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.rotate_if_needed();
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

/// 删除超过保留天数的历史日志文件(仅匹配 `prefix.YYYY-MM-DD`)
fn prune_old_logs(dir: &Path, prefix: &str, keep: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let head = format!("{}.", prefix);
    let mut files: Vec<(String, PathBuf)> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_string();
            let date = name.strip_prefix(&head)?;
            let bytes = date.as_bytes();
            if bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
                Some((name, entry.path()))
            } else {
                None
            }
        })
        .collect();
    if files.len() <= keep {
        return;
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, path) in files.iter().take(files.len() - keep) {
        let _ = std::fs::remove_file(path);
    }
}

/// 启动时把历史遗留的未分割日志(旧的 `rust_im.log`)按最后修改日期重命名,
/// 避免新旧日志继续混在同一个文件里。
fn split_history_log() {
    let legacy = Path::new(LOG_DIR).join(LOG_FILE_NAME);
    let Ok(meta) = std::fs::metadata(&legacy) else {
        return;
    };
    if !meta.is_file() {
        return;
    }
    let Ok(modified) = meta.modified() else {
        return;
    };
    let modified: chrono::DateTime<Local> = modified.into();
    let date = modified.format("%Y-%m-%d").to_string();
    let dated = Path::new(LOG_DIR).join(format!("{}.{}", LOG_FILE_NAME, date));
    if dated.exists() {
        // 目标文件已存在则追加, 避免 Windows 下 rename 无法覆盖导致日志丢失
        if let (Ok(content), Ok(mut dst)) =
            (std::fs::read(&legacy), OpenOptions::new().append(true).open(&dated))
            && dst.write_all(&content).is_ok()
        {
            let _ = std::fs::remove_file(&legacy);
        }
    } else {
        let _ = std::fs::rename(&legacy, &dated);
    }
}

/// 初始化 tracing: 双通道(文件 + stdout),返回的 guard 必须在 main 中持有
pub fn init_tracing() -> WorkerGuard {
    split_history_log();
    let file_writer = LocalDailyFile::open(LOG_DIR, LOG_FILE_NAME).expect("初始化日志文件失败");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_writer);

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
