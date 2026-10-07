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
    assert_eq!(migrated.state.schema, 2);
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
                .starts_with("before-schema2-")
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
                .starts_with("before-schema2-"))
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
