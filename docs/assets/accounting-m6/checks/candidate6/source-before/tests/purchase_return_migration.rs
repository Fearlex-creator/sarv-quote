use rusqlite::{Connection, params};
use sarv_quote::{
    domain::{CURRENT_SCHEMA, State},
    store::Store,
};
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

// Produced by the preserved M5 library, including its genuine quotation and
// posted procurement/sales history; not a current State with a relabelled schema.
const ORIGINAL: &[u8] = include_bytes!(
    "../docs/assets/accounting-m6/review/diagnostics/old-m5-helper/actual-old-M5-v6.json"
);
fn original() -> State {
    let envelope: Value = serde_json::from_slice(ORIGINAL).unwrap();
    let state: State = serde_json::from_value(envelope["state"].clone()).unwrap();
    assert_eq!(state.schema, 6);
    state.validate().unwrap();
    state
}
fn seed(path: &Path) {
    let db = Connection::open(path.join("sarv.sqlite3")).unwrap();
    db.execute_batch("CREATE TABLE state(id INTEGER PRIMARY KEY CHECK(id=1),generation INTEGER NOT NULL,payload TEXT NOT NULL)").unwrap();
    db.execute(
        "INSERT INTO state VALUES(1,29,?1)",
        params![serde_json::to_string(&original()).unwrap()],
    )
    .unwrap();
}
fn row(path: &Path) -> (i64, String) {
    Connection::open(path.join("sarv.sqlite3"))
        .unwrap()
        .query_row("SELECT generation,payload FROM state", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap()
}
fn output(name: &str, bytes: &[u8]) {
    if let Ok(root) = std::env::var("SARV_M6_MIGRATION_ARTIFACT_DIR") {
        fs::create_dir_all(&root).unwrap();
        fs::write(Path::new(&root).join(name), bytes).unwrap();
    }
}
#[test]
fn genuine_schema6_prefix_recovery_one_upgrade_and_fresh_restores_are_exact() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let before = original();
    assert!(
        !before.quotes.is_empty(),
        "preserve genuine quotation prefix"
    );
    assert!(!before.accounting.sales.records.is_empty());
    assert!(!before.accounting.procurement.records.is_empty());
    let st = Store::open(dir.path()).unwrap();
    let mut expected = before.clone();
    expected.schema = CURRENT_SCHEMA;
    assert_eq!(
        serde_json::to_value(&st.state).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    assert_eq!(st.local_generation(), 30);
    let recoveries: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("before-schema7-")
        })
        .collect();
    assert_eq!(recoveries.len(), 1);
    let recovered: Value = serde_json::from_slice(&fs::read(&recoveries[0]).unwrap()).unwrap();
    let old: Value = serde_json::from_slice(ORIGINAL).unwrap();
    assert_eq!(
        recovered, old,
        "original checksum and full logical history retained"
    );
    let backup = dir.path().join("current-v7.json");
    st.backup(&backup).unwrap();
    let current: Value = serde_json::from_slice(&fs::read(&backup).unwrap()).unwrap();
    assert_eq!(current["format"], "sarv-accounting-backup-v7");
    for input in [&backup, &recoveries[0]] {
        let fresh = tempfile::tempdir().unwrap();
        let mut restored = Store::open(fresh.path()).unwrap();
        restored
            .restore(Store::restore_preview(input).unwrap())
            .unwrap();
        assert_eq!(
            serde_json::to_value(&restored.state).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
    }
    drop(st);
    assert_eq!(Store::open(dir.path()).unwrap().local_generation(), 30);
    assert_eq!(
        fs::read_dir(dir.path())
            .unwrap()
            .filter(|e| e
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("before-schema7-"))
            .count(),
        1
    );
    output("genuine-old-M5-v6.json", ORIGINAL);
    output(
        "verified-before-schema7-v6.json",
        &fs::read(&recoveries[0]).unwrap(),
    );
    output("current-v7.json", &fs::read(&backup).unwrap());
}
#[test]
fn old_envelope_cannot_claim_current_schema_even_with_a_valid_checksum() {
    let mut old: Value = serde_json::from_slice(ORIGINAL).unwrap();
    let mut state = original();
    state.schema = CURRENT_SCHEMA;
    old["state"] = serde_json::to_value(&state).unwrap();
    old["checksum"] = sarv_quote::imports::hash(&serde_json::to_vec(&state).unwrap()).into();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forged-v6.json");
    fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    assert!(Store::restore_preview(&path).is_err());
}
#[test]
fn migration_process_child() {
    if let Some(path) = std::env::var_os("SARV_M6_MIGRATION_CHILD_PROFILE") {
        Store::open(Path::new(&path)).unwrap();
    }
}
#[test]
fn killed_startup_behind_owned_writer_preserves_schema6_and_recovers_once() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let before = row(dir.path());
    let held = Connection::open(dir.path().join("sarv.sqlite3")).unwrap();
    held.execute_batch("PRAGMA journal_mode=WAL; BEGIN IMMEDIATE")
        .unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "migration_process_child", "--nocapture"])
        .env("SARV_M6_MIGRATION_CHILD_PROFILE", dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    assert!(child.try_wait().unwrap().is_none());
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    assert_eq!(row(dir.path()), before);
    held.execute_batch("ROLLBACK").unwrap();
    drop(held);
    let st = Store::open(dir.path()).unwrap();
    assert_eq!(st.state.schema, CURRENT_SCHEMA);
    assert_eq!(st.local_generation(), 30);
    drop(st);
    assert_eq!(Store::open(dir.path()).unwrap().local_generation(), 30);
    let db = Connection::open(dir.path().join("sarv.sqlite3")).unwrap();
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    output("interrupted-schema6-startup.json", &serde_json::to_vec_pretty(&serde_json::json!({"result":"passed","schema_before":6,"schema_after":CURRENT_SCHEMA,"generation_before":29,"generation_after":30,"scope":"owned child killed while startup waits behind owned writer, before migration; not hardware power loss","exact_old_row_after_kill":true,"integrity":"ok"})).unwrap());
}
#[test]
#[ignore = "requires retained immutable M5 executable in SARV_M5_COMPAT_BINARY"]
fn actual_m5_binary_refuses_schema7_without_modifying_committed_row() {
    let binary = std::env::var("SARV_M5_COMPAT_BINARY").unwrap();
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    drop(Store::open(dir.path()).unwrap());
    let before = row(dir.path());
    let result = Command::new(&binary)
        .env("SARV_DATA_DIR", dir.path())
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("نسخه") || stderr.contains("schema"),
        "{stderr}"
    );
    assert_eq!(row(dir.path()), before);
    output("actual-M5-schema7-rejection.json", &serde_json::to_vec_pretty(&serde_json::json!({"result":"passed","old_binary":binary,"rejected_schema":CURRENT_SCHEMA,"exit_code":result.status.code(),"stderr":stderr,"exact_row_unchanged":true})).unwrap());
}
