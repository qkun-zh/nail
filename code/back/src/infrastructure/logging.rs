use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};
use tracing_subscriber::EnvFilter;

use crate::infrastructure::config::server::ServerConfig;

const LOG_TIME_FORMAT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]");
const DAY_FORMAT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]");
const HOUR_FORMAT: &[time::format_description::FormatItem<'_>] = format_description!("[hour]");
const MINUTE_FORMAT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]_[hour]-[minute]");

#[derive(Clone, Debug)]
struct OffsetTime {
    offset: UtcOffset,
}

impl OffsetTime {
    fn new(offset_seconds: i32) -> Result<Self> {
        Ok(Self {
            offset: UtcOffset::from_whole_seconds(offset_seconds).context("invalid timezone offset")?,
        })
    }
}

impl tracing_subscriber::fmt::time::FormatTime for OffsetTime {
    fn format_time(
        &self,
        writer: &mut tracing_subscriber::fmt::format::Writer<'_>,
    ) -> std::fmt::Result {
        let now = OffsetDateTime::now_utc().to_offset(self.offset);
        match now.format(LOG_TIME_FORMAT) {
            Ok(rendered) => write!(writer, "{rendered}"),
            Err(_) => write!(writer, "<invalid time>"),
        }
    }
}

pub fn init(config: &ServerConfig) -> Result<()> {
    let dir = Path::new(&config.log_dir);
    fs::create_dir_all(dir).with_context(|| format!("create log dir {}", dir.display()))?;
    prune(dir, config.log_retention_days, config.log_max_file_count)?;

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_filter));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(spawn_writer_thread(dir.to_path_buf(), config.timezone_offset_seconds)?)
        .with_ansi(false)
        .with_timer(OffsetTime::new(config.timezone_offset_seconds)?)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to init tracing: {error}"))?;
    install_panic_hook();
    Ok(())
}

pub fn record_startup_failure(error: &anyhow::Error) {
    let dir = crate::infrastructure::config::AppConfig::load()
        .map(|config| PathBuf::from(config.server.log_dir))
        .unwrap_or_else(|_| PathBuf::from("log/back"));
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("startup-errors.log"))
    else {
        return;
    };
    let timestamp = OffsetDateTime::now_utc().format(LOG_TIME_FORMAT);
    let _ = writeln!(
        file,
        "{} ERROR nail_back startup failed: {error:#}",
        timestamp.unwrap_or_else(|_| "unknown time".to_string())
    );
}

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_hook(info);
        tracing::error!(panic = %info, "thread panicked");
    }));
}

#[derive(Clone)]
struct ChannelWriter {
    tx: SyncSender<Vec<u8>>,
}

impl io::Write for ChannelWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let _ = self.tx.try_send(buffer.to_vec());
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for ChannelWriter {
    type Writer = ChannelWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn spawn_writer_thread(dir: PathBuf, offset_seconds: i32) -> Result<ChannelWriter> {
    let offset = UtcOffset::from_whole_seconds(offset_seconds).context("invalid timezone offset")?;
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(1024);
    std::thread::Builder::new()
        .name("nail-log-writer".to_string())
        .spawn(move || {
            let mut writer = MinuteLogWriter::new(dir, offset);
            while let Ok(line) = rx.recv() {
                if let Err(error) = writer.write_all(&line).and_then(|_| writer.flush()) {
                    eprintln!("[nail_log] log write failed: {error}");
                }
            }
        })
        .context("failed to spawn log writer thread")?;
    Ok(ChannelWriter { tx })
}

pub async fn prune_loop(
    dir: PathBuf,
    retention_days: u64,
    ring_size: usize,
    interval_secs: u64,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.tick().await;
    loop {
        interval.tick().await;
        let dir = dir.clone();
        let result = tokio::task::spawn_blocking(move || prune(&dir, retention_days, ring_size))
            .await;
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(error = %error, "log retention prune failed"),
            Err(error) => tracing::error!(error = %error, "log retention prune task failed"),
        }
    }
}

pub fn prune(dir: &Path, retention_days: u64, ring_size: usize) -> Result<()> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(retention_days.saturating_mul(86_400)))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();
    collect_log_files(dir, &mut files)?;

    let mut kept = Vec::with_capacity(files.len());
    for (path, modified) in files {
        if retention_days > 0 && modified < cutoff {
            if let Err(error) = fs::remove_file(&path) {
                tracing::warn!(error = %error, path = %path.display(), "remove expired log failed");
            }
        } else {
            kept.push((path, modified));
        }
    }

    if ring_size > 0 && kept.len() > ring_size {
        kept.sort_by_key(|(_, modified)| *modified);
        let overflow = kept.len() - ring_size;
        for (path, _) in kept.drain(..overflow) {
            if let Err(error) = fs::remove_file(&path) {
                tracing::warn!(error = %error, path = %path.display(), "remove ring-overflow log failed");
            }
        }
    }

    remove_empty_dirs(dir)?;
    Ok(())
}

struct MinuteLogWriter {
    dir: PathBuf,
    offset: UtcOffset,
    current: Option<(String, File)>,
}

impl MinuteLogWriter {
    fn new(dir: PathBuf, offset: UtcOffset) -> Self {
        Self {
            dir,
            offset,
            current: None,
        }
    }

    fn ensure_current(&mut self) -> io::Result<&mut File> {
        let now = OffsetDateTime::now_utc().to_offset(self.offset);
        let day = now
            .format(DAY_FORMAT)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        let hour = now
            .format(HOUR_FORMAT)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        let minute = now
            .format(MINUTE_FORMAT)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        let relative = format!("{day}/{hour}/{minute}.log");

        let rotated = self
            .current
            .as_ref()
            .is_none_or(|(key, _)| key != &relative);
        if rotated {
            let path = self.dir.join(&relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let file = OpenOptions::new().create(true).append(true).open(&path)?;
            self.current = Some((relative, file));
        }
        let (_, file) = self.current.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::Other, "log writer has no open file")
        })?;
        Ok(file)
    }
}

impl Write for MinuteLogWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.ensure_current()?.write_all(buffer)?;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.current {
            Some((_, file)) => file.flush(),
            None => Ok(()),
        }
    }
}

fn collect_log_files(dir: &Path, out: &mut Vec<(PathBuf, SystemTime)>) -> Result<()> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            collect_log_files(&path, out)?;
        } else if path.extension().is_some_and(|extension| extension == "log") {
            if let Ok(metadata) = fs::metadata(&path)
                && let Ok(modified) = metadata.modified()
            {
                out.push((path, modified));
            }
        }
    }
    Ok(())
}

fn remove_empty_dirs(dir: &Path) -> Result<()> {
    let mut dirs = Vec::new();
    collect_dirs(dir, &mut dirs)?;
    for directory in dirs {
        let _ = fs::remove_dir(directory);
    }
    Ok(())
}

fn collect_dirs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            let path = entry.path();
            collect_dirs(&path, out)?;
            out.push(path);
        }
    }
    Ok(())
}
