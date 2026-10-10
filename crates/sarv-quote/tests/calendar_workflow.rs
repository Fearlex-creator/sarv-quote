#![cfg(feature = "pdf-export")]
use sarv_quote::{
    accounting::{Money, MoneyUnit, TrialBalance, TrialBalanceRow},
    accounting_export::{trial_csv_with_dates, trial_pdf_with_dates},
    domain::{DateDisplay, State},
    export,
    imports::{accept_catalog, catalog_csv, order_csv},
    store::Store,
};
#[test]
fn jalali_preference_survives_recovery_and_preserves_approved_document_dates_and_output() {
    let profile = tempfile::tempdir().unwrap();
    let mut store = Store::open(profile.path()).unwrap();
    store
        .change(|s| {
            accept_catalog(
                s,
                catalog_csv(include_str!("../../../demo/prices.csv"), "مصنوعی").unwrap(),
            )?;
            s.draft.customer = "مشتری مصنوعی".into();
            s.draft.stale_ack = true;
            s.draft.lines = order_csv("item,quantity\nP-101,1")?;
            s.draft.quote_date = "2025-03-20".into();
            s.draft.valid_until = "2025-03-21".into();
            s.approve()?;
            Ok(())
        })
        .unwrap();
    let original = serde_json::to_vec(&store.state.quotes).unwrap();
    let before = profile.path().join("before.csv");
    export::csv(&store.state.quotes[0], &before).unwrap();
    store
        .change(|s| {
            s.settings.date_display = DateDisplay::Jalali;
            Ok(())
        })
        .unwrap();
    assert_eq!(serde_json::to_vec(&store.state.quotes).unwrap(), original);
    let after = profile.path().join("after.csv");
    export::csv(&store.state.quotes[0], &after).unwrap();
    assert_eq!(
        std::fs::read(before).unwrap(),
        std::fs::read(after).unwrap()
    );
    store
        .change(|s| {
            s.new_draft()?;
            s.draft.customer = "مشتری دوم مصنوعی".into();
            s.draft.stale_ack = true;
            s.draft.lines = order_csv("item,quantity\nP-101,1")?;
            s.draft.quote_date = "2025-03-20".into();
            s.draft.valid_until = "2025-03-21".into();
            s.approve()?;
            Ok(())
        })
        .unwrap();
    let q = &store.state.quotes[1];
    assert_eq!(q.date_display, DateDisplay::Jalali);
    assert_eq!(q.quote_date, "2025-03-20");
    assert_eq!(q.valid_until, "2025-03-21");
    assert_eq!(q.date_display.format(&q.quote_date), "1403/12/30");
    assert_eq!(q.date_display.format(&q.valid_until), "1404/01/01");
    let backup = profile.path().join("backup.json");
    store.backup(&backup).unwrap();
    let expected = serde_json::to_vec(&store.state).unwrap();
    drop(store);
    assert_eq!(
        serde_json::to_vec(&Store::open(profile.path()).unwrap().state).unwrap(),
        expected
    );
    let fresh = tempfile::tempdir().unwrap();
    let mut restored = Store::open(fresh.path()).unwrap();
    restored
        .restore(Store::restore_preview(&backup).unwrap())
        .unwrap();
    assert_eq!(serde_json::to_vec(&restored.state).unwrap(), expected);
}
#[test]
fn jalali_trial_exports_name_both_dates_and_preserve_exact_money() {
    let trial = TrialBalance {
        company_id: "SYNTHETIC".into(),
        as_of: "2025-03-20".into(),
        rows: vec![
            TrialBalanceRow {
                account_id: "A".into(),
                code: "1001".into(),
                name: "دارایی".into(),
                debit: Money::from_rials(1251),
                credit: Money::ZERO,
            },
            TrialBalanceRow {
                account_id: "E".into(),
                code: "3001".into(),
                name: "سرمایه".into(),
                debit: Money::ZERO,
                credit: Money::from_rials(1251),
            },
        ],
        total_debit: Money::from_rials(1251),
        total_credit: Money::from_rials(1251),
    };
    let dir = tempfile::tempdir().unwrap();
    let csv = dir.path().join("trial.csv");
    let pdf = dir.path().join("trial.pdf");
    trial_csv_with_dates(
        "شرکت مصنوعی",
        &trial,
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        &csv,
    )
    .unwrap();
    let bytes = std::fs::read(&csv).unwrap();
    let mut reader = csv::Reader::from_reader(&bytes[3..]);
    let headers = reader.headers().unwrap().clone();
    let first = reader.records().next().unwrap().unwrap();
    for (name, value) in [
        ("as_of_gregorian", "2025-03-20"),
        ("display_calendar", "jalali"),
        ("display_as_of", "1403/12/30"),
        ("debit_irr", "1251"),
        ("display_debit", "125.1"),
    ] {
        assert_eq!(
            &first[headers.iter().position(|h| h == name).unwrap()],
            value
        );
    }
    trial_pdf_with_dates(
        "شرکت مصنوعی",
        &trial,
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        &pdf,
    )
    .unwrap();
    let output = std::process::Command::new("pdftotext")
        .arg(&pdf)
        .arg("-")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for value in ["1403/12/30", "2025-03-20", "125.1"] {
        assert!(text.contains(value), "missing {value}");
    }
    let mut outside = trial.clone();
    outside.as_of = "1800-01-01".into();
    let bad = dir.path().join("unsupported.csv");
    assert!(
        trial_csv_with_dates(
            "شرکت مصنوعی",
            &outside,
            MoneyUnit::Rial,
            DateDisplay::Jalali,
            &bad
        )
        .is_err()
    );
    assert!(!bad.exists());
    let untouched = State::default();
    assert_eq!(untouched.schema, sarv_quote::domain::CURRENT_SCHEMA);
}
