use sarv_quote::{accounting::*, domain::State, store::Store};

fn posted() -> State {
    let mut state = State::default();
    let book = &mut state.accounting;
    let company = book.add_company("شرکت آزمون حفاظت", "آزمایشگر").unwrap();
    let period = book
        .add_period(
            &company,
            "سال آزمون",
            "2026-01-01",
            "2026-12-31",
            "آزمایشگر",
        )
        .unwrap();
    let mut ids = vec![];
    for (code, classification, normal_balance) in [
        ("1101", AccountClass::Asset, BalanceSide::Debit),
        ("3101", AccountClass::Equity, BalanceSide::Credit),
    ] {
        ids.push(
            book.add_account(
                NewAccount {
                    company_id: company.clone(),
                    parent_id: None,
                    code: code.into(),
                    name: format!("حساب {code}"),
                    classification,
                    normal_balance,
                    postable: true,
                    control: false,
                },
                "آزمایشگر",
            )
            .unwrap(),
        );
    }
    book.post(PostCommand {
        company_id: company,
        period_id: period,
        date: "2026-01-01".into(),
        source_id: "initial-fixture".into(),
        source_revision: 1,
        event: "opening".into(),
        request_key: "initial-key".into(),
        actor: "آزمایشگر".into(),
        reason: "ماندهٔ مصنوعی".into(),
        kind: JournalKind::Opening,
        lines: vec![
            JournalLineInput {
                account_id: ids[0].clone(),
                debit: Money::from_rials(1251),
                credit: Money::ZERO,
                description: "دارایی".into(),
            },
            JournalLineInput {
                account_id: ids[1].clone(),
                debit: Money::ZERO,
                credit: Money::from_rials(1251),
                description: "سرمایه".into(),
            },
        ],
    })
    .unwrap();
    state
}

#[test]
fn cached_state_cannot_bypass_committed_history_in_any_save_path() {
    for path in ["ordinary", "prepared", "detached"] {
        let dir = tempfile::tempdir().unwrap();
        let original = posted();
        let mut store = Store::open(dir.path()).unwrap();
        store.restore(original.clone()).unwrap();
        store.state.accounting = AccountingState::default();
        let result = match path {
            "ordinary" => store.change(|s| {
                s.draft.customer = "تغییر متن".into();
                Ok(())
            }),
            "prepared" => {
                let mut candidate = store.state.clone();
                candidate.draft.customer = "تغییر متن".into();
                let prepared =
                    Store::prepare_candidate(store.local_generation(), candidate).unwrap();
                store.commit_prepared(prepared)
            }
            _ => {
                let (generation, mut candidate) = store.begin_detached().unwrap();
                candidate.draft.customer = "تغییر متن".into();
                let prepared = Store::prepare_candidate(generation, candidate).unwrap();
                let result = Store::commit_detached(dir.path().to_path_buf(), prepared);
                store.finish_detached(result)
            }
        };
        assert!(result.is_err(), "cached-state bypass via {path}");
        assert_eq!(
            serde_json::to_vec(&Store::open(dir.path()).unwrap().state).unwrap(),
            serde_json::to_vec(&original).unwrap()
        );
    }
}

#[test]
fn explicit_replacement_requires_the_exact_committed_recovery_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let original = posted();
    store.restore(original.clone()).unwrap();
    let backup = dir.path().join("books.json");
    store.backup(&backup).unwrap();
    // A normal candidate is unable to erase the journal. Explicit replacement
    // with its verified exact-state recovery copy remains the separate path.
    let ordinary = Store::prepare_candidate(store.local_generation(), State::default()).unwrap();
    assert!(store.commit_prepared(ordinary).is_err());
    let approved = Store::prepare_replacement(
        store.local_generation(),
        &original,
        State::default(),
        &backup,
    )
    .unwrap();
    store.commit_prepared(approved).unwrap();
    assert!(store.state.accounting.is_empty());
    assert_eq!(
        Store::restore_preview(&backup).unwrap().accounting,
        original.accounting
    );
    assert!(
        Store::prepare_replacement(
            store.local_generation(),
            &State::default(),
            State::default(),
            &backup
        )
        .is_err()
    );
}
