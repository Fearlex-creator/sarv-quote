//! Independent workflow integrity tests. All records are synthetic.
use sarv_quote::{domain::*, export, imports::*, store::Store};

fn prepared() -> State {
    let source = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    let prices = format!(
        "code,name,measure,price,currency,source_at,aliases\nA-12,کالای آزمایشی,عدد,12.3,toman,{source},كالا|shared\nB-20,کالای دوم,عدد,200,rial,{source},A-12|shared\n"
    );
    let mut state = State::default();
    accept_catalog(&mut state, catalog_csv(&prices, "آزمایشی").unwrap()).unwrap();
    state.draft.customer = "مشتری آزمایشی".into();
    state.draft.lines = order_csv("item,quantity\nA-12,۱٫۲۵\n").unwrap();
    state
}

fn snapshot(state: &State) -> serde_json::Value {
    serde_json::to_value(state).unwrap()
}

#[test]
fn exact_rial_toman_roundtrip_including_sub_toman_amounts() {
    for rial in (0..=10_001).chain([MAX_MONEY - 1, MAX_MONEY]) {
        for unit in [Unit::Rial, Unit::Toman] {
            assert_eq!(unit.nonnegative(&unit.number(rial)).unwrap(), rial);
        }
    }
    for (price, quantity, expected) in [(123, "1.25", 154), (1, "0.5", 1), (1, "0.499", 0)] {
        assert_eq!(
            line_total(price, sarv_quote::domain::quantity(quantity).unwrap()).unwrap(),
            expected
        );
    }
}

#[test]
fn exact_code_wins_over_alias_and_ambiguity_never_approves() {
    let mut state = prepared();
    assert_eq!(state.catalog().unwrap().matches("a-١٢")[0].code, "A-12");
    state.draft.lines[0].query = "shared".into();
    let before = snapshot(&state);
    assert!(state.approve().is_err());
    assert_eq!(snapshot(&state), before);
    state.draft.lines[0].chosen_code = Some("B-20".into());
    state.approve().unwrap();
    assert_eq!(state.quotes[0].lines[0].code, "B-20");
}

#[test]
fn abort_after_approval_does_not_leak_customer_quote_or_draft() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store
        .change(|state| {
            *state = prepared();
            Ok(())
        })
        .unwrap();
    let before = snapshot(&store.state);
    let rejected: anyhow::Result<()> = store.change(|state| {
        state.approve()?;
        anyhow::bail!("synthetic failure after candidate approval")
    });
    assert!(rejected.is_err());
    assert_eq!(snapshot(&store.state), before);
    drop(store);
    assert_eq!(snapshot(&Store::open(dir.path()).unwrap().state), before);
}

#[test]
fn conflict_during_approval_does_not_publish_unsaved_quote() {
    let dir = tempfile::tempdir().unwrap();
    let mut first = Store::open(dir.path()).unwrap();
    first
        .change(|state| {
            *state = prepared();
            Ok(())
        })
        .unwrap();
    let mut second = Store::open(dir.path()).unwrap();
    let stale = snapshot(&second.state);
    first
        .change(|state| {
            state.draft.phone = "09120000000".into();
            Ok(())
        })
        .unwrap();
    assert!(second.change(State::approve).is_err());
    assert_eq!(snapshot(&second.state), stale);
    assert!(Store::open(dir.path()).unwrap().state.quotes.is_empty());
}

#[test]
fn revising_old_version_appends_without_rewriting_any_snapshot() {
    let mut state = prepared();
    let first = state.approve().unwrap();
    state.draft.lines[0].quantity_milli = 2000;
    state.approve().unwrap();
    let history = serde_json::to_value(&state.quotes).unwrap();
    state.revise(&first).unwrap();
    let third = state.approve().unwrap();
    assert_eq!(state.quotes[2].version, 3);
    assert_eq!(serde_json::to_value(&state.quotes[..2]).unwrap(), history);
    assert_eq!(state.approve().unwrap(), third);
    assert_eq!(state.quotes.len(), 3);
    state.validate().unwrap();
}

#[test]
fn historical_fresh_price_approval_remains_restorable_when_price_ages() {
    let mut state = prepared();
    state.approve().unwrap();
    let old_source = (chrono::Utc::now() - chrono::Duration::days(31)).to_rfc3339();
    let old_approval = (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339();
    for product in &mut state.catalogs[0].products {
        product.source_at = old_source.clone();
    }
    state.catalogs[0].fingerprint = product_fingerprint(&state.catalogs[0].products).unwrap();
    state.quotes[0].approved_at = old_approval;
    state.quotes[0].lines[0].source_at = old_source;
    assert!(stale(&state.quotes[0].lines[0].source_at));
    state.validate().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let backup = dir.path().join("old-valid.json");
    sarv_quote::store::backup_state(&state, &backup).unwrap();
    assert_eq!(
        snapshot(&Store::restore_preview(&backup).unwrap()),
        snapshot(&state)
    );
}

#[test]
fn restore_safety_backup_contains_exact_pre_restore_state() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store
        .change(|state| {
            *state = prepared();
            state.approve()?;
            Ok(())
        })
        .unwrap();
    let before = snapshot(&store.state);
    let replacement = State::default();
    let restored = snapshot(&replacement);
    let safety = store.restore(replacement).unwrap();
    assert_eq!(snapshot(&Store::restore_preview(&safety).unwrap()), before);
    drop(store);
    assert_eq!(snapshot(&Store::open(dir.path()).unwrap().state), restored);
}

#[test]
fn restore_invalid_candidate_leaves_memory_disk_and_directory_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store
        .change(|state| {
            *state = prepared();
            state.approve()?;
            Ok(())
        })
        .unwrap();
    let before = snapshot(&store.state);
    let count = std::fs::read_dir(dir.path()).unwrap().count();
    let mut invalid = store.state.clone();
    invalid.quotes[0].lines[0].total_rial += 1;
    assert!(store.restore(invalid).is_err());
    assert_eq!(snapshot(&store.state), before);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), count);
    assert_eq!(snapshot(&Store::open(dir.path()).unwrap().state), before);
}

#[test]
fn exported_user_fields_are_formula_safe_and_target_is_never_overwritten() {
    let mut state = prepared();
    state.draft.customer = "  +SUM(1,2)".into();
    state.draft.lines[0].query = "=1+1".into();
    state.draft.lines[0].chosen_code = Some("A-12".into());
    state.approve().unwrap();
    let bytes = export::csv_bytes(&state.quotes[0]).unwrap();
    let mut reader = csv::Reader::from_reader(&bytes[3..]);
    let headers = reader.headers().unwrap().clone();
    let record = reader.records().next().unwrap().unwrap();
    let cell = |name| &record[headers.iter().position(|header| header == name).unwrap()];
    assert_eq!(cell("customer"), "'  +SUM(1,2)");
    assert_eq!(cell("requested_item"), "'=1+1");
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("existing.csv");
    std::fs::write(&target, b"existing bytes").unwrap();
    assert!(export::csv(&state.quotes[0], &target).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"existing bytes");
}

#[test]
fn correctly_checksummed_bad_revision_chain_is_rejected_on_restore() {
    let mut state = prepared();
    state.approve().unwrap();
    state.quotes[0].version = 2;
    let bytes = serde_json::to_vec(&state).unwrap();
    let backup = serde_json::json!({"format":"sarv-quote-backup-v1", "checksum":hash(&bytes), "state":state});
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("forged.json");
    std::fs::write(&target, serde_json::to_vec(&backup).unwrap()).unwrap();
    assert!(Store::restore_preview(&target).is_err());
}

/// Opt in from a graphical session. Creates no application/window and does not
/// interact with the user's running desktop application.
#[test]
#[ignore = "requires graphical GTK/libadwaita initialization; creates no windows"]
fn native_widget_lifecycle_does_not_synthesize_new_or_empty_edit_actions() {
    use gtk::prelude::*;
    use std::{cell::Cell, rc::Rc};

    adw::init().unwrap();
    let clicked = Rc::new(Cell::new(0));
    let button = gtk::ToggleButton::with_label("آزمون ناوبری");
    let count = clicked.clone();
    button.connect_clicked(move |_| count.set(count.get() + 1));
    for _ in 0..10 {
        button.set_active(true);
        button.set_active(false);
    }
    assert_eq!(
        clicked.get(),
        0,
        "programmatic navigation activation clicked"
    );

    let changed = Rc::new(Cell::new(0));
    let entry = gtk::Entry::new();
    entry.set_text("P-101");
    let count = changed.clone();
    entry.connect_changed(move |_| count.set(count.get() + 1));
    let parent = gtk::Box::new(gtk::Orientation::Vertical, 0);
    parent.append(&entry);
    parent.remove(&entry);
    assert_eq!(entry.text(), "P-101");
    drop(entry);
    drop(parent);
    assert_eq!(
        changed.get(),
        0,
        "removal/disposal synthesized an empty edit"
    );
}
