use sarv_quote::{
    accounting::AccountClass,
    desktop::{dispatch, seed_fixture},
    store::Store,
};
use serde_json::{Value, json};

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

#[test]
fn multi_period_books_isolate_pnl_and_carry_position_balances() {
    let (d, c, _p1) = fixture();

    let year1_before = call(
        d.path(),
        "report",
        json!({"company_id": c, "from": "2026-03-21", "to": "2027-03-20"}),
    );
    let y1_cash: i64 = year1_before["position"]["cash"].as_str().unwrap().parse().unwrap();
    let y1_assets: i64 = year1_before["balance_sheet"]["total_assets"].as_str().unwrap().parse().unwrap();
    assert!(y1_assets > 0);

    let store = Store::open(d.path()).unwrap();
    let generation = store.local_generation().to_string();
    let expense_account = store
        .state
        .accounting
        .accounts
        .iter()
        .find(|a| a.company_id == c && !a.control && a.postable && a.classification == AccountClass::Expense)
        .unwrap()
        .id
        .clone();
    let equity_account = store
        .state
        .accounting
        .accounts
        .iter()
        .find(|a| a.company_id == c && !a.control && a.postable && a.classification == AccountClass::Equity)
        .unwrap()
        .id
        .clone();
    drop(store);

    let added = call(
        d.path(),
        "period_add",
        json!({
            "company_id": c,
            "name": "1406",
            "start_date": "2027-03-21",
            "end_date": "2028-03-20",
            "expected_generation": generation,
        }),
    );
    let p2 = added["period_id"].as_str().unwrap().to_string();

    let mut cmd = json!({
        "company_id": c,
        "period_id": p2,
        "source_id": "y2-rev",
        "request_key": "y2-rev",
        "date": "2027-04-01",
        "reason": "Second year revenue",
        "reference": "Y2-1",
        "lines": [
            {"account_id": expense_account, "debit": "500", "credit": "0"},
            {"account_id": equity_account, "debit": "0", "credit": "500"},
        ],
    });
    let preview = call(d.path(), "journal_preview", cmd.clone());
    cmd["expected_generation"] = preview["_generation"].clone();
    let posted = call(d.path(), "journal_post", cmd);
    assert!(posted.as_str().is_some());

    let year2_report = call(
        d.path(),
        "report",
        json!({"company_id": c, "from": "2027-03-21", "to": "2028-03-20"}),
    );
    assert_eq!(year2_report["profit_loss"]["expenses"], "500");
    assert_eq!(year2_report["profit_loss"]["profit"], "-500");

    let year1_after = call(
        d.path(),
        "report",
        json!({"company_id": c, "from": "2026-03-21", "to": "2027-03-20"}),
    );
    assert_eq!(year1_after["profit_loss"]["expenses"], year1_before["profit_loss"]["expenses"]);
    assert_eq!(year1_after["profit_loss"]["profit"], year1_before["profit_loss"]["profit"]);

    let y2_cash: i64 = year2_report["position"]["cash"].as_str().unwrap().parse().unwrap();
    let y2_assets: i64 = year2_report["balance_sheet"]["total_assets"].as_str().unwrap().parse().unwrap();
    assert_eq!(y2_cash, y1_cash);
    assert_eq!(y2_assets, y1_assets);
    assert_eq!(year2_report["balance_sheet"]["balanced"], true);

    let reopened = Store::open(d.path()).unwrap();
    reopened.state.validate().unwrap();
}

#[test]
fn period_add_happy_path_and_validations() {
    let (d, c, _p1) = fixture();
    let initial_gen = Store::open(d.path()).unwrap().local_generation();

    // Gap rejected
    let gap_err = dispatch(
        d.path(),
        "period_add",
        json!({
            "company_id": c,
            "name": "Gap year",
            "start_date": "2027-03-22",
            "end_date": "2028-03-20",
            "expected_generation": initial_gen.to_string(),
        }),
    )
    .unwrap_err();
    assert!(
        gap_err
            .to_string()
            .contains("The new fiscal year must start the day after the previous one ends."),
        "unexpected gap error: {gap_err}"
    );
    assert_eq!(Store::open(d.path()).unwrap().local_generation(), initial_gen);

    // Overlap rejected
    let overlap_err = dispatch(
        d.path(),
        "period_add",
        json!({
            "company_id": c,
            "name": "Overlap year",
            "start_date": "2027-01-01",
            "end_date": "2027-12-31",
            "expected_generation": initial_gen.to_string(),
        }),
    )
    .unwrap_err();
    assert_eq!(Store::open(d.path()).unwrap().local_generation(), initial_gen);
    let _ = overlap_err;

    // Stale expected_generation rejected
    let stale_gen = if initial_gen == 0 { 999 } else { initial_gen - 1 };
    let stale_err = dispatch(
        d.path(),
        "period_add",
        json!({
            "company_id": c,
            "name": "1406",
            "start_date": "2027-03-21",
            "end_date": "2028-03-20",
            "expected_generation": stale_gen.to_string(),
        }),
    )
    .unwrap_err();
    assert!(
        stale_err.to_string().contains("Books changed since review"),
        "unexpected stale error: {stale_err}"
    );
    assert_eq!(Store::open(d.path()).unwrap().local_generation(), initial_gen);

    // Happy path
    let res = call(
        d.path(),
        "period_add",
        json!({
            "company_id": c,
            "name": "1406",
            "start_date": "2027-03-21",
            "end_date": "2028-03-20",
            "expected_generation": initial_gen.to_string(),
        }),
    );
    assert!(res["period_id"].as_str().is_some());

    let w = call(d.path(), "workspace", json!({"company_id": c}));
    assert_eq!(w["periods"].as_array().unwrap().len(), 2);

    // Reusing the old generation after a write is also rejected
    let after_gen = Store::open(d.path()).unwrap().local_generation();
    assert!(after_gen > initial_gen);
    assert!(
        dispatch(
            d.path(),
            "period_add",
            json!({
                "company_id": c,
                "name": "1407",
                "start_date": "2028-03-21",
                "end_date": "2029-03-20",
                "expected_generation": initial_gen.to_string(),
            }),
        )
        .is_err()
    );
    assert_eq!(Store::open(d.path()).unwrap().local_generation(), after_gen);
}
