use rusqlite::Connection;
use sarv_quote::{
    desktop::{dispatch, seed_fixture},
    store::Store,
};
use serde_json::{Value, json};

fn call(dir: &std::path::Path, action: &str, p: Value) -> Value {
    dispatch(dir, action, p).unwrap()
}

fn fixture() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    seed_fixture(d.path()).unwrap();
    let _ = Store::open(d.path()).unwrap();
    d
}

#[test]
fn recover_restores_books_when_the_row_is_unreadable() {
    let d = fixture();
    let created = call(d.path(), "backup_create", json!({}));
    let filename = created["filename"].as_str().unwrap().to_string();

    let conn = Connection::open(d.path().join("sarv.sqlite3")).unwrap();
    conn.execute("UPDATE state SET payload='{' WHERE id=1", []).unwrap();
    drop(conn);

    assert!(dispatch(d.path(), "workspace", json!({})).is_err());

    let listed = call(d.path(), "backup_list", json!({}));
    let backups = listed["backups"].as_array().unwrap();
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0]["filename"], filename);
    assert_eq!(backups[0]["kind"], "manual");
    assert_eq!(backups[0]["valid"], true);

    let preview = call(d.path(), "backup_restore_preview", json!({"filename": filename}));
    assert!(preview["current"].is_null());
    assert!(preview["backup"]["companies"].is_array());

    let recovered = call(d.path(), "recover_from_backup", json!({"filename": filename, "confirmed": true}));
    assert_eq!(recovered["recovered"], true);

    let ws = call(d.path(), "workspace", json!({}));
    assert!(!ws.is_null());

    let unreadable: Vec<_> = std::fs::read_dir(d.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("unreadable-") && name.ends_with(".json"))
        .collect();
    assert_eq!(unreadable.len(), 1);
    assert_eq!(std::fs::read_to_string(d.path().join(&unreadable[0])).unwrap(), "{");
}

#[test]
fn recover_is_refused_when_books_load() {
    let d = fixture();
    let created = call(d.path(), "backup_create", json!({}));
    let filename = created["filename"].as_str().unwrap().to_string();

    let err = dispatch(d.path(), "recover_from_backup", json!({"filename": filename, "confirmed": true})).unwrap_err();
    assert!(err.to_string().contains("Books load normally"));
}

#[test]
fn safety_backup_can_be_restored() {
    let d = fixture();
    let created = call(d.path(), "backup_create", json!({}));
    let filename = created["filename"].as_str().unwrap().to_string();

    let restored = call(d.path(), "backup_restore", json!({"filename": filename, "confirmed": true}));
    assert_eq!(restored["restored"], true);
    let safety_name = restored["safety_backup"].as_str().unwrap().to_string();
    assert!(safety_name.starts_with("before-restore-"));

    let listed = call(d.path(), "backup_list", json!({}));
    let backups = listed["backups"].as_array().unwrap();
    let safety_row = backups
        .iter()
        .find(|row| row["filename"] == safety_name)
        .expect("safety backup should be listed");
    assert_eq!(safety_row["kind"], "before-restore");
    assert_eq!(safety_row["valid"], true);

    let restored_safety = call(d.path(), "backup_restore", json!({"filename": safety_name, "confirmed": true}));
    assert_eq!(restored_safety["restored"], true);
}

#[test]
fn restore_rejects_other_filenames() {
    let d = fixture();
    for bad in ["../x.json", "sarv.sqlite3", "evil.json"] {
        assert!(
            dispatch(d.path(), "backup_restore_preview", json!({"filename": bad})).is_err(),
            "preview should reject {bad}"
        );
        assert!(
            dispatch(d.path(), "backup_restore", json!({"filename": bad, "confirmed": true}),).is_err(),
            "restore should reject {bad}"
        );
    }
}
