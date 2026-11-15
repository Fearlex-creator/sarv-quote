//! Append-only local log beside the books: `<data>/ir.sarv.accounting/logs/sarv.log`.
//! Never write payloads, amounts or names — action names and error text only.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const MAX_LOG_BYTES: u64 = 1024 * 1024;
const MAX_CRASH_FILES: usize = 10;

static LOGS_DIR: OnceLock<PathBuf> = OnceLock::new();

fn prepare_logs_dir(books_dir: &Path) -> Option<PathBuf> {
    if books_dir.as_os_str().is_empty() {
        return None;
    }
    let dir = books_dir.parent().unwrap_or(books_dir).join("logs");
    if fs::create_dir_all(&dir).is_err() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    }
    rotate_if_needed(&dir, MAX_LOG_BYTES);
    Some(dir)
}

fn rotate_if_needed(dir: &Path, max_bytes: u64) {
    let log_path = dir.join("sarv.log");
    let Ok(meta) = fs::metadata(&log_path) else {
        return;
    };
    if meta.len() > max_bytes {
        let rotated = dir.join("sarv.log.1");
        let _ = fs::remove_file(&rotated);
        let _ = fs::rename(&log_path, &rotated);
    }
}

fn write_line_to(dir: &Path, level: &str, message: &str) {
    let ts = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z");
    let clean: String = message.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
    let entry = format!("{ts} {level:<5} {clean}\n");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(dir.join("sarv.log")) {
        let _ = f.write_all(entry.as_bytes());
    }
}

/// Remembers the logs directory in a `OnceLock`, creates it (`0700` on Unix),
/// and rotates `sarv.log` -> `sarv.log.1` if larger than 1 MiB.
pub fn init(books_dir: &Path) {
    if LOGS_DIR.get().is_some() {
        return;
    }
    if let Some(dir) = prepare_logs_dir(books_dir) {
        let _ = LOGS_DIR.set(dir);
    }
}

pub fn logs_dir() -> Option<PathBuf> {
    LOGS_DIR.get().cloned()
}

/// Best-effort append-only log line; no-op before `init`, never panics.
pub fn line(level: &str, message: &str) {
    if let Some(dir) = LOGS_DIR.get() {
        write_line_to(dir, level, message);
    }
}

fn panic_message(info: &std::panic::PanicHookInfo<'_>) -> String {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

fn prune_crash_files(dir: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("crash-") && n.ends_with(".txt"))
        })
        .collect();
    if files.len() <= keep {
        return;
    }
    files.sort();
    let remove_count = files.len() - keep;
    for old in files.into_iter().take(remove_count) {
        let _ = fs::remove_file(old);
    }
}

/// Write a crash report into `<logs>/crash-<YYYYMMDD-HHMMSS>.txt`, append a summary
/// to `sarv.log`, and keep at most 10 crash files.
pub fn write_crash_report(info: &std::panic::PanicHookInfo<'_>, version: &str, git_sha: &str) -> Option<PathBuf> {
    let dir = LOGS_DIR.get()?;
    let now = chrono::Local::now();
    let stamp = now.format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("crash-{stamp}.txt"));
    let msg = panic_message(info);
    let loc = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "unknown".to_string());
    let bt = std::backtrace::Backtrace::force_capture();
    let body = format!(
        "Timestamp: {}\nVersion: {} ({})\nOS: {}\nLocation: {}\nMessage: {}\n\nBacktrace:\n{}\n",
        now.format("%Y-%m-%dT%H:%M:%S%:z"),
        version,
        git_sha,
        std::env::consts::OS,
        loc,
        msg,
        bt
    );
    let _ = fs::write(&path, body.as_bytes());
    write_line_to(dir, "ERROR", &format!("panic at {loc}: {msg}"));
    prune_crash_files(dir, MAX_CRASH_FILES);
    Some(path)
}

/// If any unseen `crash-*.txt` (not ending with `-seen.txt`) exists in `logs_dir()`,
/// mark all unseen crash files as `-seen.txt` and return the newest renamed path.
pub fn take_unseen_crash() -> Option<PathBuf> {
    let dir = LOGS_DIR.get()?;
    let entries = fs::read_dir(dir).ok()?;
    let mut unseen: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("crash-") && n.ends_with(".txt") && !n.ends_with("-seen.txt"))
        })
        .collect();
    if unseen.is_empty() {
        return None;
    }
    unseen.sort();
    let mut newest_seen = None;
    for path in unseen {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let seen_path = dir.join(format!("{stem}-seen.txt"));
            if fs::rename(&path, &seen_path).is_ok() {
                newest_seen = Some(seen_path);
            }
        }
    }
    newest_seen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_writes_timestamp_and_level() {
        let tmp = tempfile::tempdir().unwrap();
        let books = tmp.path().join("books");
        fs::create_dir_all(&books).unwrap();
        let logs = prepare_logs_dir(&books).expect("logs dir");
        write_line_to(&logs, "INFO", "hello sarv");
        let content = fs::read_to_string(logs.join("sarv.log")).unwrap();
        assert!(content.contains("INFO  hello sarv"), "unexpected log content: {content}");
        // RFC3339-like timestamp prefix YYYY-MM-DDTHH:MM:SS
        assert_eq!(content.chars().nth(4), Some('-'));
        assert_eq!(content.chars().nth(10), Some('T'));
    }

    #[test]
    fn rotates_above_one_mib() {
        let tmp = tempfile::tempdir().unwrap();
        let books = tmp.path().join("books");
        fs::create_dir_all(&books).unwrap();
        let logs = books.parent().unwrap().join("logs");
        fs::create_dir_all(&logs).unwrap();
        let big = vec![b'a'; (MAX_LOG_BYTES as usize) + 64];
        fs::write(logs.join("sarv.log"), &big).unwrap();

        let prepared = prepare_logs_dir(&books).expect("logs dir");
        assert!(!prepared.join("sarv.log").exists());
        assert!(prepared.join("sarv.log.1").exists());
        assert_eq!(fs::metadata(prepared.join("sarv.log.1")).unwrap().len(), big.len() as u64);
    }

    #[test]
    fn line_before_init_is_safe_noop() {
        let slot: OnceLock<PathBuf> = OnceLock::new();
        if let Some(dir) = slot.get() {
            write_line_to(dir, "INFO", "should not run");
        }
        line("INFO", "safe even if uninitialized");
    }
}
