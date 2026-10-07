//! Durable core commands use the same CAS/immutable transition boundary as quotes.
use sarv_quote::{
    accounting::Money, core::*, core_import::opening_csv, domain::State, imports::hash,
    store::Store,
};
fn installed(store: &mut Store) -> String {
    store
        .change(|s| {
            s.accounting
                .setup_core_company(NewCoreCompany {
                    name: "F2 synthetic store".into(),
                    fiscal_name: "1405".into(),
                    start_date: "2026-03-21".into(),
                    end_date: "2027-03-20".into(),
                    timezone: CompanyTimezone::AsiaTehran,
                    chart: default_chart(),
                    actor: "test".into(),
                })
                .map(|r| r.company_id)
        })
        .unwrap()
}
fn command(state: &State, company: &str, key: &str) -> OpeningCommand {
    let input =
        include_str!("../docs/assets/accounting-m3/opening.csv").replace("F2-core-v1", company);
    opening_csv(&state.accounting, company, &input, "test", key).unwrap()
}
fn exact(s: &State) -> Vec<u8> {
    serde_json::to_vec(s).unwrap()
}
#[test]
fn concurrent_prepared_opening_has_one_durable_effect_then_different_key_retry_reuses_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let company = installed(&mut store);
    let (generation, mut first) = store.snapshot().unwrap();
    let mut second = first.clone();
    first
        .accounting
        .post_opening(command(&first, &company, "worker-1"))
        .unwrap();
    second
        .accounting
        .post_opening(command(&second, &company, "worker-2"))
        .unwrap();
    let first = Store::prepare_candidate(generation, first).unwrap();
    let second = Store::prepare_candidate(generation, second).unwrap();
    let path = dir.path().to_owned();
    let receipt = std::thread::spawn(move || Store::commit_detached(path, first))
        .join()
        .unwrap()
        .unwrap();
    store.adopt_committed(receipt).unwrap();
    assert!(Store::commit_detached(dir.path().to_owned(), second).is_err());
    let stored = Store::open(dir.path()).unwrap();
    assert_eq!(stored.state.accounting.journals.len(), 1);
    assert_eq!(stored.state.accounting.core.openings.len(), 1);
    let original = stored.state.accounting.core.openings[0].journal_id.clone();
    let retry = command_from_record(&stored.state, "worker-2");
    assert_eq!(
        store.change(|s| s.accounting.post_opening(retry)).unwrap(),
        original
    );
    assert_eq!(store.state.accounting.journals.len(), 1);
    assert_eq!(store.state.accounting.requests.len(), 2);
    assert_eq!(store.state.accounting.core.openings.len(), 1);
    let p = store
        .state
        .accounting
        .reconcile_opening(&company, "2026-03-21")
        .unwrap();
    assert_eq!(
        (p.total_debit.rials(), p.total_credit.rials()),
        (1251, 1251)
    );
    assert_eq!(p.controls.len(), 12);
    assert!(p.controls.iter().all(|r| r.difference == Money::ZERO));
    let expected = store.state.accounting.clone();
    drop(store);
    assert_eq!(Store::open(dir.path()).unwrap().state.accounting, expected);
}
fn command_from_record(state: &State, key: &str) -> OpeningCommand {
    let mut c = state.accounting.core.openings[0].command.clone();
    c.request_key = key.into();
    c
}
#[test]
fn immutable_details_policy_and_recomputed_backup_checksum_cannot_bypass_restore_validation() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let company = installed(&mut store);
    let c = command(&store.state, &company, "opening");
    store.change(|s| s.accounting.post_opening(c)).unwrap();
    let before = exact(&store.state);
    let generation = store.local_generation();
    assert!(
        store
            .change(|s| {
                s.accounting.core.openings[0].command.stock[0].quantity_thousandths = 3000;
                Ok(())
            })
            .is_err()
    );
    assert!(
        store
            .change(|s| {
                s.accounting.core.setups[0].command.policy.timezone = CompanyTimezone::Utc;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(exact(&store.state), before);
    assert_eq!(store.local_generation(), generation);
    let mut forged = store.state.clone();
    forged.accounting.core.openings[0].command.open_items[0].due_date = "2026-06-01".into();
    let payload = exact(&forged);
    let p = dir.path().join("forged.json");
    std::fs::write(&p,serde_json::to_vec(&serde_json::json!({"format":"sarv-accounting-backup-v4","checksum":hash(&payload),"state":forged})).unwrap()).unwrap();
    assert!(Store::restore_preview(&p).is_err());
    assert_eq!(exact(&Store::open(dir.path()).unwrap().state), before);
}
#[test]
fn complete_core_backup_fresh_restore_and_restart_keep_all_financial_and_source_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let company = installed(&mut store);
    let c = command(&store.state, &company, "opening");
    store.change(|s| s.accounting.post_opening(c)).unwrap();
    let before = exact(&store.state);
    let backup = dir.path().join("core-v4.json");
    store.backup(&backup).unwrap();
    let fresh = tempfile::tempdir().unwrap();
    let mut restored = Store::open(fresh.path()).unwrap();
    restored
        .restore(Store::restore_preview(&backup).unwrap())
        .unwrap();
    assert_eq!(exact(&restored.state), before);
    drop(restored);
    let restored = Store::open(fresh.path()).unwrap();
    assert_eq!(exact(&restored.state), before);
    assert_eq!(
        restored.state.accounting.core.openings[0]
            .command
            .import_source
            .as_ref()
            .unwrap()
            .rows
            .len(),
        35
    );
    if let Ok(output) = std::env::var("SARV_M3_CORE_RESTORE_ARTIFACT_DIR") {
        std::fs::create_dir_all(&output).unwrap();
        std::fs::copy(backup, std::path::Path::new(&output).join("core-v4.json")).unwrap();
        std::fs::write(std::path::Path::new(&output).join("verification.json"),serde_json::to_vec_pretty(&serde_json::json!({"result":"passed","fresh_restore_full_state_equal":true,"restart_equal":true,"state_sha256":hash(&before),"journal_count":1,"total_debit_irr":1251,"total_credit_irr":1251,"controls":12,"all_control_differences_irr":0,"complete_source_rows":restored.state.accounting.core.openings[0].command.import_source.as_ref().unwrap().rows.len()})).unwrap()).unwrap();
    }
}

#[test]
fn zero_value_initialization_and_its_retry_restore_without_an_empty_financial_journal() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let company = installed(&mut store);
    let mut c = command(&store.state, &company, "zero-init");
    c.source_id = "ZERO-DECLARED-SYNTHETIC".into();
    c.reason = "موجودی با بهای صفر صریح؛ بدون ماندهٔ پولی".into();
    c.import_source = None;
    for b in &mut c.balances {
        b.debit = Money::ZERO;
        b.credit = Money::ZERO;
    }
    for x in &mut c.cash {
        x.balance = Money::ZERO;
    }
    for x in &mut c.banks {
        x.balance = Money::ZERO;
        x.statement_balance = Money::ZERO;
    }
    c.open_items.clear();
    c.cheques.clear();
    for x in &mut c.stock {
        x.value = Money::ZERO;
    }
    for x in &mut c.grni {
        x.value = Money::ZERO;
    }
    let marker = store
        .change(|s| s.accounting.post_opening(c.clone()))
        .unwrap();
    c.request_key = "zero-other-key".into();
    assert_eq!(
        store.change(|s| s.accounting.post_opening(c)).unwrap(),
        marker
    );
    assert!(store.state.accounting.journals.is_empty());
    assert!(store.state.accounting.requests.is_empty());
    assert_eq!(store.state.accounting.core.zero_requests.len(), 2);
    assert_eq!(
        store.state.accounting.core.openings[0].effect,
        OpeningEffect::ZeroBalanceInitialization
    );
    let p = store
        .state
        .accounting
        .reconcile_opening(&company, "2026-03-21")
        .unwrap();
    assert_eq!(p.total_debit, Money::ZERO);
    assert_eq!(p.total_credit, Money::ZERO);
    assert_eq!(p.controls.len(), 12);
    assert!(p.reconciled);
    assert_eq!(
        store.state.accounting.core.openings[0]
            .command
            .base_quantity("ITEM-A", "CARTON", 2000)
            .unwrap(),
        24000
    );
    let expected = exact(&store.state);
    let backup = dir.path().join("zero-v4.json");
    store.backup(&backup).unwrap();
    let fresh = tempfile::tempdir().unwrap();
    let mut restored = Store::open(fresh.path()).unwrap();
    restored
        .restore(Store::restore_preview(&backup).unwrap())
        .unwrap();
    assert_eq!(exact(&restored.state), expected);
    drop(restored);
    assert_eq!(exact(&Store::open(fresh.path()).unwrap().state), expected);
}
