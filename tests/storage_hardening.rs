use sarv_quote::{
    domain::{Customer, OrderLine, State, id},
    imports::{MAX_IMPORT, accept_catalog, catalog_csv, hash, order_csv, product_fingerprint},
    store::{Store, read_import, write_new},
};
use std::{
    fs,
    sync::{Arc, Barrier},
};

fn prices(rows: &str) -> String {
    format!("code,name,measure,price,currency,source_at,aliases\n{rows}")
}
fn catalog() -> sarv_quote::domain::Catalog {
    catalog_csv(&prices("A-۱,کاغذ,بسته,۱۲۳,toman,2020-01-01T00:00:00Z,کاغذ آ۴|ورق\nB,قلم,عدد,50,rial,2020-01-01T00:00:00Z,قلم آبی"), "نمایشی").unwrap()
}
fn bytes(state: &State) -> Vec<u8> {
    serde_json::to_vec(state).unwrap()
}

#[test]
fn equivalent_csv_is_duplicate_without_changing_legacy_fingerprint() {
    let mut state = State::default();
    let first = catalog();
    let fingerprint = first.fingerprint.clone();
    accept_catalog(&mut state, first).unwrap();
    let before = bytes(&state);
    let equivalent = catalog_csv(&prices("B,قلم,عدد,۵۰,rial,2020-01-01T03:30:00+03:30,قلم آبی\na-١,كاغذ,بسته,123,toman,2020-01-01T03:30:00+03:30,ورق|كاغذ آ۴|ورق"), "نام دیگر").unwrap();
    assert_ne!(equivalent.fingerprint, fingerprint);
    assert!(accept_catalog(&mut state, equivalent).is_err());
    assert_eq!(bytes(&state), before);
    assert_eq!(state.catalogs[0].fingerprint, fingerprint);
    assert_eq!(
        product_fingerprint(&state.catalogs[0].products).unwrap(),
        fingerprint
    );
}

#[test]
fn changed_price_or_source_is_a_new_catalog() {
    let mut state = State::default();
    accept_catalog(&mut state, catalog()).unwrap();
    for change_source in [false, true] {
        let mut new = catalog();
        if change_source {
            new.products[0].source_at = "2020-01-02T00:00:00Z".into();
        } else {
            new.products[0].price_rial += 1;
        }
        new.fingerprint = product_fingerprint(&new.products).unwrap();
        accept_catalog(&mut state, new).unwrap();
    }
    assert_eq!(state.catalogs.len(), 3);
}

#[test]
fn invalid_catalog_approval_is_atomic_and_success_clears_resolutions() {
    let mut state = State::default();
    accept_catalog(&mut state, catalog()).unwrap();
    state.draft.lines = vec![OrderLine {
        query: "A-1".into(),
        quantity_milli: 1000,
        chosen_code: Some("A-1".into()),
    }];
    state.draft.stale_ack = true;
    let before = bytes(&state);
    let mut invalid = catalog();
    invalid.products[0].price_rial = -1;
    invalid.fingerprint = product_fingerprint(&invalid.products).unwrap();
    assert!(accept_catalog(&mut state, invalid).is_err());
    assert_eq!(bytes(&state), before);
    let mut valid = catalog();
    valid.products[0].price_rial += 1;
    valid.fingerprint = product_fingerprint(&valid.products).unwrap();
    accept_catalog(&mut state, valid).unwrap();
    assert!(state.draft.lines[0].chosen_code.is_none());
    assert!(!state.draft.stale_ack);
}

#[test]
fn csv_limits_utf8_bom_controls_and_row_errors() {
    assert!(catalog_csv(&"x".repeat(MAX_IMPORT + 1), "نام").is_err());
    assert!(order_csv(&"x".repeat(MAX_IMPORT + 1)).is_err());
    assert_eq!(
        order_csv("\u{feff}item,quantity\nA,١٫٥\nA,2")
            .unwrap()
            .len(),
        2
    );
    assert!(format!("{}", order_csv("item,quantity\nA,1,extra").unwrap_err()).contains("ردیف 2"));
    assert!(
        catalog_csv(
            &prices("A,قلم,عدد,1,rial,2020-01-01T00:00:00Z,\"alias\nsecond\""),
            "نام"
        )
        .is_err()
    );
    assert!(
        catalog_csv(
            &prices("A,قلم,عدد,1,rial,2020-01-01T00:00:00Z,a\u{202e}"),
            "نام"
        )
        .is_err()
    );
    assert!(order_csv("item,quantity\nA,1\nB,0").is_err());
}

#[test]
fn failed_changes_leave_memory_and_disk_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let before = bytes(&store.state);
    assert!(
        store
            .change::<()>(|state| {
                state.draft.customer = "変更".into();
                anyhow::bail!("cancelled")
            })
            .is_err()
    );
    assert!(
        store
            .change(|state| {
                state.schema = 999;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(bytes(&store.state), before);
    assert_eq!(bytes(&Store::open(dir.path()).unwrap().state), before);
}

#[test]
fn repeated_noop_does_not_invalidate_another_window_but_stale_noop_fails() {
    let dir = tempfile::tempdir().unwrap();
    let mut first = Store::open(dir.path()).unwrap();
    let mut second = Store::open(dir.path()).unwrap();
    for _ in 0..10 {
        first.change(|_| Ok(())).unwrap();
    }
    second
        .change(|state| {
            state.draft.customer = "مشتری".into();
            Ok(())
        })
        .unwrap();
    assert!(first.change(|_| Ok(())).is_err());
    let path = dir.path().join("stale.json");
    assert!(first.backup(&path).is_err());
    assert!(!path.exists());
    assert!(first.restore(State::default()).is_err());
    assert!(fs::read_dir(dir.path()).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("before-restore-")
    }));
    assert_eq!(
        Store::open(dir.path()).unwrap().state.draft.customer,
        "مشتری"
    );
}

#[test]
fn lightweight_generation_checks_do_not_write_and_reject_stale_windows() {
    let dir = tempfile::tempdir().unwrap();
    let first = Store::open(dir.path()).unwrap();
    let mut second = Store::open(dir.path()).unwrap();
    let database = rusqlite::Connection::open(dir.path().join("sarv.sqlite3")).unwrap();
    let snapshot = || {
        database
            .query_row(
                "SELECT generation,payload FROM state WHERE id=1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .unwrap()
    };
    let before = snapshot();
    for _ in 0..10 {
        first.verify_current().unwrap();
        second.verify_current().unwrap();
    }
    assert_eq!(snapshot(), before);
    second
        .change(|state| {
            state.draft.customer = "مشتری تازه".into();
            Ok(())
        })
        .unwrap();
    let committed = snapshot();
    assert_eq!(committed.0, before.0 + 1);
    assert!(first.verify_current().is_err());
    second.verify_current().unwrap();
    assert_eq!(snapshot(), committed);
    assert!(first.state.draft.customer.is_empty());
    assert_eq!(
        Store::open(dir.path()).unwrap().state.draft.customer,
        "مشتری تازه"
    );
}

#[test]
fn simultaneous_windows_have_one_winner_and_no_lost_update() {
    let dir = tempfile::tempdir().unwrap();
    let initial = Store::open(dir.path()).unwrap();
    drop(initial);
    let barrier = Arc::new(Barrier::new(2));
    let handles = ["اول", "دوم"].map(|name| {
        let directory = dir.path().to_path_buf();
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            let mut store = Store::open(&directory).unwrap();
            barrier.wait();
            let result = store.change(|state| {
                state.draft.customer = name.into();
                Ok(())
            });
            (name, result.is_ok())
        })
    });
    let results = handles.map(|handle| handle.join().unwrap());
    let winner = results
        .iter()
        .filter(|(_, saved)| *saved)
        .collect::<Vec<_>>();
    assert_eq!(winner.len(), 1);
    assert_eq!(
        Store::open(dir.path()).unwrap().state.draft.customer,
        winner[0].0
    );
}

#[test]
fn restore_is_validated_and_retains_the_committed_safety_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store
        .change(|state| {
            state.draft.customer = "پیش از بازیابی".into();
            Ok(())
        })
        .unwrap();
    let before = bytes(&store.state);
    let invalid = State {
        schema: 99,
        ..State::default()
    };
    assert!(store.restore(invalid).is_err());
    assert_eq!(bytes(&store.state), before);
    let mut restored = State::default();
    restored.draft.customer = "پس از بازیابی".into();
    let safety = store.restore(restored).unwrap();
    assert_eq!(bytes(&Store::restore_preview(&safety).unwrap()), before);
    assert_eq!(
        Store::open(dir.path()).unwrap().state.draft.customer,
        "پس از بازیابی"
    );
}

#[test]
fn backup_envelope_and_legacy_optional_fields_are_validated() {
    let dir = tempfile::tempdir().unwrap();
    let mut legacy = State {
        schema: 1,
        ..State::default()
    };
    legacy.draft.quote_date.clear();
    let payload = bytes(&legacy);
    assert!(!String::from_utf8_lossy(&payload).contains("quote_date"));
    let good = serde_json::json!({ "format": "sarv-quote-backup-v1", "checksum": hash(&payload), "state": legacy });
    let path = dir.path().join("legacy.json");
    fs::write(&path, serde_json::to_vec(&good).unwrap()).unwrap();
    let upgraded = Store::restore_preview(&path).unwrap();
    assert_eq!(upgraded.schema, sarv_quote::domain::CURRENT_SCHEMA);
    let mut expected = legacy.clone();
    expected.schema = sarv_quote::domain::CURRENT_SCHEMA;
    assert_eq!(bytes(&upgraded), bytes(&expected));
    for kind in ["unknown", "checksum", "schema", "extra"] {
        let mut bad = good.clone();
        match kind {
            "unknown" => bad["format"] = serde_json::json!("v2"),
            "checksum" => bad["checksum"] = serde_json::json!("bad"),
            "schema" => {
                bad["state"]["schema"] = serde_json::json!(99);
                let state: State = serde_json::from_value(bad["state"].clone()).unwrap();
                bad["checksum"] = serde_json::json!(hash(&bytes(&state)));
            }
            _ => bad["unexpected"] = serde_json::json!(true),
        }
        fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(Store::restore_preview(&path).is_err(), "{kind}");
    }
}

#[test]
fn oversized_and_non_file_inputs_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    assert!(read_import(dir.path()).is_err());
    assert!(Store::restore_preview(dir.path()).is_err());
    let path = dir.path().join("large.csv");
    fs::File::create(&path)
        .unwrap()
        .set_len(MAX_IMPORT as u64 + 1)
        .unwrap();
    assert!(read_import(&path).is_err());
    fs::File::create(&path)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    assert!(Store::restore_preview(&path).is_err());
    fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(read_import(&path).is_err());
}

#[test]
fn state_that_cannot_fit_a_restorable_backup_is_not_committed() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let before = bytes(&store.state);
    let result = store.change(|state| {
        state.customers = (0..65_000)
            .map(|_| Customer {
                id: id(),
                name: "x".repeat(1000),
                phone: String::new(),
                ..Customer::default()
            })
            .collect();
        Ok(())
    });
    assert!(result.is_err());
    assert!(format!("{:#}", result.unwrap_err()).contains("۶۴"));
    assert_eq!(bytes(&store.state), before);
    assert_eq!(bytes(&Store::open(dir.path()).unwrap().state), before);
    let backup = dir.path().join("still-valid.json");
    store.backup(&backup).unwrap();
    assert_eq!(bytes(&Store::restore_preview(&backup).unwrap()), before);
}

#[test]
fn concurrent_export_never_overwrites_and_leaves_no_staging_files() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("export.csv");
    let barrier = Arc::new(Barrier::new(2));
    let handles = [b"one".to_vec(), b"two".to_vec()].map(|data| {
        let target = target.clone();
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            write_new(&target, &data)
        })
    });
    let successes = handles
        .into_iter()
        .map(|handle| usize::from(handle.join().unwrap().is_ok()))
        .sum::<usize>();
    assert_eq!(successes, 1);
    let actual = fs::read(&target).unwrap();
    assert!(actual == b"one" || actual == b"two");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    assert!(write_new(&target, b"three").is_err());
    assert_eq!(fs::read(&target).unwrap(), actual);
    let missing_parent = dir.path().join("missing/out.csv");
    assert!(write_new(&missing_parent, b"partial").is_err());
    assert!(!missing_parent.exists());
}

#[cfg(unix)]
#[test]
fn export_does_not_follow_or_replace_destination_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.csv");
    fs::write(&original, b"original").unwrap();
    let link = dir.path().join("link.csv");
    std::os::unix::fs::symlink(&original, &link).unwrap();
    assert!(write_new(&link, b"changed").is_err());
    assert_eq!(fs::read(&original).unwrap(), b"original");
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn prepared_worker_changes_are_validated_and_commit_only_to_the_original_generation() {
    fn assert_send<T: Send>() {}
    assert_send::<sarv_quote::store::PreparedChange>();
    let directory = tempfile::tempdir().unwrap();
    let mut first = Store::open(directory.path()).unwrap();
    let mut second = Store::open(directory.path()).unwrap();
    let (generation, mut candidate) = first.snapshot().unwrap();
    candidate.draft.customer = "آماده‌شده در پردازش پس‌زمینه".into();
    let prepared = std::thread::spawn(move || Store::prepare_candidate(generation, candidate))
        .join()
        .unwrap()
        .unwrap();
    first.commit_prepared(prepared).unwrap();
    assert_eq!(
        Store::open(directory.path()).unwrap().state.draft.customer,
        first.state.draft.customer
    );
    assert!(second.snapshot().is_err());
    let invalid = State {
        schema: 99,
        ..State::default()
    };
    assert!(Store::prepare_candidate(0, invalid).is_err());
    assert!(Store::prepare_candidate(-1, State::default()).is_err());
    let obsolete = Store::prepare_candidate(0, State::default()).unwrap();
    let before = bytes(&first.state);
    assert!(first.commit_prepared(obsolete).is_err());
    assert_eq!(bytes(&first.state), before);
    // A second window still holding the original generation must fail the SQL
    // condition even when its in-memory generation matches the prepared value.
    let stale = Store::prepare_candidate(0, State::default()).unwrap();
    assert!(second.commit_prepared(stale).is_err());
    assert_eq!(bytes(&Store::open(directory.path()).unwrap().state), before);
}

#[test]
fn prepared_candidate_becomes_stale_after_a_regular_change_without_losing_either_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(directory.path()).unwrap();
    let (generation, mut candidate) = store.snapshot().unwrap();
    candidate.draft.customer = "نام قدیمی آماده‌شده".into();
    let pending = Store::prepare_candidate(generation, candidate).unwrap();
    store
        .change(|s| {
            s.draft.customer = "نام تازه ثبت‌شده".into();
            Ok(())
        })
        .unwrap();
    let before = bytes(&store.state);
    assert!(store.commit_prepared(pending).is_err());
    assert_eq!(bytes(&store.state), before);
    assert_eq!(bytes(&Store::open(directory.path()).unwrap().state), before);
}
