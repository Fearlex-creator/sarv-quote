//! Software interruption tests. Only spawned test children are terminated.
//! Raw SQLite and staging-protocol cases identify their scope explicitly;
//! none of these tests simulates a physical power cut or storage hardware loss.
#![cfg(unix)]

use anyhow::{Context, Result};
use rusqlite::Connection;
use sarv_quote::{
    domain::{Customer, State, id},
    imports::hash,
    store::{Store, write_new},
};
use serde_json::json;
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

const PAYLOAD_SIZE: usize = 8 * 1024 * 1024;
const READY: &str = "SARV_RECOVERY_READY";

fn payload() -> Vec<u8> {
    vec![b'Q'; PAYLOAD_SIZE]
}
fn encoded(state: &State) -> Vec<u8> {
    serde_json::to_vec(state).unwrap()
}
fn initialized(directory: &Path) -> State {
    let mut store = Store::open(directory).unwrap();
    store
        .change(|state| {
            state.draft.customer = "baseline".into();
            Ok(())
        })
        .unwrap();
    store.state.clone()
}
fn integrity(directory: &Path) {
    let conn = Connection::open(directory.join("sarv.sqlite3")).unwrap();
    let result: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(result, "ok");
}
fn evidence(case: &str, value: serde_json::Value) {
    println!("RECOVERY_EVIDENCE {case}: {value}");
    if let Some(directory) = std::env::var_os("SARV_RECOVERY_PROOF_DIR") {
        let directory = PathBuf::from(directory);
        fs::create_dir_all(&directory).unwrap();
        let report = json!({
            "case": case, "recorded_at": sarv_quote::domain::now(),
            "platform": "Unix child-process SIGKILL; software recovery only",
            "evidence": value,
        });
        fs::write(
            directory.join(format!("{case}.json")),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
}

struct OwnedWorker {
    child: Child,
    output: Receiver<String>,
}
impl OwnedWorker {
    fn spawn(mode: &str, directory: &Path) -> Self {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "recovery_worker_entry",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("SARV_RECOVERY_WORKER_MODE", mode)
            .env("SARV_RECOVERY_WORKER_DIR", directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, output) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => {
                        if sender.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Self { child, output }
    }
    fn ready(&self) {
        let until = Instant::now() + Duration::from_secs(15);
        loop {
            let remaining = until.saturating_duration_since(Instant::now());
            let line = self
                .output
                .recv_timeout(remaining)
                .expect("owned worker did not reach READY within 15 seconds");
            if line.contains(READY) {
                return;
            }
        }
    }
    fn go(&mut self) {
        let input = self.child.stdin.as_mut().unwrap();
        input.write_all(b"G").unwrap();
        input.flush().unwrap();
    }
    fn kill(&mut self) {
        self.child
            .kill()
            .expect("kill only the spawned owned worker");
        let status = self.child.wait().unwrap();
        assert_eq!(status.signal(), Some(9), "test must observe actual SIGKILL");
    }
}
impl Drop for OwnedWorker {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
fn worker_ready(phase: &str) -> Result<()> {
    println!("{READY} {phase}");
    std::io::stdout().flush()?;
    std::io::stdin().read_exact(&mut [0u8; 1])?;
    Ok(())
}

/// This is a no-op in the parent harness. Child modes use only private profiles.
#[test]
fn recovery_worker_entry() {
    let Ok(mode) = std::env::var("SARV_RECOVERY_WORKER_MODE") else {
        return;
    };
    run_worker(
        &mode,
        &PathBuf::from(std::env::var_os("SARV_RECOVERY_WORKER_DIR").unwrap()),
    )
    .unwrap();
}
fn run_worker(mode: &str, directory: &Path) -> Result<()> {
    match mode {
        "store_candidate" => {
            let mut store = Store::open(directory)?;
            store.change(|state| {
                state.draft.customer = "uncommitted candidate".into();
                worker_ready("inside Store.change candidate closure")?;
                Ok(())
            })?;
        }
        "wal_uncommitted" => {
            let store = Store::open(directory)?;
            let mut candidate = store.state.clone();
            drop(store);
            candidate.draft.customer = "uncommitted WAL candidate".into();
            candidate.customers = (0..2000)
                .map(|_| Customer {
                    id: id(),
                    name: "x".repeat(384),
                    phone: String::new(),
                    ..Customer::default()
                })
                .collect();
            candidate.validate()?;
            let mut conn = Connection::open(directory.join("sarv.sqlite3"))?;
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA cache_size=10; PRAGMA cache_spill=ON;")?;
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute(
                "UPDATE state SET generation=generation+1,payload=?1 WHERE id=1",
                [serde_json::to_string(&candidate)?],
            )?;
            worker_ready("raw SQLite UPDATE spilled to WAL, no COMMIT")?;
            tx.commit()?;
        }
        "store_committed" => {
            let mut store = Store::open(directory)?;
            store.change(|state| {
                state.draft.customer = "committed by Store.change".into();
                Ok(())
            })?;
            worker_ready("Store.change returned after COMMIT")?;
        }
        "staging_only" | "published_only" => {
            let staging = directory.join(".sarv-write-recovery.tmp");
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&staging)?;
            if mode == "staging_only" {
                file.write_all(&payload()[..1024 * 1024])?;
            } else {
                file.write_all(&payload())?;
            }
            file.sync_all()?;
            drop(file);
            if mode == "published_only" {
                fs::hard_link(&staging, directory.join("quote.csv"))?;
            }
            worker_ready("exact staging/publication protocol boundary; no production hook")?;
        }
        "public_export" => {
            let bytes = payload();
            worker_ready("about to call public write_new")?;
            write_new(&directory.join("quote.csv"), &bytes)?;
            worker_ready("public write_new returned")?;
        }
        "existing_export" => {
            let rejected = write_new(&directory.join("quote.csv"), &payload()).is_err();
            anyhow::ensure!(rejected, "existing file should be rejected");
            worker_ready("public write_new rejected existing target")?;
        }
        "restore_blocked" | "restore_committed" => {
            let mut store = Store::open(directory)?;
            let mut candidate = store.state.clone();
            candidate.draft.customer = "restored candidate".into();
            if mode == "restore_blocked" {
                worker_ready("restore initialized, before external writer lock")?;
            }
            let safety = store.restore(candidate)?;
            anyhow::ensure!(safety.is_file(), "safety backup should exist");
            worker_ready("public Store.restore returned after COMMIT")?;
        }
        _ => anyhow::bail!("unknown worker mode"),
    }
    Ok(())
}

#[test]
fn kill_inside_store_candidate_does_not_leak_to_committed_state() {
    let dir = tempfile::tempdir().unwrap();
    let baseline = initialized(dir.path());
    let mut worker = OwnedWorker::spawn("store_candidate", dir.path());
    worker.ready();
    worker.kill();
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(encoded(&reopened.state), encoded(&baseline));
    integrity(dir.path());
    evidence(
        "store_candidate_before_commit",
        json!({"scope":"public Store.change; killed inside candidate closure", "sigkill":true, "candidate_visible_after_restart":false, "integrity_check":"ok", "state_sha256":hash(&encoded(&reopened.state))}),
    );
}

#[test]
fn sigkill_spilled_uncommitted_wal_recovers_previous_state() {
    let dir = tempfile::tempdir().unwrap();
    let baseline = initialized(dir.path());
    let mut worker = OwnedWorker::spawn("wal_uncommitted", dir.path());
    worker.ready();
    let wal_bytes = fs::metadata(dir.path().join("sarv.sqlite3-wal"))
        .unwrap()
        .len();
    assert!(wal_bytes > 32 * 1024, "exercise actual spilled WAL frames");
    worker.kill();
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(encoded(&reopened.state), encoded(&baseline));
    integrity(dir.path());
    evidence(
        "raw_sqlite_uncommitted_wal",
        json!({"scope":"raw SQLite transaction using Store schema/WAL/FULL settings; not an internal Store transaction fault hook", "sigkill":true, "uncommitted_wal_bytes_before_kill":wal_bytes, "candidate_visible_after_restart":false, "integrity_check":"ok"}),
    );
}

#[test]
fn sigkill_after_public_store_commit_keeps_complete_state() {
    let dir = tempfile::tempdir().unwrap();
    initialized(dir.path());
    let mut worker = OwnedWorker::spawn("store_committed", dir.path());
    worker.ready();
    worker.kill();
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.state.draft.customer, "committed by Store.change");
    integrity(dir.path());
    evidence(
        "store_after_commit",
        json!({"scope":"public Store.change returned before SIGKILL", "sigkill":true, "committed_value_retained":true, "integrity_check":"ok", "state_sha256":hash(&encoded(&reopened.state))}),
    );
}

#[test]
fn interrupted_staging_protocol_is_absent_before_link_and_complete_after_link() {
    let expected = hash(&payload());
    for mode in ["staging_only", "published_only"] {
        let dir = tempfile::tempdir().unwrap();
        let mut worker = OwnedWorker::spawn(mode, dir.path());
        worker.ready();
        worker.kill();
        let target = dir.path().join("quote.csv");
        if mode == "staging_only" {
            assert!(!target.exists());
        } else {
            assert_eq!(hash(&fs::read(&target).unwrap()), expected);
        }
        let orphan_bytes = fs::metadata(dir.path().join(".sarv-write-recovery.tmp"))
            .unwrap()
            .len();
        evidence(
            mode,
            json!({"scope":"exact same-directory write/sync/hard-link protocol, boundary implemented in test worker", "sigkill":true, "destination_exists":target.exists(), "destination_complete":mode == "published_only", "orphan_staging_bytes":orphan_bytes, "orphan_removed_with_isolated_temp_profile":true}),
        );
    }
}

#[test]
fn interrupted_public_export_is_absent_or_complete_and_never_overwrites() {
    let expected = hash(&payload());
    let mut trials = Vec::new();
    for delay_ms in [0, 1, 3, 8] {
        let dir = tempfile::tempdir().unwrap();
        let mut worker = OwnedWorker::spawn("public_export", dir.path());
        worker.ready();
        worker.go();
        if delay_ms != 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }
        worker.kill();
        let target = dir.path().join("quote.csv");
        let complete = if target.exists() {
            assert_eq!(hash(&fs::read(&target).unwrap()), expected);
            true
        } else {
            false
        };
        let orphans = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".sarv-write-")
            })
            .count();
        trials.push(json!({"kill_delay_ms":delay_ms,"destination_complete":complete,"destination_absent":!complete,"orphan_staging_files":orphans}));
    }
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("quote.csv");
    fs::write(&target, b"existing approved export").unwrap();
    let mut worker = OwnedWorker::spawn("existing_export", dir.path());
    worker.ready();
    worker.kill();
    assert_eq!(fs::read(target).unwrap(), b"existing approved export");
    evidence(
        "public_write_new_interrupted",
        json!({"scope":"actual public write_new API, four interruption timings", "payload_bytes":PAYLOAD_SIZE,"trials":trials,"existing_destination_preserved":true,"hardware_durability_tested":false}),
    );
}

fn safety_file(directory: &Path) -> Option<PathBuf> {
    fs::read_dir(directory)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("before-restore-")
                && path.extension().is_some_and(|ext| ext == "json")
        })
}

#[test]
fn killed_public_restore_after_safety_before_commit_retains_original_and_valid_backup() {
    let dir = tempfile::tempdir().unwrap();
    let baseline = initialized(dir.path());
    let mut worker = OwnedWorker::spawn("restore_blocked", dir.path());
    worker.ready();
    let blocker = Connection::open(dir.path().join("sarv.sqlite3")).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    worker.go();
    let until = Instant::now() + Duration::from_secs(2);
    let safety = loop {
        if let Some(path) = safety_file(dir.path()) {
            break path;
        }
        assert!(
            Instant::now() < until,
            "restore did not publish safety backup before timeout"
        );
        thread::sleep(Duration::from_millis(5));
    };
    // Complete valid JSON proves atomic publication, not just path creation.
    let saved = Store::restore_preview(&safety).unwrap();
    assert_eq!(encoded(&saved), encoded(&baseline));
    worker.kill();
    blocker.execute_batch("ROLLBACK").unwrap();
    drop(blocker);
    let mut reopened = Store::open(dir.path()).unwrap();
    assert_eq!(encoded(&reopened.state), encoded(&baseline));
    integrity(dir.path());
    reopened
        .change(|state| {
            state.draft.customer = "post-recovery edit".into();
            Ok(())
        })
        .unwrap();
    reopened.restore(saved).unwrap();
    assert_eq!(encoded(&reopened.state), encoded(&baseline));
    evidence(
        "public_restore_before_commit",
        json!({"scope":"actual Store.restore; separate owned SQLite writer lock blocks UPDATE after safety publication", "sigkill":true,"original_state_retained":true,"safety_checksum_and_state_valid":true,"safety_backup_successfully_restored_after_restart":true,"integrity_check":"ok"}),
    );
}

#[test]
fn killed_public_restore_after_commit_retains_restored_state_and_original_safety() {
    let dir = tempfile::tempdir().unwrap();
    let baseline = initialized(dir.path());
    let mut worker = OwnedWorker::spawn("restore_committed", dir.path());
    worker.ready();
    worker.kill();
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.state.draft.customer, "restored candidate");
    let safety = safety_file(dir.path())
        .context("committed restore should retain safety file")
        .unwrap();
    assert_eq!(
        encoded(&Store::restore_preview(&safety).unwrap()),
        encoded(&baseline)
    );
    integrity(dir.path());
    evidence(
        "public_restore_after_commit",
        json!({"scope":"actual Store.restore returned before SIGKILL", "sigkill":true,"restored_state_retained":true,"original_safety_checksum_and_state_valid":true,"integrity_check":"ok"}),
    );
}
