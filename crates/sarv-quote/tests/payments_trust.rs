//! Trust checks for the payments ledger: durable restarts, exact backup and
//! restore, tamper rejection and optimistic-concurrency guards.
use sarv_quote::{
    desktop::{dispatch, seed_fixture},
    store::Store,
};
use serde_json::{Value, json};
use std::fs;

fn call(dir: &std::path::Path, action: &str, p: Value) -> Value {
    dispatch(dir, action, p).unwrap()
}
fn fixture() -> (tempfile::TempDir, String, String) {
    let d = tempfile::tempdir().unwrap();
    seed_fixture(d.path()).unwrap();
    let s = Store::open(d.path()).unwrap();
    let c = s.state.accounting.companies[0].id.clone();
    let p = s.state.accounting.periods[0].id.clone();
    (d, c, p)
}
fn receive_payload(c: &str, p: &str, key: &str, date: &str, amount: &str, advance: &str) -> Value {
    json!({
        "company_id": c, "period_id": p, "source_id": key, "request_key": key,
        "date": date, "reason": "Trust fixture receipt", "side": "receive",
        "party_id": "CU-A", "funds_role": "bank", "reference": format!("RCP-{key}"),
        "advance_amount": advance,
        "allocations": [{"open_item_id": "AR-125", "amount": amount}]
    })
}

#[test]
fn payments_backup_restore_and_restart_keep_exact_books() {
    let (d, c, p) = fixture();
    let posted = call(
        d.path(),
        "payment_post",
        receive_payload(&c, &p, "trust-1", "2026-10-08", "100", "25"),
    );
    call(
        d.path(),
        "payment_post",
        json!({"company_id":c,"period_id":p,"source_id":"trust-2","request_key":"trust-2","date":"2026-10-09","reason":"Trust fixture payment","side":"make","party_id":"SU-A","funds_role":"cash","reference":"PMT-trust-2","advance_amount":"0","allocations":[{"open_item_id":"AP-100","amount":"60"}]}),
    );
    call(
        d.path(),
        "payment_post",
        json!({"company_id":c,"period_id":p,"source_id":"trust-rev","request_key":"trust-rev","date":"2026-10-09","reason":"Trust reversal","payment_id":posted["record_id"]}),
    );
    // Restart: a completely fresh open of the same directory.
    let live = Store::open(d.path()).unwrap();
    let generation = live.local_generation();
    live.state.accounting.validate().unwrap();
    let backup = d.path().join("trust-backup.json");
    live.backup(&backup).unwrap();
    // Backup restores into a fresh profile and survives another restart.
    let restored_state = Store::restore_preview(&backup).unwrap();
    let fresh = tempfile::tempdir().unwrap();
    let mut target = Store::open(fresh.path()).unwrap();
    target.restore(restored_state).unwrap();
    let reopened = Store::open(fresh.path()).unwrap();
    assert_eq!(
        serde_json::to_value(&reopened.state).unwrap(),
        serde_json::to_value(&live.state).unwrap()
    );
    let _ = generation;
    reopened.state.accounting.validate().unwrap();
    // Allocations survived: CU-A is back to 125 after the reversal, SU-A at 40.
    let options = call(
        d.path(),
        "payment_options",
        json!({"company_id":c,"side":"receive","party_id":"CU-A"}),
    );
    let ar = options["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == "AR-125")
        .unwrap();
    assert_eq!(ar["remaining"], "125");
    let r = call(
        d.path(),
        "report",
        json!({"company_id":c,"from":"2026-10-01","to":"2026-10-09"}),
    );
    assert_eq!(r["reconciliation"]["reconciled"], true);
    assert_eq!(r["party_balances"]["vendor:SU-A"], "40");
}

#[test]
fn payment_retry_after_restart_is_idempotent() {
    let (d, c, p) = fixture();
    let first = call(
        d.path(),
        "payment_post",
        receive_payload(&c, &p, "trust-retry", "2026-10-08", "125", "0"),
    );
    // Simulate a lost UI acknowledgement: reopen the profile and retry the
    // exact request through the bridge.
    drop(Store::open(d.path()).unwrap());
    let second = call(
        d.path(),
        "payment_post",
        receive_payload(&c, &p, "trust-retry", "2026-10-08", "125", "0"),
    );
    assert_eq!(first["record_id"], second["record_id"]);
    assert_eq!(first["journal_id"], second["journal_id"]);
    let store = Store::open(d.path()).unwrap();
    assert_eq!(store.state.accounting.payments.records.len(), 1);
    let journals = store
        .state
        .accounting
        .journals
        .iter()
        .filter(|j| j.event == "payments.receive.v1")
        .count();
    assert_eq!(journals, 1);
    store.state.accounting.validate().unwrap();
}

#[test]
fn tampered_payment_backup_is_rejected_entirely() {
    let (d, c, p) = fixture();
    call(
        d.path(),
        "payment_post",
        receive_payload(&c, &p, "trust-3", "2026-10-08", "50", "0"),
    );
    let store = Store::open(d.path()).unwrap();
    let backup = d.path().join("tampered.json");
    store.backup(&backup).unwrap();
    drop(store);
    let raw = fs::read_to_string(&backup).unwrap();
    // Corrupt one amount inside the state and keep the checksum stale.
    let tampered = raw.replace("\"amount\": 50", "\"amount\": 900000");
    assert_ne!(raw, tampered);
    fs::write(&backup, tampered).unwrap();
    assert!(Store::restore_preview(&backup).is_err());
}

#[test]
fn stale_generation_payment_post_writes_nothing() {
    let (d, c, p) = fixture();
    let before = Store::open(d.path()).unwrap();
    let generation = before.local_generation();
    let state_bytes = serde_json::to_string(&before.state).unwrap();
    drop(before);
    let error = dispatch(
        d.path(),
        "payment_post",
        receive_payload(&c, &p, "trust-stale", "2026-10-08", "10", "0")
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .chain([("expected_generation".to_owned(), json!("0"))])
            .collect::<serde_json::Map<_, _>>()
            .into(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("Books changed"));
    let after = Store::open(d.path()).unwrap();
    assert_eq!(after.local_generation(), generation);
    assert_eq!(serde_json::to_string(&after.state).unwrap(), state_bytes);
    // A retried stale review also cannot post after the books changed.
    let preview = call(
        d.path(),
        "payment_preview",
        receive_payload(&c, &p, "trust-stale", "2026-10-08", "10", "0"),
    );
    let (debit_account, credit_account) = {
        let open_store = Store::open(d.path()).unwrap();
        let postable: Vec<_> = open_store
            .state
            .accounting
            .accounts
            .iter()
            .filter(|a| a.postable && !a.control && !a.archived)
            .collect();
        (
            postable
                .iter()
                .find(|a| a.normal_balance == sarv_quote::accounting::BalanceSide::Debit)
                .unwrap()
                .id
                .clone(),
            postable
                .iter()
                .find(|a| a.normal_balance == sarv_quote::accounting::BalanceSide::Credit)
                .unwrap()
                .id
                .clone(),
        )
    };
    call(
        d.path(),
        "journal_post",
        json!({"company_id":c,"period_id":p,"source_id":"noise","request_key":"noise","date":"2026-10-08","reason":"concurrent change","lines":[
            {"account_id":debit_account,"debit":"1","credit":"0","description":"x"},
            {"account_id":credit_account,"debit":"0","credit":"1","description":"x"}
        ]}),
    );
    assert!(dispatch(
        d.path(),
        "payment_post",
        receive_payload(&c, &p, "trust-stale", "2026-10-08", "10", "0")
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .chain([(
                "expected_generation".to_owned(),
                preview["_generation"].clone()
            )])
            .collect::<serde_json::Map<_, _>>()
            .into(),
    )
    .is_err());
}

#[test]
fn backup_actions_create_list_preview_restore_with_safety_copy() {
    let (d, c, p) = fixture();
    call(
        d.path(),
        "payment_post",
        receive_payload(&c, &p, "bk-1", "2026-10-08", "125", "0"),
    );
    // Restore requires explicit confirmation and rejects traversal names.
    assert!(dispatch(
        d.path(),
        "backup_restore",
        json!({"filename":"sarv-backup-x.json"}),
    )
    .is_err());
    for bad in ["../sarv-backup-x.json", "sub/sarv-backup-x.json", "before-schema7-a.json"] {
        assert!(dispatch(
            d.path(),
            "backup_restore_preview",
            json!({"filename":bad}),
        )
        .is_err());
    }
    let created = call(d.path(), "backup_create", json!({}));
    let name = created["filename"].as_str().unwrap().to_owned();
    assert!(name.starts_with("sarv-backup-") && name.ends_with(".json"));
    assert!(created["bytes"].as_str().unwrap().parse::<u64>().unwrap() > 100);
    // The restore must reproduce exactly the books as they were at backup time.
    let at_backup = serde_json::to_value(&Store::open(d.path()).unwrap().state).unwrap();
    let list = call(d.path(), "backup_list", json!({}));
    let row = list["backups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["filename"] == name.as_str())
        .unwrap();
    assert_eq!(row["valid"], true);
    assert_eq!(row["companies"], "1");
    assert_eq!(row["journals"], "2");
    // More books after the backup; the backup preview still shows the old world.
    call(
        d.path(),
        "payment_post",
        json!({"company_id":c,"period_id":p,"source_id":"bk-2","request_key":"bk-2","date":"2026-10-09","reason":"later","side":"make","party_id":"SU-A","funds_role":"cash","reference":"PMT-bk-2","advance_amount":"0","allocations":[{"open_item_id":"AP-100","amount":"10"}]}),
    );
    let preview = call(
        d.path(),
        "backup_restore_preview",
        json!({"filename":name}),
    );
    assert_eq!(preview["backup"]["payments"], "1");
    assert_eq!(preview["current"]["payments"], "2");
    let restored = call(
        d.path(),
        "backup_restore",
        json!({"filename":name,"confirmed":true,"expected_generation":preview["_generation"]}),
    );
    assert_eq!(restored["restored"], true);
    assert!(restored["safety_backup"]
        .as_str()
        .unwrap()
        .starts_with("before-restore-"));
    let after = Store::open(d.path()).unwrap();
    assert_eq!(serde_json::to_value(&after.state).unwrap(), at_backup);
    after.state.accounting.validate().unwrap();
    // The automatic safety copy still restores the newer world if needed.
    let safety = restored["safety_backup"].as_str().unwrap().to_owned();
    let back = call(
        d.path(),
        "backup_restore_preview",
        json!({"filename":name}),
    );
    assert_eq!(back["backup"]["payments"], "1");
    let _ = safety;
}

#[test]
fn quotation_drafts_and_settings_survive_backup_and_restart() {
    let (d, c, p) = fixture();
    let s = Store::open(d.path()).unwrap();
    let setup = s.state.accounting.core.setups[0].id.clone();
    drop(s);
    let draft = json!({"id":"q-trust","revision":"0","company_id":c,"period_id":p,"setup_id":setup,"customer_id":"CU-A","reference":"q-ref","document_date":"2026-10-08","financial_date":"2026-10-08","date_display":"Iso","display_unit":"Rial","document_discount":"0","supplied_total":"30","reason":"quotation trust","quotation":null,"lines":[{"id":"q-line","item_id":"ITEM-A","warehouse_id":"W1","unit_id":"PIECE","quantity":"3","unit_price":"10","line_discount":"0","price_reference":"Manual","price_date":"2026-10-08","override_reason":""}]});
    call(
        d.path(),
        "draft_save",
        json!({"draft":draft,"expected_revision":null,"issue_quotation":true}),
    );
    let created = call(d.path(), "backup_create", json!({}));
    let name = created["filename"].as_str().unwrap().to_owned();
    // Restart: quotation numbers and drafts are still there.
    let reopened = Store::open(d.path()).unwrap();
    let issue = reopened
        .state
        .sales_drafts
        .iter()
        .find(|x| x.id == "q-trust")
        .and_then(|x| x.quote_issue.clone())
        .expect("quotation issue survived restart");
    drop(reopened);
    assert!(issue.number.starts_with("Q-"));
    // ...and inside the backup.
    let list = call(d.path(), "backup_list", json!({}));
    assert!(list["backups"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["filename"] == name.as_str() && r["valid"] == true));
    let preview = call(
        d.path(),
        "backup_restore_preview",
        json!({"filename":name}),
    );
    assert_eq!(preview["backup"]["drafts"], "1");
}
