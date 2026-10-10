//! Independent combined product acceptance checks. Fixtures are synthetic;
//! assertions follow observable persistence/export rather than UI internals.
use sarv_quote::{domain::*, export, imports, store::Store};

fn catalog(price: &str, name: &str) -> Catalog {
    imports::catalog_csv(
        &format!("code,name,measure,price,currency,source_at,aliases\nQA-1,کالای ساختگی مستقل,عدد,{price},toman,2020-01-01T00:00:00Z,نام مستعار ساختگی"),
        name,
    ).unwrap()
}

#[test]
fn complete_customer_catalog_revision_export_and_restore_preserve_exact_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let first = store
        .change(|s| {
            imports::accept_catalog(s, catalog("125.1", "منبع اول"))?;
            let customer = s.save_customer_details(
                "شرکت ساختگی",
                "09120000001",
                "مخاطب اول",
                "qa@example.test",
                "تهران\nنشانی اول",
            )?;
            s.select_customer(&customer)?;
            s.draft.lines = imports::order_csv("item,quantity\nنام مستعار ساختگی,2.375")?;
            s.draft.stale_ack = true;
            s.approve()
        })
        .unwrap();
    let historical = serde_json::to_value(&store.state.quotes[0]).unwrap();
    assert_eq!(store.state.quotes[0].lines[0].price_rial, 1251);
    assert_eq!(store.state.quotes[0].total_rial, 2971);
    let cid = store.state.quotes[0].customer_id.clone();
    let second = store
        .change(|s| {
            s.edit_customer_details(
                &cid,
                "شرکت تازه",
                "09120000002",
                "مخاطب تازه",
                "new@example.test",
                "نشانی تازه",
            )?;
            s.set_customer_archived(&cid, true)?;
            s.revise(&first)?;
            imports::accept_catalog(s, catalog("250.3", "نسخهٔ تازه"))?;
            s.draft.lines[0].quantity_milli = 3125;
            s.draft.stale_ack = true;
            s.approve()
        })
        .unwrap();
    assert_ne!(first, second);
    assert_eq!(store.state.quotes[1].version, 2);
    assert_eq!(store.state.quotes[1].total_rial, 7822);
    assert_eq!(store.state.quotes[1].customer_contact, "مخاطب اول");
    assert_eq!(store.state.quotes[1].customer_address, "تهران\nنشانی اول");
    assert_eq!(
        serde_json::to_value(&store.state.quotes[0]).unwrap(),
        historical
    );
    assert_eq!(store.state.catalogs[0].products[0].price_rial, 1251);
    let exported = export::csv_bytes(&store.state.quotes[1]).unwrap();
    let mut reader = csv::Reader::from_reader(exported.as_slice());
    let headers = reader.headers().unwrap().clone();
    let row = reader.records().next().unwrap().unwrap();
    for (key, expected) in [
        ("customer_contact", "مخاطب اول"),
        ("customer_address", "تهران\nنشانی اول"),
        ("unit_price_rial", "2503"),
        ("quote_total_rial", "7822"),
    ] {
        assert_eq!(
            &row[headers.iter().position(|h| h == key).unwrap()],
            expected
        );
    }
    let backup = dir.path().join("independent.json");
    store.backup(&backup).unwrap();
    let expected = serde_json::to_value(&store.state).unwrap();
    drop(store);
    let mut reopened = Store::open(dir.path()).unwrap();
    assert_eq!(serde_json::to_value(&reopened.state).unwrap(), expected);
    reopened
        .change(|s| {
            s.new_draft()?;
            s.set_customer_archived(&cid, false)
        })
        .unwrap();
    let recovery = reopened
        .restore(Store::restore_preview(&backup).unwrap())
        .unwrap();
    assert!(recovery.exists());
    assert_eq!(serde_json::to_value(&reopened.state).unwrap(), expected);
}

#[test]
fn conflicting_duplicate_customer_and_diagnostic_import_never_partially_commit() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store
        .change(|s| {
            s.save_customer_details("شرکت", "۰۹۱۲", "مخاطب", "a@example.test", "نشانی")?;
            Ok(())
        })
        .unwrap();
    let before = serde_json::to_value(&store.state).unwrap();
    assert!(
        store
            .change(|s| s.save_customer_details(
                "شرکت",
                "0912",
                "مخاطب دیگر",
                "b@example.test",
                "دیگر"
            ))
            .is_err()
    );
    assert_eq!(serde_json::to_value(&store.state).unwrap(), before);
    let bad = "code,name,measure,price,currency,source_at,aliases\nGOOD,صحیح,عدد,125.1,toman,2020-01-01T00:00:00Z,\nBAD,نامعتبر,عدد,0,toman,2020-01-01T00:00:00Z,\nGOOD,تکرار,عدد,2,rial,2020-01-01T00:00:00Z,";
    let report = imports::catalog_csv_report(bad, "همه یا هیچ");
    assert!(report.value.is_none());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.row == 3 && d.column == "price")
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.row == 4 && d.column == "code")
    );
    assert!(
        store
            .change(|s| imports::accept_catalog(s, imports::catalog_csv(bad, "همه یا هیچ")?))
            .is_err()
    );
    assert_eq!(serde_json::to_value(&store.state).unwrap(), before);
    drop(store);
    assert_eq!(
        serde_json::to_value(Store::open(dir.path()).unwrap().state).unwrap(),
        before
    );
}
