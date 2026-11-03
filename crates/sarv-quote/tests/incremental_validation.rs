//! ADR-0007: the fast save path (`State::validate_changes`) must reach the same verdict as
//! the full validator, and the safety nets must catch what the fast path cannot see.
use sarv_quote::{
    desktop::{dispatch_in, seed_fixture},
    domain::{State, id},
    imports,
    store::{IntegrityStatus, Store},
};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

fn catalog(name: &str, products: usize) -> sarv_quote::domain::Catalog {
    let source = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    let mut csv = String::from("code,name,measure,price,currency,source_at,aliases\n");
    for i in 0..products {
        csv.push_str(&format!("P-{i:04},Item {i},pcs,{},rial,{source},alias {i}\n", 100_000 + i));
    }
    imports::catalog_csv(&csv, name).unwrap()
}
fn customer(name: &str) -> sarv_quote::domain::Customer {
    serde_json::from_value(json!({"id": id(), "name": name, "phone": "", "contact_name": "", "email": "", "address": ""})).unwrap()
}
/// A populated, fully valid book: seeded accounting plus a catalog and a customer.
fn base() -> State {
    let dir = tempfile::tempdir().unwrap();
    seed_fixture(dir.path()).unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store
        .change(|s| {
            let c = imports::accept_catalog(s, catalog("base", 50))?;
            s.draft.catalog_id = c;
            s.customers.push(customer("Base customer"));
            Ok(())
        })
        .unwrap();
    store.state.validate().expect("fixture must be fully valid");
    store.state.clone()
}

type Mutation = (&'static str, bool, fn(&mut State));
/// (name, expected to be valid, mutation)
const MUTATIONS: &[Mutation] = &[
    ("rename business", true, |s| s.settings.business_name = "Renamed".into()),
    ("edit draft notes", true, |s| s.draft.notes = "hello".into()),
    ("add customer", true, |s| s.customers.push(customer("Another"))),
    ("add second catalog", true, |s| {
        let c = catalog("second", 20);
        s.catalogs.push(c);
    }),
    ("tax rate out of range", false, |s| s.settings.tax_basis_points = 20_000),
    ("catalog fingerprint corrupted", false, |s| s.catalogs[0].fingerprint = "bad".into()),
    ("duplicate customer id", false, |s| {
        let c = s.customers[0].clone();
        s.customers.push(c);
    }),
    ("negative draft discount", false, |s| s.draft.discount_rial = -1),
    ("draft points at a missing customer", false, |s| s.draft.customer_id = id()),
    ("product price zero", false, |s| s.catalogs[0].products[0].price_rial = 0),
    ("period lock flipped without audit event", false, |s| {
        let p = &mut s.accounting.periods[0];
        p.locked = !p.locked;
    }),
    ("unknown company in desktop preferences", false, |s| s.desktop.active_company = id()),
];

fn verdicts(base: &State, candidate: &State) -> (Result<(), String>, Result<(), String>) {
    (
        candidate.validate().map_err(|e| e.to_string()),
        candidate.validate_changes(base).map_err(|e| e.to_string()),
    )
}

#[test]
fn every_single_mutation_gets_the_same_verdict_and_message_on_both_paths() {
    let base = base();
    for (name, expect_valid, mutate) in MUTATIONS {
        let mut candidate = base.clone();
        mutate(&mut candidate);
        let (full, fast) = verdicts(&base, &candidate);
        assert_eq!(full.is_ok(), *expect_valid, "full validator on '{name}': {full:?}");
        assert_eq!(full, fast, "paths disagree on '{name}'");
    }
}

/// Deterministic xorshift so a failure is reproducible from the printed seed.
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn random_combinations_agree() {
    let base = base();
    let mut rng = 0x9E37_79B9_7F4A_7C15_u64;
    let mut saw_valid = 0;
    let mut saw_invalid = 0;
    for round in 0..300 {
        let mut candidate = base.clone();
        let mut applied = vec![];
        for _ in 0..(1 + next(&mut rng) % 3) {
            let (name, _, mutate) = MUTATIONS[(next(&mut rng) % MUTATIONS.len() as u64) as usize];
            // Mutations index into vectors; skip ones that no longer apply after earlier edits.
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut probe = candidate.clone();
                mutate(&mut probe);
            }))
            .is_err()
            {
                continue;
            }
            mutate(&mut candidate);
            applied.push(name);
        }
        let (full, fast) = verdicts(&base, &candidate);
        assert_eq!(full, fast, "round {round} {applied:?}");
        if full.is_ok() { saw_valid += 1 } else { saw_invalid += 1 }
    }
    assert!(saw_valid > 20 && saw_invalid > 20, "mix too lopsided: {saw_valid}/{saw_invalid}");
}

#[test]
fn unchanged_state_is_accepted_without_work() {
    let base = base();
    assert!(base.validate_changes(&base).is_ok());
}

/// The fast path trusts its base. Corruption already inside the base is invisible to it,
/// which is exactly why backup, close and the background check run the full validator.
fn store_with_hidden_corruption() -> (tempfile::TempDir, Store, String) {
    let dir = tempfile::tempdir().unwrap();
    seed_fixture(dir.path()).unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store
        .change(|s| {
            imports::accept_catalog(s, catalog("c", 10))?;
            Ok(())
        })
        .unwrap();
    let company = store.state.accounting.companies[0].id.clone();
    store.state.catalogs[0].fingerprint = "corrupted-in-memory".into();
    (dir, store, company)
}

#[test]
fn fast_save_misses_hidden_corruption_but_every_safety_net_catches_it() {
    let (dir, mut store, company) = store_with_hidden_corruption();
    // 1. An unrelated save passes the fast path (this is the accepted trade-off)…
    store.change(|s| { s.settings.business_name = "unrelated".into(); Ok(()) }).unwrap();
    // 2. …but the full validator, backup and period close all refuse the same book.
    assert!(store.validate_full().is_err());
    assert!(store.backup(&dir.path().join("b.json")).is_err(), "backup must not be written");
    let period = store.state.accounting.periods[0].id.clone();
    let lock = json!({"company_id": company, "period_id": period, "locked": true, "reason": "close", "request_key": "k"});
    let err = dispatch_in(&mut store, "period_lock", lock).unwrap_err().to_string();
    assert!(!err.is_empty());
    assert!(!store.state.accounting.periods[0].locked, "period must stay open");
}

fn wait_for(store: &Store, done: impl Fn(&IntegrityStatus) -> bool) -> IntegrityStatus {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let s = store.integrity();
        if done(&s) || Instant::now() > deadline {
            return s;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn background_check_reports_ok_or_failed_and_failed_blocks_close_with_the_reason() {
    let dir = tempfile::tempdir().unwrap();
    seed_fixture(dir.path()).unwrap();
    let good = Store::open(dir.path()).unwrap();
    assert_eq!(good.integrity(), IntegrityStatus::Unchecked);
    good.start_background_check();
    assert!(matches!(wait_for(&good, |s| !matches!(s, IntegrityStatus::Running { .. })), IntegrityStatus::Ok { .. }));

    let (_d, mut bad, company) = store_with_hidden_corruption();
    bad.start_background_check();
    let status = wait_for(&bad, |s| !matches!(s, IntegrityStatus::Running { .. }));
    let IntegrityStatus::Failed { message, .. } = status else { panic!("expected Failed, got {status:?}") };
    assert!(!message.is_empty());
    // The status is visible to the UI through the workspace and the dedicated action.
    let w: Value = dispatch_in(&mut bad, "workspace", json!({"company_id": company})).unwrap();
    assert_eq!(w["integrity"]["status"], "failed");
    let direct = dispatch_in(&mut bad, "integrity", json!({})).unwrap();
    assert_eq!(direct["status"], "failed");
    // Failed integrity blocks close and names the cause.
    let period = bad.state.accounting.periods[0].id.clone();
    let err = dispatch_in(&mut bad, "period_lock", json!({"company_id": company, "period_id": period, "locked": true, "reason": "close", "request_key": "k2"}))
        .unwrap_err()
        .to_string();
    assert!(err.contains("Integrity check failed") && err.contains(&message), "{err}");
}

#[test]
fn closing_a_healthy_period_still_works() {
    let dir = tempfile::tempdir().unwrap();
    seed_fixture(dir.path()).unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let company = store.state.accounting.companies[0].id.clone();
    let period = store.state.accounting.periods[0].id.clone();
    let locked = store.state.accounting.periods[0].locked;
    if locked {
        return; // fixture already closed; nothing to prove here
    }
    dispatch_in(&mut store, "period_lock", json!({"company_id": company, "period_id": period, "locked": true, "reason": "month end", "request_key": "close-1"})).unwrap();
    assert!(store.state.accounting.periods[0].locked);
}

#[test]
fn fixture_period_starts_open_so_the_close_test_is_not_vacuous() {
    let dir = tempfile::tempdir().unwrap();
    seed_fixture(dir.path()).unwrap();
    let store = Store::open(dir.path()).unwrap();
    assert!(!store.state.accounting.periods[0].locked);
}
