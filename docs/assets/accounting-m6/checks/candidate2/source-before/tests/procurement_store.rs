use sarv_quote::{
    accounting::Money,
    procurement::{
        OrderLine, ProcurementCommand, ProcurementContext, PurchaseOrder, ReceiveLine, ReceiveOrder,
    },
    store::Store,
};
use std::{fs, path::Path};

fn equal_state(actual: &sarv_quote::domain::State, expected: &sarv_quote::domain::State) {
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
}

fn installed(path: &Path) -> Store {
    let mut store = Store::open(path).unwrap();
    let backup = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs/assets/accounting-m3/restore/core-v4.json");
    store
        .restore(Store::restore_preview(&backup).unwrap())
        .unwrap();
    store
}
fn context(store: &Store, source: &str, key: &str) -> ProcurementContext {
    let opening = &store.state.accounting.core.openings[0];
    ProcurementContext {
        company_id: opening.command.company_id.clone(),
        period_id: opening.command.period_id.clone(),
        setup_id: opening.command.setup_id.clone(),
        source_id: source.into(),
        source_revision: 1,
        date: "2026-03-21".into(),
        request_key: key.into(),
        actor: "synthetic-store-review".into(),
        reason: "Synthetic purchase workflow; no live data".into(),
    }
}
fn order(store: &Store) -> ProcurementCommand {
    let c = &store.state.accounting.core.openings[0].command;
    ProcurementCommand::Order(PurchaseOrder {
        context: context(store, "STORE-PO-1", "store-order-1"),
        supplier_id: c.parties.iter().find(|p| p.supplier).unwrap().id.clone(),
        reference: "STORE-SUPPLIER-PO-1".into(),
        document_date: "2026-03-21".into(),
        lines: vec![OrderLine {
            id: "STORE-PO-1-L1".into(),
            item_id: c.items[0].id.clone(),
            warehouse_id: c.warehouses[0].id.clone(),
            unit_id: c.items[0].base_unit_id.clone(),
            quantity_thousandths: 12_000,
            unit_price: Money::from_rials(15),
            line_discount: Money::ZERO,
            price_reference: "synthetic-price-15-IRR-per-piece".into(),
            price_date: "2026-03-21".into(),
            override_reason: Some("Independent artificial purchase fixture".into()),
        }],
        document_discount: Money::ZERO,
        supplied_total: Money::from_rials(180),
    })
}
fn receipt(store: &Store, order_id: &str, key: &str) -> ProcurementCommand {
    let c = &store.state.accounting.core.openings[0].command;
    ProcurementCommand::Receive(ReceiveOrder {
        context: context(store, "STORE-RECEIPT-1", key),
        order_id: order_id.into(),
        reference: "STORE-RECEIPT-1".into(),
        document_date: "2026-03-21".into(),
        lines: vec![ReceiveLine {
            id: "STORE-RECEIPT-1-L1".into(),
            order_line_id: "STORE-PO-1-L1".into(),
            warehouse_id: c.warehouses[0].id.clone(),
            unit_id: c.items[0].base_unit_id.clone(),
            quantity_thousandths: 5_000,
        }],
        supplied_total: Money::from_rials(75),
    })
}

#[test]
fn durable_competing_receipts_have_one_winner_and_losing_different_key_retry_resolves_it() {
    let profile = tempfile::tempdir().unwrap();
    let mut store = installed(profile.path());
    let command = order(&store);
    let order_id = store
        .change(|s| s.accounting.post_procurement_command(command))
        .unwrap()
        .record_id;
    let (generation, original) = store.snapshot().unwrap();
    let mut first = original.clone();
    first
        .accounting
        .post_procurement_command(receipt(&store, &order_id, "receipt-first"))
        .unwrap();
    let mut second = original;
    second
        .accounting
        .post_procurement_command(receipt(&store, &order_id, "receipt-second"))
        .unwrap();
    let a = Store::prepare_candidate(generation, first).unwrap();
    let b = Store::prepare_candidate(generation, second).unwrap();
    store.commit_prepared(a).unwrap();
    assert!(store.commit_prepared(b).is_err());
    let company = store.state.accounting.core.openings[0]
        .command
        .company_id
        .clone();
    let before_journals = store.state.accounting.journals.len();
    let retry = receipt(&store, &order_id, "receipt-second");
    store
        .change(|s| s.accounting.post_procurement_command(retry))
        .unwrap();
    assert_eq!(store.state.accounting.journals.len(), before_journals);
    assert_eq!(store.state.accounting.procurement.records.len(), 2);
    assert_eq!(store.state.accounting.procurement.requests.len(), 3);
    let report = store
        .state
        .accounting
        .procurement_reconcile(&company, "2026-03-21")
        .unwrap();
    assert!(report.reconciled);
    assert_eq!(
        report.stock.iter().map(|p| p.value.rials()).sum::<i64>(),
        375
    );
    let exact = store.state.clone();
    drop(store);
    equal_state(&Store::open(profile.path()).unwrap().state, &exact);
}

#[test]
fn complete_operational_snapshot_fresh_restore_and_mutated_posted_cost_rejection() {
    let profile = tempfile::tempdir().unwrap();
    let mut store = installed(profile.path());
    let command = order(&store);
    let order_id = store
        .change(|s| s.accounting.post_procurement_command(command))
        .unwrap()
        .record_id;
    let command = receipt(&store, &order_id, "receipt-first");
    store
        .change(|s| s.accounting.post_procurement_command(command))
        .unwrap();
    let exact = store.state.clone();
    let generation = store.local_generation();
    let mut tampered = exact.clone();
    tampered.accounting.procurement.records[1].movements[0]
        .after
        .value = Money::from_rials(1);
    assert!(Store::prepare_candidate(generation, tampered).is_err());
    equal_state(&store.state, &exact);
    assert_eq!(store.local_generation(), generation);
    let backup = profile.path().join("procurement-v5.json");
    store.backup(&backup).unwrap();
    let fresh = tempfile::tempdir().unwrap();
    let mut restored = Store::open(fresh.path()).unwrap();
    restored
        .restore(Store::restore_preview(&backup).unwrap())
        .unwrap();
    equal_state(&restored.state, &exact);
    drop(restored);
    equal_state(&Store::open(fresh.path()).unwrap().state, &exact);
    if let Ok(root) = std::env::var("SARV_M4_RESTORE_ARTIFACT_DIR") {
        fs::create_dir_all(&root).unwrap();
        fs::copy(&backup, Path::new(&root).join("procurement-v5.json")).unwrap();
        fs::write(Path::new(&root).join("verification.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "result":"passed", "schema":5,"fresh_restore_and_restart_equal":true,
            "posted_cost_tamper_rejected":true,"records":exact.accounting.procurement.records.len(),
            "requests":exact.accounting.procurement.requests.len(),"journals":exact.accounting.journals.len()
        })).unwrap()).unwrap();
    }
}
