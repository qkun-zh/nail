
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use chrono::Local;
use tracing_subscriber::EnvFilter;

use crate::other::conf::ServerConfig;

#[derive(Clone, Debug)]
struct LocalTime;

impl tracing_subscriber::fmt::time::FormatTime for LocalTime {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"))
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
        .with_writer(spawn_writer_thread(dir.to_path_buf()))
        .with_ansi(false)
        .with_timer(LocalTime)
        .init();
    install_panic_hook();
    Ok(())
}

pub fn record_startup_failure(e: &anyhow::Error) {
    let dir = crate::other::conf::AppConfig::load()
        .map(|c| PathBuf::from(c.server.log_dir))
        .unwrap_or_else(|_| PathBuf::from("log/back"));
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("startup-errors.log"))
    {
        let _ = writeln!(
            f,
            "{} ERROR nail_back startup failed: {e:#}",
            Local::now().format("%Y-%m-%d %H:%M:%S%.3f")
        );
    }
}

#[derive(Clone)]
struct ChannelWriter {
    tx: SyncSender<Vec<u8>>,
}

impl io::Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _ = self.tx.try_send(buf.to_vec());
        Ok(buf.len())
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

fn spawn_writer_thread(dir: PathBuf) -> ChannelWriter {
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(1024);
    std::thread::Builder::new()
        .name("nail-log-writer".into())
        .spawn(move || {
            let mut w = MinuteLogWriter::new(dir);
            while let Ok(line) = rx.recv() {
                if let Err(e) = w.write_all(&line).and_then(|_| w.flush()) {
                    eprintln!("[nail_log] log write failed: {e}");
                }
            }
        })
        .expect("failed to spawn log writer thread");
    ChannelWriter { tx }
}

fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default(info);
        tracing::error!(panic = %info, "thread panicked");
    }));
}

pub async fn prune_loop(dir: PathBuf, retention_days: u64, ring_size: usize, interval_secs: u64) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.tick().await;
    loop {
        interval.tick().await;
        let dir = dir.clone();
        tokio::task::spawn_blocking(move || prune(&dir, retention_days, ring_size))
            .await
            .map(|r| {
                if let Err(e) = r {
                    tracing::warn!(error = %e, "log retention prune failed");
                }
            })
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "log retention prune task panicked or cancelled");
            });
    }
}

pub fn prune(dir: &Path, retention_days: u64, ring_size: usize) -> Result<()> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(retention_days.saturating_mul(86_400)))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();
    collect_log_files(dir, &mut files)?;

    let mut kept = Vec::with_capacity(files.len());
    for (path, mtime) in files {
        if retention_days > 0 && mtime < cutoff {
            if let Err(e) = fs::remove_file(&path) {
                tracing::warn!(error = %e, path = %path.display(), "remove expired log failed");
            }
        } else {
            kept.push((path, mtime));
        }
    }

    if ring_size > 0 && kept.len() > ring_size {
        kept.sort_by_key(|(_, mtime)| *mtime);
        let overflow = kept.len() - ring_size;
        for (path, _) in kept.drain(..overflow) {
            if let Err(e) = fs::remove_file(&path) {
                tracing::warn!(error = %e, path = %path.display(), "remove ring-overflow log failed");
            }
        }
    }

    remove_empty_dirs(dir)?;
    Ok(())
}

struct MinuteLogWriter {
    dir: PathBuf,
    current: Option<(String, File)>,
}

impl MinuteLogWriter {
    fn new(dir: PathBuf) -> Self {
        Self { dir, current: None }
    }

    fn ensure_current(&mut self) -> io::Result<&mut File> {
        let now = Local::now();
        let rel = format!(
            "{}/{}/{}.log",
            now.format("%Y-%m-%d"),
            now.format("%H"),
            now.format("%Y-%m-%d_%H-%M"),
        );
        let rotated = match &self.current {
            Some((key, _)) => key != &rel,
            None => true,
        };
        if rotated {
            let path = self.dir.join(&rel);
            fs::create_dir_all(path.parent().expect("rel always has a parent"))?;
            let file = OpenOptions::new().create(true).append(true).open(&path)?;
            self.current = Some((rel, file));
        }
        Ok(&mut self.current.as_mut().expect("rotated above").1)
    }
}

impl Write for MinuteLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.ensure_current()?.write_all(buf)?;
        Ok(buf.len())
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
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            collect_log_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "log")
            && let Ok(meta) = fs::metadata(&path)
            && let Ok(mtime) = meta.modified()
        {
            out.push((path, mtime));
        }
    }
    Ok(())
}

fn remove_empty_dirs(dir: &Path) -> Result<()> {
    let mut dirs = Vec::new();
    collect_dirs(dir, &mut dirs)?;
    for d in dirs {
        let _ = fs::remove_dir(d);
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
