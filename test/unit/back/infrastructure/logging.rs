use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::infrastructure::logging::prune;

fn temp_log_dir() -> PathBuf {
    std::env::temp_dir().join(format!("nail_log_{}", uuid::Uuid::now_v7()))
}

fn write_log(dir: &std::path::Path, name: &str, modified: SystemTime) -> PathBuf {
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).expect("create log file");
    file.write_all(b"log line\n").expect("write log file");
    file.set_modified(modified).expect("set modified time");
    path
}

fn days_ago(days: u64) -> SystemTime {
    SystemTime::now()
        .checked_sub(Duration::from_secs(days * 86_400))
        .expect("past time")
}

#[test]
fn prune_removes_files_older_than_the_retention_days() {
    let dir = temp_log_dir();
    std::fs::create_dir_all(&dir).expect("create dir");
    let expired = write_log(&dir, "expired.log", days_ago(2));
    let fresh = write_log(&dir, "fresh.log", SystemTime::now());

    prune(&dir, 1, 0).expect("prune");

    assert!(!expired.exists(), "expired log must be removed");
    assert!(fresh.exists(), "fresh log must be kept");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn prune_enforces_the_ring_size_by_removing_the_oldest_kept_files() {
    let dir = temp_log_dir();
    std::fs::create_dir_all(&dir).expect("create dir");
    let oldest = write_log(&dir, "oldest.log", days_ago(3));
    let middle = write_log(&dir, "middle.log", days_ago(2));
    let newest = write_log(&dir, "newest.log", days_ago(1));

    prune(&dir, 0, 2).expect("prune");

    assert!(!oldest.exists(), "oldest of three must be removed by the ring");
    assert!(middle.exists(), "middle log must be kept within the ring");
    assert!(newest.exists(), "newest log must be kept within the ring");

    let _ = std::fs::remove_dir_all(&dir);
}
