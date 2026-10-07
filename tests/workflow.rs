use sarv_quote::{domain::*, export::*, imports::*, store::Store};
use std::path::Path;
fn demo_prices() -> String {
    include_str!("../demo/prices.csv")
        .replace(
            "2026-10-01T08:00:00Z",
            &(chrono::Utc::now() - chrono::Duration::days(1))
                .format("%Y-%m-%dT00:00:00Z")
                .to_string(),
        )
        .replace(
            "2026-09-01T08:00:00Z",
            &(chrono::Utc::now() - chrono::Duration::days(30))
                .format("%Y-%m-%dT00:00:00Z")
                .to_string(),
        )
}
fn prepared() -> State {
    let mut s = State::default();
    let c = catalog_csv(&demo_prices(), "synthetic").unwrap();
    accept_catalog(&mut s, c).unwrap();
    s.draft.customer = "مشتری نمایشی".into();
    s.draft.lines = order_csv("item,quantity\nP-101,۲\nقلم آبی,١٢\n").unwrap();
    s
}
#[test]
fn digits_and_persian_normalization() {
    assert_eq!(normalize("  كاغذ\u{200c}ي  ١۲٣ "), "کاغذ ی 123");
    assert_eq!(quantity("١٫٢٥").unwrap(), 1250);
    assert_eq!(fa("125.5"), "۱۲۵.۵");
    assert!(text("a\u{202e}").is_err());
    assert!(text("a\0").is_err());
}
#[test]
fn money_and_rounding() {
    assert_eq!(Unit::Toman.amount("۱۲۵۰۰۰").unwrap(), 1_250_000);
    assert_eq!(Unit::Rial.amount("٨٥٠٠٠").unwrap(), 85_000);
    assert_eq!(line_total(3, 500).unwrap(), 2);
    assert_eq!(line_total(1, 499).unwrap(), 0);
    assert_eq!(Unit::Toman.display(123), "۱۲.۳ تومان");
    assert_eq!(Unit::Toman.amount("۱۲٫۳").unwrap(), 123);
    assert_eq!(Unit::Toman.nonnegative("۰").unwrap(), 0);
    assert!(Unit::Rial.amount("1.1").is_err());
    assert!(Unit::Toman.amount("1.11").is_err());
    assert_eq!(Unit::Toman.amount(&Unit::Toman.number(123)).unwrap(), 123);
    assert!(Unit::Rial.amount("1e5").is_err());
    assert!(Unit::Toman.amount("999999999999999999").is_err());
    assert!(line_total(MAX_MONEY, 1001).is_err());
    for bad in ["0", "-1", "1.0001", "1e5", "1,000", "NaN", "1.", ".5"] {
        assert!(quantity(bad).is_err(), "{bad}");
    }
}
#[test]
fn csv_validation_and_duplicates() {
    let mut s = prepared();
    let duplicate = catalog_csv(&demo_prices(), "renamed").unwrap();
    assert!(accept_catalog(&mut s, duplicate).is_err());
    let bad = "code,name,measure,price,currency,source_at,aliases\nA,one,pcs,1,rial,2026-01-01T00:00:00Z,\na,two,pcs,2,rial,2026-01-01T00:00:00Z,";
    assert!(catalog_csv(bad, "bad").is_err());
    assert!(catalog_csv(&bad.replace("1,rial", "1,USD"), "bad").is_err());
    assert!(catalog_csv(&bad.replace("2026-01-01", "2099-01-01"), "bad").is_err());
    assert!(order_csv("item,quantity\nP-101,-1").is_err());
    assert!(catalog_csv("code,name\nA,B", "bad").is_err());
}
#[test]
fn only_exact_codes_or_approved_aliases() {
    let mut s = prepared();
    assert_eq!(s.catalog().unwrap().matches("p-101").len(), 1);
    assert_eq!(s.catalog().unwrap().matches("كاغذ آ۴").len(), 1);
    assert_eq!(s.catalog().unwrap().matches("خودکار").len(), 2);
    assert!(s.catalog().unwrap().matches("خودکار آبی نمایشی").is_empty());
    s.draft.lines = order_csv(include_str!("../demo/order.csv")).unwrap();
    assert!(s.approve().is_err());
    s.draft.lines[2].chosen_code = Some("P-103".into());
    assert!(s.approve().is_err());
    s.draft.lines[3].chosen_code = Some("P-104".into());
    assert!(s.approve().is_err());
    s.draft.stale_ack = true;
    assert!(s.approve().is_ok());
    s.validate().unwrap();
}
#[test]
fn approval_idempotency_and_revisions() {
    let mut s = prepared();
    s.draft.discount_rial = 10_000;
    let first = s.approve().unwrap();
    assert_eq!(s.quotes[0].total_rial, 3_510_000);
    assert_eq!(s.approve().unwrap(), first);
    assert_eq!(s.quotes.len(), 1);
    s.revise(&first).unwrap();
    assert_eq!(s.approve().unwrap(), first);
    s.draft.lines[0].quantity_milli = 3000;
    let second = s.approve().unwrap();
    assert_ne!(first, second);
    assert_eq!(s.quotes[0].total_rial, 3_510_000);
    assert_eq!(s.quotes[1].version, 2);
    assert_eq!(s.quotes[1].total_rial, 4_760_000);
    assert_eq!(s.approve().unwrap(), second);
    assert_eq!(s.customers.len(), 1);
    s.validate().unwrap();
    s.quotes.reverse();
    assert!(s.validate().is_err());
}
#[test]
fn csv_injection() {
    for s in ["=cmd()", " +1", "\t@sum()", "\u{feff}-2"] {
        assert!(safe_cell(s).starts_with('\''));
    }
    let mut s = prepared();
    s.draft.customer = "=HYPERLINK(\"x\")".into();
    s.approve().unwrap();
    let b = csv_bytes(&s.quotes[0]).unwrap();
    let mut r = csv::Reader::from_reader(&b[3..]);
    let record = r.records().next().unwrap().unwrap();
    assert_eq!(&record[2], "'=HYPERLINK(\"x\")");
}
#[test]
fn persistence_restart_backup_restore_and_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = Store::open(dir.path()).unwrap();
    st.change(|s| {
        *s = prepared();
        s.approve()?;
        Ok(())
    })
    .unwrap();
    let mut concurrent = Store::open(dir.path()).unwrap();
    st.change(|s| {
        s.draft.phone = "123".into();
        Ok(())
    })
    .unwrap();
    assert!(
        concurrent
            .change(|s| {
                s.draft.phone = "other".into();
                Ok(())
            })
            .is_err()
    );
    let backup = dir.path().join("backup.json");
    st.backup(&backup).unwrap();
    assert!(st.backup(&backup).is_err());
    let saved = Store::restore_preview(&backup).unwrap();
    let approved = saved.quotes[0].id.clone();
    st.change(|s| {
        s.draft = Draft::default();
        Ok(())
    })
    .unwrap();
    let safety = st.restore(saved).unwrap();
    assert!(safety.exists());
    drop(st);
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.state.quotes[0].id, approved);
    assert_eq!(reopened.state.draft.phone, "123");
    let mut b: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&backup).unwrap()).unwrap();
    b["state"]["quotes"][0]["total_rial"] = serde_json::json!(1);
    let corrupt = dir.path().join("corrupt.json");
    std::fs::write(&corrupt, serde_json::to_vec(&b).unwrap()).unwrap();
    assert!(Store::restore_preview(&corrupt).is_err());
    assert_eq!(reopened.state.quotes[0].total_rial, 3_520_000);
}
#[test]
fn malicious_backup_valid_hash_rejected() {
    let mut s = prepared();
    s.approve().unwrap();
    s.quotes[0].lines[0].price_rial = 1;
    assert!(s.validate().is_err());
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("bad.json");
    let bytes = serde_json::to_vec(&s).unwrap();
    let b =
        serde_json::json!({"format":"sarv-accounting-backup-v3","checksum":hash(&bytes),"state":s});
    std::fs::write(&p, serde_json::to_vec(&b).unwrap()).unwrap();
    assert!(Store::restore_preview(&p).is_err());
    assert!(Store::restore_preview(Path::new("/no-such-backup")).is_err());
}
#[test]
fn export_real_pdf_and_no_overwrite() {
    let mut s = prepared();
    s.approve().unwrap();
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("quote.pdf");
    pdf(&s.quotes[0], &p).unwrap();
    assert!(std::fs::read(&p).unwrap().starts_with(b"%PDF"));
    assert!(pdf(&s.quotes[0], &p).is_err());
    let c = d.path().join("quote.csv");
    csv(&s.quotes[0], &c).unwrap();
    assert!(csv(&s.quotes[0], &c).is_err());
}
#[test]
fn old_catalog_snapshot_and_new_catalog_revision() {
    let mut s = prepared();
    let first = s.approve().unwrap();
    let mut c = catalog_csv(&demo_prices(), "new synthetic").unwrap();
    c.products[0].price_rial += 10;
    c.fingerprint = product_fingerprint(&c.products).unwrap();
    accept_catalog(&mut s, c).unwrap();
    s.approve().unwrap();
    assert_eq!(s.quotes[0].lines[0].price_rial, 1_250_000);
    assert_eq!(s.quotes[1].lines[0].price_rial, 1_250_010);
    s.revise(&first).unwrap();
    assert_eq!(s.preview().unwrap().lines[0].price_rial, 1_250_000);
}
