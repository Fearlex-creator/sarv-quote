use rusqlite::{Connection, params};
use sarv_quote::{
    domain::State,
    imports::{accept_catalog, catalog_csv, order_csv},
    store::Store,
};

fn legacy() -> State {
    let mut state = State {
        schema: 1,
        ..State::default()
    };
    let catalog = catalog_csv(include_str!("../demo/prices.csv"), "منبع مصنوعی").unwrap();
    accept_catalog(&mut state, catalog).unwrap();
    state.draft.customer = "مشتری مصنوعی مهاجرت".into();
    state.draft.lines = order_csv("item,quantity\nP-101,2").unwrap();
    state.draft.stale_ack = true;
    state.approve().unwrap();
    state
}

#[test]
fn schema_one_upgrade_preserves_quotation_bytes_and_verifies_recovery_before_upgrade() {
    let dir = tempfile::tempdir().unwrap();
    let original = legacy();
    let old_quote = serde_json::to_vec(&original.quotes).unwrap();
    let old_payload = serde_json::to_string(&original).unwrap();
    let conn = Connection::open(dir.path().join("sarv.sqlite3")).unwrap();
    conn.execute_batch("CREATE TABLE state(id INTEGER PRIMARY KEY CHECK(id=1), generation INTEGER NOT NULL, payload TEXT NOT NULL);").unwrap();
    conn.execute("INSERT INTO state VALUES(1,42,?1)", [&old_payload])
        .unwrap();
    drop(conn);
    let migrated = Store::open(dir.path()).unwrap();
    assert_eq!(migrated.state.schema, sarv_quote::domain::CURRENT_SCHEMA);
    assert_eq!(migrated.local_generation(), 43);
    assert_eq!(
        serde_json::to_vec(&migrated.state.quotes).unwrap(),
        old_quote
    );
    assert!(migrated.state.accounting.is_empty());
    let recovery = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("before-schema4-")
        })
        .unwrap();
    let envelope: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&recovery).unwrap()).unwrap();
    assert_eq!(envelope["format"], "sarv-quote-backup-v1");
    assert_eq!(envelope["state"]["schema"], 1);
    assert_eq!(
        serde_json::to_vec(&Store::restore_preview(&recovery).unwrap().quotes).unwrap(),
        old_quote
    );
    let new_backup = dir.path().join("upgraded.json");
    migrated.backup(&new_backup).unwrap();
    let fresh = tempfile::tempdir().unwrap();
    let mut restored = Store::open(fresh.path()).unwrap();
    restored
        .restore(Store::restore_preview(&new_backup).unwrap())
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.state).unwrap(),
        serde_json::to_vec(&migrated.state).unwrap()
    );
    drop(migrated);
    assert_eq!(Store::open(dir.path()).unwrap().local_generation(), 43);
    assert_eq!(
        std::fs::read_dir(dir.path())
            .unwrap()
            .filter(|e| e
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("before-schema4-"))
            .count(),
        1
    );
    let db = Connection::open(dir.path().join("sarv.sqlite3")).unwrap();
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn unsupported_or_corrupt_upgrade_never_rewrites_legacy_row() {
    for payload in [
        "{bad-json".to_owned(),
        serde_json::to_string(&State {
            schema: 99,
            ..State::default()
        })
        .unwrap(),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let conn = Connection::open(dir.path().join("sarv.sqlite3")).unwrap();
        conn.execute_batch("CREATE TABLE state(id INTEGER PRIMARY KEY, generation INTEGER NOT NULL, payload TEXT NOT NULL);").unwrap();
        conn.execute("INSERT INTO state VALUES(1,9,?1)", params![payload])
            .unwrap();
        assert!(Store::open(dir.path()).is_err());
        let actual: (i64, String) = conn
            .query_row("SELECT generation,payload FROM state", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(actual, (9, payload));
    }
}

#[test]
fn schema_two_upgrade_verifies_m1_recovery_and_preserves_full_ledger_and_quote_history() {
    let dir = tempfile::tempdir().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/assets/accounting-m1/migration/upgraded-v2.json"
    ))
    .unwrap();
    let mut old: State = serde_json::from_value(envelope["state"].clone()).unwrap();
    let ledger: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/assets/accounting-m1/review/ledger-fixture.json"
    ))
    .unwrap();
    old.accounting = serde_json::from_value(ledger["state"]["accounting"].clone()).unwrap();
    old.validate().unwrap();
    assert_eq!(old.schema, 2);
    assert_eq!(old.quotes.len(), 3);
    assert_eq!(old.accounting.journals.len(), 3);
    let quote_bytes = serde_json::to_vec(&old.quotes).unwrap();
    let ledger_bytes = serde_json::to_vec(&old.accounting).unwrap();
    let conn = Connection::open(dir.path().join("sarv.sqlite3")).unwrap();
    conn.execute_batch("CREATE TABLE state(id INTEGER PRIMARY KEY CHECK(id=1),generation INTEGER NOT NULL,payload TEXT NOT NULL);").unwrap();
    conn.execute(
        "INSERT INTO state VALUES(1,17,?1)",
        [serde_json::to_string(&old).unwrap()],
    )
    .unwrap();
    drop(conn);
    let current = Store::open(dir.path()).unwrap();
    assert_eq!(current.state.schema, sarv_quote::domain::CURRENT_SCHEMA);
    assert_eq!(current.local_generation(), 18);
    assert_eq!(
        serde_json::to_vec(&current.state.quotes).unwrap(),
        quote_bytes
    );
    assert_eq!(
        serde_json::to_vec(&current.state.accounting).unwrap(),
        ledger_bytes
    );
    let recovery = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("before-schema4-")
        })
        .unwrap();
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&recovery).unwrap()).unwrap();
    assert_eq!(saved["format"], "sarv-accounting-backup-v2");
    assert_eq!(saved["state"]["schema"], 2);
    assert_eq!(
        serde_json::to_vec(&serde_json::from_value::<State>(saved["state"].clone()).unwrap())
            .unwrap(),
        serde_json::to_vec(&old).unwrap()
    );
    let backup = dir.path().join("current-v4.json");
    current.backup(&backup).unwrap();
    let new_envelope: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&backup).unwrap()).unwrap();
    assert_eq!(new_envelope["format"], "sarv-accounting-backup-v4");
    let fresh = tempfile::tempdir().unwrap();
    let mut restored = Store::open(fresh.path()).unwrap();
    restored
        .restore(Store::restore_preview(&backup).unwrap())
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.state).unwrap(),
        serde_json::to_vec(&current.state).unwrap()
    );
    if let Ok(output) = std::env::var("SARV_M3_LEGACY_MIGRATION_ARTIFACT_DIR") {
        let p = std::path::Path::new(&output);
        std::fs::create_dir_all(p).unwrap();
        std::fs::copy(recovery, p.join("verified-before-schema4-v2.json")).unwrap();
        std::fs::copy(backup, p.join("current-v4.json")).unwrap();
        std::fs::write(p.join("verification.json"),serde_json::to_vec_pretty(&serde_json::json!({"result":"passed","schema_before":2,"schema_after":4,"generation_before":17,"generation_after":18,"quotes":3,"journals":3,"exact_quotes_sha256":sarv_quote::imports::hash(&quote_bytes),"exact_ledger_sha256":sarv_quote::imports::hash(&ledger_bytes),"fresh_restore_full_state_equal":true,"interrupted_schema_upgrade":"untested"})).unwrap()).unwrap();
    }
    drop(current);
    assert_eq!(Store::open(dir.path()).unwrap().local_generation(), 18);
}
