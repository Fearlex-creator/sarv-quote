use rusqlite::{Connection, params};
use sarv_quote::{
    accounting::{AccountClass, BalanceSide, JournalKind, NewAccount},
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

fn equal_state(actual: &State, expected: &State) {
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
}

fn row(path: &Path) -> (i64, String) {
    Connection::open(path.join("sarv.sqlite3"))
        .unwrap()
        .query_row("SELECT generation,payload FROM state", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap()
}

#[test]
fn procurement_migration_child() {
    if let Some(path) = std::env::var_os("SARV_M4_MIGRATION_CHILD_PROFILE") {
        Store::open(Path::new(&path)).unwrap();
    }
}

#[test]
fn interrupted_schema4_startup_before_migration_retains_exact_row_and_recovers_once() {
    let envelope: Value = serde_json::from_slice(include_bytes!(
        "../docs/assets/accounting-m3/restore/core-v4.json"
    ))
    .unwrap();
    let original: State = serde_json::from_value(envelope["state"].clone()).unwrap();
    let profile = tempfile::tempdir().unwrap();
    seed(profile.path(), &original);
    let held = Connection::open(profile.path().join("sarv.sqlite3")).unwrap();
    held.execute_batch("PRAGMA journal_mode=WAL; BEGIN IMMEDIATE;")
        .unwrap();
    let preceding = row(profile.path());
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "procurement_migration_child", "--nocapture"])
        .env("SARV_M4_MIGRATION_CHILD_PROFILE", profile.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // Precisely the startup INSERT blocked behind an owned writer, before the
    // schema UPDATE; this is not hardware power loss or an internal write hook.
    std::thread::sleep(Duration::from_millis(150));
    assert!(child.try_wait().unwrap().is_none());
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    assert_eq!(row(profile.path()), preceding);
    assert!(!fs::read_dir(profile.path()).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("before-schema5-")
    }));
    held.execute_batch("ROLLBACK").unwrap();
    drop(held);
    let opened = Store::open(profile.path()).unwrap();
    let mut expected = original.clone();
    expected.schema = CURRENT_SCHEMA;
    equal_state(&opened.state, &expected);
    assert_eq!(opened.local_generation(), 42);
    drop(opened);
    assert_eq!(Store::open(profile.path()).unwrap().local_generation(), 42);
    let db = Connection::open(profile.path().join("sarv.sqlite3")).unwrap();
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    if let Ok(root) = std::env::var("SARV_M4_MIGRATION_ARTIFACT_DIR") {
        fs::create_dir_all(&root).unwrap();
        fs::write(Path::new(&root).join("interrupted-startup.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "result":"passed", "schema_before":4,"schema_after":5,
            "scope":"owned-child kill while startup INSERT waits, before schema UPDATE; software interruption only",
            "exact_old_row_after_kill":true,"no_recovery_before_kill":true,"next_open_generation":42,
            "repeat_open_generation":42,"integrity":"ok"
        })).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "requires retained immutable M3 executable in SARV_M3_COMPAT_BINARY"]
fn actual_m3_binary_rejects_schema5_without_modifying_committed_row() {
    let binary = std::env::var("SARV_M3_COMPAT_BINARY").expect("retained M3 executable required");
    let envelope: Value = serde_json::from_slice(include_bytes!(
        "../docs/assets/accounting-m3/restore/core-v4.json"
    ))
    .unwrap();
    let original: State = serde_json::from_value(envelope["state"].clone()).unwrap();
    let profile = tempfile::tempdir().unwrap();
    seed(profile.path(), &original);
    drop(Store::open(profile.path()).unwrap());
    let preceding = row(profile.path());
    let output = Command::new(&binary)
        .env("SARV_DATA_DIR", profile.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(
        diagnostic.contains("schema") || diagnostic.contains("نسخه"),
        "unexpected failure: {diagnostic}"
    );
    assert_eq!(row(profile.path()), preceding);
    if let Ok(root) = std::env::var("SARV_M4_MIGRATION_ARTIFACT_DIR") {
        fs::create_dir_all(&root).unwrap();
        fs::write(
            Path::new(&root).join("actual-m3-rejection.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "result":"passed","old_binary":binary,"old_schema":4,"rejected_schema":5,
                "exit_code":output.status.code(),"stderr":diagnostic,"committed_row_unchanged":true
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

fn seed(path: &Path, state: &State) {
    let db = Connection::open(path.join("sarv.sqlite3")).unwrap();
    db.execute_batch("CREATE TABLE state(id INTEGER PRIMARY KEY CHECK(id=1),generation INTEGER NOT NULL,payload TEXT NOT NULL)").unwrap();
    db.execute(
        "INSERT INTO state VALUES(1,41,?1)",
        params![serde_json::to_string(state).unwrap()],
    )
    .unwrap();
}

// M3 permitted these event names in ordinary manual books. The independent
// archived-M3 probe also creates and validates such original/reversed v4 books.
fn legacy_named_journal(kind: JournalKind, event: &str) -> State {
    let envelope: Value = serde_json::from_slice(include_bytes!(
        "../docs/assets/accounting-m3/migration/current-v4.json"
    ))
    .unwrap();
    let mut state: State = serde_json::from_value(envelope["state"].clone()).unwrap();
    let journal = state
        .accounting
        .journals
        .iter_mut()
        .find(|journal| journal.kind == kind)
        .unwrap();
    journal.event = event.into();
    let id = journal.id.clone();
    for request in &mut state.accounting.requests {
        if request.journal_id == id {
            request.command.event = event.into();
        }
    }
    state.validate().unwrap();
    state
}

#[test]
fn legacy_procurement_named_manual_and_linked_reversal_migrate_with_exact_history() {
    for event in ["procurement.receive", "procurement.receive.v1"] {
        let original = legacy_named_journal(JournalKind::Manual, event);
        let profile = tempfile::tempdir().unwrap();
        seed(profile.path(), &original);
        let store = Store::open(profile.path()).unwrap();
        let mut expected = original.clone();
        expected.schema = CURRENT_SCHEMA;
        equal_state(&store.state, &expected);
        assert_eq!(store.local_generation(), 42);
        assert!(store.state.accounting.procurement.is_empty());
        let recovery = fs::read_dir(profile.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("before-schema5-")
            })
            .unwrap();
        let raw: Value = serde_json::from_slice(&fs::read(&recovery).unwrap()).unwrap();
        assert_eq!(raw["format"], "sarv-accounting-backup-v4");
        assert_eq!(raw["state"], serde_json::to_value(&original).unwrap());
        let backup = profile.path().join("legacy-manual-v5.json");
        store.backup(&backup).unwrap();
        let fresh = tempfile::tempdir().unwrap();
        let mut restored = Store::open(fresh.path()).unwrap();
        restored
            .restore(Store::restore_preview(&backup).unwrap())
            .unwrap();
        equal_state(&restored.state, &expected);
        drop(restored);
        equal_state(&Store::open(fresh.path()).unwrap().state, &expected);
        if let Ok(root) = std::env::var("SARV_M4_MIGRATION_ARTIFACT_DIR") {
            let output = Path::new(&root).join(event.replace('.', "-"));
            fs::create_dir_all(&output).unwrap();
            fs::copy(recovery, output.join("verified-before-schema5-v4.json")).unwrap();
            fs::copy(backup, output.join("current-v5.json")).unwrap();
            fs::write(
                output.join("verification.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "result":"passed","legacy_manual_event":event,
                    "schema_before":4,"schema_after":CURRENT_SCHEMA,
                    "generation_before":41,"generation_after":42,
                    "original_and_linked_reversal_history_exact":true,
                    "fresh_restore_and_restart_equal":true,"typed_procurement_records":0
                }))
                .unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn legacy_namespace_can_reverse_but_new_reserved_manual_append_cannot_commit() {
    let original = legacy_named_journal(JournalKind::Opening, "procurement.receive.v1");
    let profile = tempfile::tempdir().unwrap();
    seed(profile.path(), &original);
    let mut store = Store::open(profile.path()).unwrap();
    let company_id = store.state.accounting.companies[0].id.clone();
    // The retained fixture deliberately archives its original asset account.
    // A new ordinary journal uses an active account; linked reversal below
    // continues to use the original immutable account snapshot.
    let active_asset = store
        .change(|state| {
            state.accounting.add_account(
                NewAccount {
                    company_id,
                    parent_id: None,
                    code: "1102".into(),
                    name: "Synthetic active asset".into(),
                    classification: AccountClass::Asset,
                    normal_balance: BalanceSide::Debit,
                    postable: true,
                    control: false,
                },
                "synthetic-review",
            )
        })
        .unwrap();
    let before = store.state.clone();
    let generation = store.local_generation();
    let legacy = before
        .accounting
        .journals
        .iter()
        .find(|journal| journal.event == "procurement.receive.v1")
        .unwrap();
    assert!(!before.accounting.has_typed_journal_source(&legacy.id));
    let period = before
        .accounting
        .periods
        .iter()
        .find(|period| !period.locked)
        .unwrap();
    let mut command = before.accounting.requests[0].command.clone();
    command.kind = JournalKind::Manual;
    command.period_id = period.id.clone();
    command.date = period.start_date.clone();
    command.source_id = "new-reserved-manual".into();
    command.request_key = "new-reserved-manual-key".into();
    command.lines[0].account_id = active_asset;
    let mut public_attempt = before.clone();
    assert!(public_attempt.accounting.post(command.clone()).is_err());
    equal_state(&public_attempt, &before);

    command.event = "manual.allowed".into();
    let mut candidate = before.clone();
    let id = candidate.accounting.post(command).unwrap();
    candidate
        .accounting
        .journals
        .iter_mut()
        .find(|journal| journal.id == id)
        .unwrap()
        .event = "procurement.receive.v1".into();
    for request in &mut candidate.accounting.requests {
        if request.journal_id == id {
            request.command.event = "procurement.receive.v1".into();
        }
    }
    // Structurally valid legacy namespace replay is not permission to append a
    // newly reserved manual event through either ordinary or prepared writes.
    candidate.validate().unwrap();
    let prepared = Store::prepare_candidate(generation, candidate.clone()).unwrap();
    assert!(store.commit_prepared(prepared).is_err());
    assert!(
        store
            .change(|state| {
                *state = candidate;
                Ok(())
            })
            .is_err()
    );
    equal_state(&store.state, &before);
    assert_eq!(store.local_generation(), generation);
    equal_state(&Store::open(profile.path()).unwrap().state, &before);

    let legacy_id = legacy.id.clone();
    let legacy_period = legacy.period_id.clone();
    let new_period = period.id.clone();
    let date = period.start_date.clone();
    store
        .change(|state| {
            state.accounting.set_period_lock(
                &legacy_period,
                false,
                "synthetic-review",
                "Explicit synthetic reopen before ordinary legacy reversal",
            )
        })
        .unwrap();
    store
        .change(|state| {
            state.accounting.reverse(
                &legacy_id,
                &new_period,
                &date,
                "legacy-safe-reversal",
                "synthetic-review",
                "Linked reversal of an ordinary pre-M4 manual journal",
            )
        })
        .unwrap();
    assert!(
        store
            .state
            .accounting
            .journals
            .starts_with(&before.accounting.journals)
    );
    let exact = store.state.clone();
    drop(store);
    equal_state(&Store::open(profile.path()).unwrap().state, &exact);
}

#[test]
fn retained_schema4_quotations_and_detailed_opening_upgrade_with_exact_v4_recovery() {
    for (name, bytes) in [
        (
            "historical-quotes",
            include_bytes!("../docs/assets/accounting-m3/migration/current-v4.json").as_slice(),
        ),
        (
            "detailed-opening",
            include_bytes!("../docs/assets/accounting-m3/restore/core-v4.json").as_slice(),
        ),
    ] {
        let envelope: Value = serde_json::from_slice(bytes).unwrap();
        let old: State = serde_json::from_value(envelope["state"].clone()).unwrap();
        assert_eq!(old.schema, 4);
        assert!(old.accounting.procurement.is_empty());
        old.validate().unwrap();
        let profile = tempfile::tempdir().unwrap();
        seed(profile.path(), &old);
        let opened = Store::open(profile.path()).unwrap();
        let mut expected = old.clone();
        expected.schema = CURRENT_SCHEMA;
        equal_state(&opened.state, &expected);
        assert_eq!(opened.local_generation(), 42);
        let recovery = fs::read_dir(profile.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("before-schema5-")
            })
            .unwrap();
        let raw: Value = serde_json::from_slice(&fs::read(&recovery).unwrap()).unwrap();
        assert_eq!(raw["format"], "sarv-accounting-backup-v4");
        assert_eq!(raw["state"], envelope["state"]);
        equal_state(&Store::restore_preview(&recovery).unwrap(), &expected);
        let backup = profile.path().join("current-v5.json");
        opened.backup(&backup).unwrap();
        let current: Value = serde_json::from_slice(&fs::read(&backup).unwrap()).unwrap();
        assert_eq!(current["format"], "sarv-accounting-backup-v5");
        let fresh = tempfile::tempdir().unwrap();
        let mut restored = Store::open(fresh.path()).unwrap();
        restored
            .restore(Store::restore_preview(&backup).unwrap())
            .unwrap();
        equal_state(&restored.state, &expected);
        drop(restored);
        equal_state(&Store::open(fresh.path()).unwrap().state, &expected);
        drop(opened);
        let reopened = Store::open(profile.path()).unwrap();
        assert_eq!(reopened.local_generation(), 42);
        equal_state(&reopened.state, &expected);
        assert_eq!(
            fs::read_dir(profile.path())
                .unwrap()
                .filter(|e| e
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("before-schema5-"))
                .count(),
            1
        );
        if let Ok(root) = std::env::var("SARV_M4_MIGRATION_ARTIFACT_DIR") {
            let output = Path::new(&root).join(name);
            fs::create_dir_all(&output).unwrap();
            fs::copy(&recovery, output.join("verified-before-schema5-v4.json")).unwrap();
            fs::copy(&backup, output.join("current-v5.json")).unwrap();
            fs::write(
                output.join("verification.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "result":"passed", "schema_before":4, "schema_after":CURRENT_SCHEMA,
                    "generation_before":41,"generation_after":42,
                    "quotes":old.quotes.len(),"journals":old.accounting.journals.len(),
                    "opening_details":old.accounting.core.openings.len(),
                    "previous_snapshot_exact":true,"procurement_installed_into_old_books":false,
                    "full_state_equal_except_schema":true,"fresh_restore_and_restart_equal":true
                }))
                .unwrap(),
            )
            .unwrap();
        }
    }
}
