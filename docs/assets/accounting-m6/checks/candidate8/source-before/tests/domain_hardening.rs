use chrono::{Duration, Utc};
use sarv_quote::{domain::*, imports::*};

fn prepared(age: i64) -> State {
    let source = (Utc::now() - Duration::days(age)).to_rfc3339();
    let input = format!(
        "code,name,measure,price,currency,source_at,aliases\nSKU-۱,کالای آزمایشی,عدد,۱۲٫۳,toman,{source},كالا|آزمایش\n"
    );
    let mut state = State::default();
    accept_catalog(&mut state, catalog_csv(&input, "قیمت آزمایشی").unwrap()).unwrap();
    state.draft.customer = "مشتری آزمایشی".into();
    state.draft.lines = order_csv("item,quantity\nsku-١,۱٫۲۵\n").unwrap();
    state
}

#[test]
fn metadata_snapshots_and_metadata_only_revisions_are_idempotent() {
    let mut state = prepared(1);
    state.draft.reference = "REF-۱۴۰۵-۱".into();
    state.draft.quote_date = "۲۰۲۶-۰۲-۲۸".into();
    state.draft.valid_until = "٢٠٢٦-٠٣-٠٢".into();
    state.draft.notes = "یادداشت اول\nتحویل پس از توافق".into();
    let first = state.approve().unwrap();
    let quote = &state.quotes[0];
    assert_eq!(quote.quote_date, "2026-02-28");
    assert_eq!(quote.valid_until, "2026-03-02");
    assert_eq!(quote.reference, "REF-۱۴۰۵-۱");
    assert_eq!(quote.total_rial, 154);
    assert_eq!(state.approve().unwrap(), first);
    state.revise(&first).unwrap();
    assert_eq!(state.draft.notes, "یادداشت اول\nتحویل پس از توافق");
    assert_eq!(state.draft.reference, "REF-۱۴۰۵-۱");
    assert_eq!(state.draft.valid_until, "2026-03-02");
    assert_eq!(state.approve().unwrap(), first);
    state.draft.notes = "شرایط اصلاح‌شده".into();
    let second = state.approve().unwrap();
    assert_ne!(first, second);
    assert_eq!(state.quotes.len(), 2);
    assert_eq!(state.quotes[1].version, 2);
    assert_eq!(state.quotes[0].notes, "یادداشت اول\nتحویل پس از توافق");
    assert_eq!(state.approve().unwrap(), second);
    state.validate().unwrap();
}

#[test]
fn dates_require_real_iso_dates_and_ordered_expiry() {
    let state = prepared(1);
    for bad in [
        "2026-02-29",
        "2026-2-01",
        "2026-13-01",
        "2026-01-01x",
        "2026-01-01\n",
        "0000-01-01",
        "-0001-01-01",
    ] {
        let mut copy = state.clone();
        copy.draft.quote_date = bad.into();
        assert!(copy.preview().is_err(), "{bad}");
        assert!(copy.validate().is_err(), "{bad}");
    }
    let mut copy = state;
    copy.draft.quote_date = "2024-02-29".into();
    copy.draft.valid_until = "2024-02-28".into();
    assert!(copy.preview().is_err());
    copy.draft.valid_until = "2024-02-29".into();
    copy.preview().unwrap();
    copy.draft.quote_date.clear();
    assert!(copy.preview().is_err());
    copy.draft.valid_until.clear();
    copy.preview().unwrap(); // Legacy documents have no metadata dates.
}

#[test]
fn optional_metadata_preserves_legacy_serialization_bytes() {
    let mut state = prepared(1);
    state.draft.quote_date.clear();
    state.approve().unwrap();
    let original = serde_json::to_vec(&state).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    for field in ["reference", "quote_date", "valid_until", "notes"] {
        assert!(value["draft"].get(field).is_none());
        assert!(value["quotes"][0].get(field).is_none());
    }
    let restored: State = serde_json::from_slice(&original).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), original);
    assert!(restored.draft.quote_date.is_empty());
    restored.validate().unwrap();
    assert_eq!(
        Draft::default().quote_date,
        Utc::now().format("%Y-%m-%d").to_string()
    );
}

#[test]
fn stale_ack_is_required_at_approval_but_archives_do_not_age_invalid() {
    let mut state = prepared(30);
    assert!(state.approve().is_err());
    state.draft.stale_ack = true;
    state.approve().unwrap();
    state.validate().unwrap();
    state.quotes[0].stale_ack = false;
    assert!(state.validate().is_err());
    let source = state.quotes[0].lines[0].source_at.clone();
    state.quotes[0].approved_at =
        (chrono::DateTime::parse_from_rfc3339(&source).unwrap() + Duration::days(1)).to_rfc3339();
    state.validate().unwrap();
    assert!(stale(&source));
    assert!(!stale_at(&source, &state.quotes[0].approved_at));
    state.quotes[0].approved_at = (chrono::DateTime::parse_from_rfc3339(&source).unwrap()
        - Duration::seconds(1))
    .to_rfc3339();
    assert!(state.validate().is_err());
}

#[test]
fn malformed_inputs_cannot_bypass_review_using_a_chosen_code() {
    let mut state = prepared(1);
    state.draft.lines[0].chosen_code = Some("SKU-1".into());
    for bad in ["", "\u{202e}spoof", "bad\0"] {
        state.draft.lines[0].query = bad.into();
        assert!(state.approve().is_err());
        assert!(state.quotes.is_empty());
    }
    state.draft.lines[0].query = "explicit human choice".into();
    state.draft.phone = "123\u{2066}".into();
    assert!(state.approve().is_err());
    state.draft.phone.clear();
    state.draft.reference = "ر".repeat(161);
    assert!(state.approve().is_err());
    state.draft.reference = "ر".repeat(160);
    state.draft.notes = "ن".repeat(4001);
    assert!(state.approve().is_err());
    state.draft.notes = "ن".repeat(4000);
    state.approve().unwrap();
    state.validate().unwrap();
}

#[test]
fn exact_money_boundaries_and_sum_overflow_never_mutate_approval() {
    for amount in [1, 9, 10, 11, 123, MAX_MONEY - 1, MAX_MONEY] {
        assert_eq!(
            Unit::Toman.amount(&Unit::Toman.number(amount)).unwrap(),
            amount
        );
        assert_eq!(
            Unit::Rial.amount(&Unit::Rial.number(amount)).unwrap(),
            amount
        );
    }
    assert!(Unit::Rial.amount(&(MAX_MONEY + 1).to_string()).is_err());
    assert!(Unit::Toman.amount("900000000000000.1").is_err());
    assert_eq!(line_total(1, 500).unwrap(), 1);
    assert_eq!(line_total(MAX_MONEY, 1000).unwrap(), MAX_MONEY);
    let mut state = prepared(1);
    state.catalogs[0].products[0].price_rial = MAX_MONEY;
    state.catalogs[0].fingerprint = product_fingerprint(&state.catalogs[0].products).unwrap();
    state.draft.lines[0].quantity_milli = 1000;
    state.draft.lines.push(state.draft.lines[0].clone());
    assert!(state.approve().is_err());
    assert!(state.quotes.is_empty());
    assert!(state.customers.is_empty());
}

#[test]
fn revision_chains_reject_missing_versions_and_reversed_approval_time() {
    let mut state = prepared(2);
    state.approve().unwrap();
    state.draft.reference = "revision".into();
    state.approve().unwrap();
    state.validate().unwrap();
    let valid = state.clone();
    state.quotes[1].version = 3;
    assert!(state.validate().is_err());
    state = valid;
    state.quotes[1].approved_at =
        (chrono::DateTime::parse_from_rfc3339(&state.quotes[0].approved_at).unwrap()
            - Duration::seconds(1))
        .to_rfc3339();
    assert!(state.validate().is_err());
}

#[test]
fn preview_and_archive_line_count_limits_are_consistent() {
    let mut state = prepared(1);
    let first = state.draft.lines[0].clone();
    state.draft.lines = vec![first; MAX_QUOTE_LINES + 1];
    assert!(state.preview().is_err());
    assert!(state.validate().is_err());
    state.draft.lines.truncate(1);
    state.approve().unwrap();
    state.quotes[0].lines = vec![state.quotes[0].lines[0].clone(); MAX_QUOTE_LINES + 1];
    assert!(state.validate().is_err());
}
