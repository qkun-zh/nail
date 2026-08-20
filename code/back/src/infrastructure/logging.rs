use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use tracing_appender::non_blocking::NonBlockingBuilder;
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::EnvFilter;

use crate::infrastructure::config::logging::LoggingConfig;

pub fn init(config: &LoggingConfig) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let dir = Path::new(&config.dir);
    fs::create_dir_all(dir).with_context(|| format!("create log dir {}", dir.display()))?;

    let file_appender = RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::HOURLY)
        .filename_prefix("nail_server")
        .max_log_files(usize::try_from(config.retention_days).unwrap_or(usize::MAX) * 24)
        .build(dir)
        .context("create log file appender")?;

    let (non_blocking, guard) = NonBlockingBuilder::default()
        .thread_name("nail-log-writer")
        .lossy(false)
        .finish(file_appender);

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.filter));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to init tracing: {error}"))?;

    Ok(guard)
}
