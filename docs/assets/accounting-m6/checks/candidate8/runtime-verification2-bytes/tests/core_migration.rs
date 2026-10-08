use rusqlite::Connection;
use sarv_quote::{
    domain::{CURRENT_SCHEMA, DateDisplay, State},
    imports::hash,
    store::Store,
};
use std::{
    fs,
    process::{Command, Stdio},
    time::Duration,
};

fn same_state(a: &State, b: &State) {
    assert_eq!(
        serde_json::to_vec(a).unwrap(),
        serde_json::to_vec(b).unwrap()
    );
}

fn predecessor() -> State {
    let envelope: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/assets/accounting-m2/migration/current-v3.json"
    ))
    .unwrap();
    let mut state: State = serde_json::from_value(envelope["state"].clone()).unwrap();
    assert_eq!(state.schema, 3);
    state.settings.date_display = DateDisplay::Jalali;
    state.validate().unwrap();
    state
}
fn seed(path: &std::path::Path, original: &State) {
    let conn = Connection::open(path.join("sarv.sqlite3")).unwrap();
    conn.execute_batch("CREATE TABLE state(id INTEGER PRIMARY KEY CHECK(id=1),generation INTEGER NOT NULL,payload TEXT NOT NULL);").unwrap();
    conn.execute(
        "INSERT INTO state VALUES(1,37,?1)",
        [serde_json::to_string(original).unwrap()],
    )
    .unwrap();
}
fn recovery(path: &std::path::Path) -> Option<std::path::PathBuf> {
    fs::read_dir(path)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!(
                    "before-schema{}-",
                    sarv_quote::domain::CURRENT_SCHEMA
                ))
                && p.extension().is_some_and(|x| x == "json")
        })
}
fn row(path: &std::path::Path) -> (i64, String) {
    Connection::open(path.join("sarv.sqlite3"))
        .unwrap()
        .query_row("SELECT generation,payload FROM state", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap()
}

#[test]
fn schema_three_upgrade_preserves_jalali_and_exact_history_with_v3_recovery_and_current_restore() {
    let dir = tempfile::tempdir().unwrap();
    let original = predecessor();
    seed(dir.path(), &original);
    let old_row = row(dir.path());
    let store = Store::open(dir.path()).unwrap();
    let mut expected = original.clone();
    expected.schema = CURRENT_SCHEMA;
    same_state(&store.state, &expected);
    assert_eq!(store.local_generation(), 38);
    assert!(store.state.accounting.core.is_empty());
    assert!(
        !serde_json::to_string(&store.state.accounting)
            .unwrap()
            .contains("\"core\"")
    );
    let recovery = recovery(dir.path()).unwrap();
    let envelope: serde_json::Value =
        serde_json::from_slice(&fs::read(&recovery).unwrap()).unwrap();
    assert_eq!(envelope["format"], "sarv-accounting-backup-v3");
    let saved: State = serde_json::from_value(envelope["state"].clone()).unwrap();
    same_state(&saved, &original);
    assert_eq!(
        envelope["checksum"],
        hash(&serde_json::to_vec(&saved).unwrap())
    );
    same_state(&Store::restore_preview(&recovery).unwrap(), &expected);
    let backup = dir.path().join("current-v5.json");
    store.backup(&backup).unwrap();
    let envelope: serde_json::Value = serde_json::from_slice(&fs::read(&backup).unwrap()).unwrap();
    assert_eq!(envelope["format"], "sarv-accounting-backup-v7");
    let fresh = tempfile::tempdir().unwrap();
    let mut restored = Store::open(fresh.path()).unwrap();
    restored
        .restore(Store::restore_preview(&backup).unwrap())
        .unwrap();
    same_state(&restored.state, &store.state);
    drop(restored);
    same_state(&Store::open(fresh.path()).unwrap().state, &expected);
    drop(store);
    assert_eq!(Store::open(dir.path()).unwrap().local_generation(), 38);
    assert_eq!(
        fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(&format!(
                "before-schema{}-",
                sarv_quote::domain::CURRENT_SCHEMA
            )))
            .count(),
        1
    );
    assert_eq!(old_row.0, 37);
    if let Ok(output) = std::env::var("SARV_M4_LEGACY_CORE_MIGRATION_ARTIFACT_DIR") {
        fs::create_dir_all(&output).unwrap();
        let p = std::path::Path::new(&output);
        fs::copy(recovery, p.join("verified-before-schema7-v3.json")).unwrap();
        fs::copy(backup, p.join("current-v5.json")).unwrap();
        fs::write(p.join("verification.json"),serde_json::to_vec_pretty(&serde_json::json!({"result":"passed","schema_before":3,"schema_after":CURRENT_SCHEMA,"generation_before":37,"generation_after":38,"quotes":original.quotes.len(),"journals":original.accounting.journals.len(),"date_display":"Jalali","core_installed_into_old_company":false,"full_state_equal_except_schema":true,"fresh_restore_and_restart_equal":true,"quote_sha256":hash(&serde_json::to_vec(&original.quotes).unwrap()),"ledger_sha256":hash(&serde_json::to_vec(&original.accounting).unwrap())})).unwrap()).unwrap();
    }
}

#[test]
fn core_migration_child() {
    if let Some(path) = std::env::var_os("SARV_MIGRATION_CHILD_PROFILE") {
        Store::open(std::path::Path::new(&path)).unwrap();
    }
}

#[test]
fn interrupted_upgrade_waiting_for_startup_write_retains_exact_legacy_row() {
    let dir = tempfile::tempdir().unwrap();
    let original = predecessor();
    seed(dir.path(), &original);
    let held = Connection::open(dir.path().join("sarv.sqlite3")).unwrap();
    held.execute_batch("PRAGMA journal_mode=WAL; BEGIN IMMEDIATE;")
        .unwrap();
    let old = row(dir.path());
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "core_migration_child", "--nocapture"])
        .env("SARV_MIGRATION_CHILD_PROFILE", dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // Store startup's INSERT OR IGNORE waits behind the held transaction.
    // This is interruption before the migration writes, not simulated power loss.
    std::thread::sleep(Duration::from_millis(150));
    assert!(child.try_wait().unwrap().is_none());
    child.kill().unwrap();
    let status = child.wait().unwrap();
    assert!(!status.success());
    assert_eq!(row(dir.path()), old);
    assert!(recovery(dir.path()).is_none());
    held.execute_batch("ROLLBACK").unwrap();
    drop(held);
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.local_generation(), 38);
    let mut expected = original.clone();
    expected.schema = CURRENT_SCHEMA;
    same_state(&reopened.state, &expected);
    let envelope: serde_json::Value =
        serde_json::from_slice(&fs::read(recovery(dir.path()).unwrap()).unwrap()).unwrap();
    assert_eq!(envelope["state"], serde_json::to_value(original).unwrap());
    let conn = Connection::open(dir.path().join("sarv.sqlite3")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn mismatched_backup_versions_and_future_schema_never_replace_state() {
    let dir = tempfile::tempdir().unwrap();
    let state = State::default();
    let bytes = serde_json::to_vec(&state).unwrap();
    for format in ["sarv-accounting-backup-v2", "sarv-accounting-backup-v3"] {
        let target = dir.path().join(format);
        fs::write(
            &target,
            serde_json::to_vec(
                &serde_json::json!({"format":format,"checksum":hash(&bytes),"state":state}),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(Store::restore_preview(&target).is_err());
    }
    let mut future = state;
    future.schema = CURRENT_SCHEMA + 1;
    seed(dir.path(), &future);
    let original = row(dir.path());
    assert!(Store::open(dir.path()).is_err());
    assert_eq!(row(dir.path()), original);
}
