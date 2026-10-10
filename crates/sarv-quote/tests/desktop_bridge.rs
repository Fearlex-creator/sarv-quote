use sarv_quote::{
    desktop::{dispatch, seed_fixture, wire},
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
fn ctx(c: &str, p: &str, key: &str) -> Value {
    json!({"company_id":c,"period_id":p,"source_id":key,"request_key":key,"date":"2026-10-08","reason":"Independent desktop bridge test","reference":key,"due_date":"2026-10-30"})
}
#[test]
fn wire_preserves_i64_extremes_and_nested_money_exactly() {
    let v = wire(
        json!({"max":i64::MAX,"min":i64::MIN,"lines":[{"debit":9007199254740993_i64}],"bool":true}),
    );
    assert_eq!(v["max"], "9223372036854775807");
    assert_eq!(v["min"], "-9223372036854775808");
    assert_eq!(v["lines"][0]["debit"], "9007199254740993");
    assert_eq!(v["bool"], true);
}
#[test]
fn settings_round_trip_preserves_upstream_fiscal_month_and_books() {
    let (d, c, _) = fixture();
    let mut store = Store::open(d.path()).unwrap();
    store.change(|s| {
        s.settings.fiscal_year_start_month = 9;
        Ok(())
    }).unwrap();
    let books = serde_json::to_value(&store.state.accounting).unwrap();
    drop(store);
    let workspace = call(d.path(), "workspace", json!({"company_id": c}));
    let mut settings = workspace["settings"].clone();
    assert_eq!(settings["fiscal_year_start_month"], "9");
    settings["appearance"] = json!("Light");
    call(d.path(), "settings", settings);
    let store = Store::open(d.path()).unwrap();
    assert_eq!(store.state.settings.fiscal_year_start_month, 9);
    assert_eq!(serde_json::to_value(&store.state.accounting).unwrap(), books);
}
#[test]
fn company_setup_zero_is_explicit_atomic_and_persists() {
    let d = tempfile::tempdir().unwrap();
    let r = call(
        d.path(),
        "setup_company",
        json!({"name":"Independent fresh business","fiscal_name":"2026","start_date":"2026-01-01","end_date":"2026-12-31"}),
    );
    let c = r["company_id"].as_str().unwrap();
    let mut p = json!({"company_id":c,"source_id":"fresh-open","request_key":"fresh-open","confirmed":false});
    let before = Store::open(d.path()).unwrap().local_generation();
    assert!(dispatch(d.path(), "initialize_zero", p.clone()).is_err());
    assert_eq!(before, Store::open(d.path()).unwrap().local_generation());
    p["confirmed"] = json!(true);
    call(d.path(), "initialize_zero", p);
    let w = call(d.path(), "workspace", json!({"company_id":c}));
    assert_eq!(w["initialized"], true);
    assert_eq!(w["accounts"].as_array().unwrap().len(), 26);
    assert_eq!(w["journal_count"], "0");
}
#[test]
fn purchase_receipt_bill_preserve_exact_stock_and_reconcile() {
    let (d, c, p) = fixture();
    let mut order = ctx(&c, &p, "bridge-po");
    order["supplier_id"] = json!("SU-A");
    order["lines"] = json!([{"id":"po-line","item_id":"ITEM-A","warehouse_id":"W1","unit_id":"PIECE","quantity":"3","unit_price":"11","line_discount":"0"}]);
    let preview = call(d.path(), "purchase_preview", order.clone());
    assert_eq!(preview["total"], "33");
    assert_eq!(preview["nonfinancial"], true);
    let r = call(d.path(), "purchase_post", order);
    let mut receipt = ctx(&c, &p, "bridge-receipt");
    receipt["order_id"] = r["record_id"].clone();
    assert_eq!(
        call(d.path(), "receive_preview", receipt.clone())["total"],
        "33"
    );
    let r = call(d.path(), "receive_post", receipt);
    let mut bill = ctx(&c, &p, "bridge-bill");
    bill["receipt_id"] = r["record_id"].clone();
    assert_eq!(call(d.path(), "bill_preview", bill.clone())["total"], "33");
    call(d.path(), "bill_post", bill);
    let r = call(
        d.path(),
        "report",
        json!({"company_id":c,"from":"2026-03-21","to":"2026-10-08"}),
    );
    assert_eq!(r["reconciliation"]["reconciled"], true);
    let stock = r["reconciliation"]["stock"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["warehouse_id"] == "W1")
        .unwrap();
    assert_eq!(stock["quantity_thousandths"], "27000");
    assert_eq!(stock["value"], "273");
    assert_eq!(r["position"]["payables"], "182");
}
#[test]
fn sales_draft_preview_is_readonly_then_order_delivery_invoice_reconcile() {
    let (d, c, p) = fixture();
    let s = Store::open(d.path()).unwrap();
    let setup = s.state.accounting.core.setups[0].id.clone();
    let draft = json!({"id":"bridge-draft","revision":"0","company_id":c,"period_id":p,"setup_id":setup,"customer_id":"CU-A","reference":"bridge-sales","document_date":"2026-10-08","financial_date":"2026-10-08","date_display":"Iso","display_unit":"Rial","document_discount":"1","supplied_total":"0","reason":"Desktop test sale","quotation":null,"lines":[{"id":"bridge-sale-line","item_id":"ITEM-A","warehouse_id":"W1","unit_id":"PIECE","quantity":"2","unit_price":"20","line_discount":"0","price_reference":"Manual","price_date":"2026-10-08","override_reason":""}]});
    let before = Store::open(d.path()).unwrap().local_generation();
    let preview = call(d.path(), "draft_preview", json!({"draft":draft}));
    assert_eq!(preview["supplied_total"], "39");
    assert_eq!(before, Store::open(d.path()).unwrap().local_generation());
    let mut draft = draft;
    draft["supplied_total"] = preview["supplied_total"].clone();
    assert_eq!(
        call(
            d.path(),
            "draft_save",
            json!({"draft":draft,"expected_revision":null})
        ),
        "1"
    );
    let order = call(
        d.path(),
        "draft_post",
        json!({"id":"bridge-draft","revision":"1","request_key":"bridge-post"}),
    );
    let mut fulfill = ctx(&c, &p, "bridge-invoice");
    fulfill["order_id"] = order["record_id"].clone();
    assert_eq!(
        call(d.path(), "fulfill_preview", fulfill.clone())["total"],
        "39"
    );
    let invoice = call(d.path(), "fulfill_post", fulfill.clone());
    assert_eq!(call(d.path(), "fulfill_post", fulfill), invoice);
    assert!(invoice["journal_id"].as_str().is_some());
    let r = call(
        d.path(),
        "report",
        json!({"company_id":c,"from":"2026-10-01","to":"2026-10-08"}),
    );
    assert_eq!(r["profit_loss"]["revenue"], "39");
    assert_eq!(r["profit_loss"]["expenses"], "20");
    assert_eq!(r["profit_loss"]["profit"], "19");
    assert_eq!(r["position"]["receivables"], "239");
    // CU-A's opening is 125; the other customer's 75 is not attributed here.
    assert_eq!(r["party_balances"]["customer:CU-A"], "164");
    assert_eq!(r["reconciliation"]["reconciled"], true);
}
#[test]
fn journal_rejects_unbalanced_duplicate_payload_and_cross_company_without_writes() {
    let (d, c, p) = fixture();
    let st = Store::open(d.path()).unwrap();
    let accounts = &st.state.accounting.accounts;
    let exp = accounts
        .iter()
        .find(|a| {
            !a.control
                && a.postable
                && a.classification == sarv_quote::accounting::AccountClass::Expense
        })
        .unwrap()
        .id
        .clone();
    let equity = accounts
        .iter()
        .find(|a| {
            !a.control
                && a.postable
                && a.classification == sarv_quote::accounting::AccountClass::Equity
        })
        .unwrap()
        .id
        .clone();
    let mut cmd = ctx(&c, &p, "bridge-journal");
    cmd["lines"] = json!([{"account_id":exp,"debit":"10","credit":"0"},{"account_id":equity,"debit":"0","credit":"9"}]);
    let before = st.local_generation();
    assert!(dispatch(d.path(), "journal_post", cmd.clone()).is_err());
    assert_eq!(before, Store::open(d.path()).unwrap().local_generation());
    cmd["lines"][1]["credit"] = json!("10");
    let preview = call(d.path(), "journal_preview", cmd.clone());
    assert_eq!(preview["total_debit"], "10");
    assert_eq!(preview["total_credit"], "10");
    let result = call(d.path(), "journal_post", cmd.clone());
    assert_eq!(call(d.path(), "journal_post", cmd.clone()), result);
    assert_eq!(
        Store::open(d.path())
            .unwrap()
            .state
            .accounting
            .journals
            .len(),
        2
    );
    cmd["lines"][0]["debit"] = json!("11");
    cmd["lines"][1]["credit"] = json!("11");
    assert!(dispatch(d.path(), "journal_post", cmd).is_err());
    let c2 = call(
        d.path(),
        "setup_company",
        json!({"name":"Second isolated company","fiscal_name":"2026","start_date":"2026-01-01","end_date":"2026-12-31"}),
    );
    assert!(
        dispatch(
            d.path(),
            "detail",
            json!({"company_id":c2["company_id"],"id":result})
        )
        .is_err()
    );
    let w = call(
        d.path(),
        "workspace",
        json!({"company_id":c2["company_id"]}),
    );
    assert_eq!(w["journals"].as_array().unwrap().len(), 0);
    assert_eq!(w["sales"].as_array().unwrap().len(), 0);
}
#[test]
fn report_range_filter_and_formula_safe_export_are_authoritative() {
    let (d, c, _) = fixture();
    assert!(
        dispatch(
            d.path(),
            "report",
            json!({"company_id":c,"from":"2026-10-09","to":"2026-10-08"})
        )
        .is_err()
    );
    let r = call(
        d.path(),
        "report",
        json!({"company_id":c,"from":"2026-10-01","to":"2026-10-08"}),
    );
    assert_eq!(r["profit_loss"]["profit"], "0");
    assert_eq!(r["trial"]["total_debit"], "1251");
    let export = call(
        d.path(),
        "report_csv",
        json!({"company_id":c,"from":"2026-10-01","to":"2026-10-08"}),
    );
    assert!(export["content"].as_str().unwrap().starts_with('\u{feff}'));
    assert!(
        export["content"]
            .as_str()
            .unwrap()
            .contains("Debit IRR,Credit IRR")
    );
}

#[test]
fn changed_books_reject_stale_financial_review_without_a_write() {
    let (d, c, p) = fixture();
    let mut order = ctx(&c, &p, "review-po");
    order["supplier_id"] = json!("SU-A");
    order["lines"] = json!([{"id":"review-line","item_id":"ITEM-A","warehouse_id":"W1","unit_id":"PIECE","quantity":"1","unit_price":"7"}]);
    let preview = call(d.path(), "purchase_preview", order.clone());
    order["expected_generation"] = preview["_generation"].clone();
    let mut resource = ctx(&c, &p, "resource-change");
    resource["kind"] = json!("unit");
    resource["id"] = json!("unit-change");
    resource["name"] = json!("Independent unit");
    call(d.path(), "resource", resource);
    let before = Store::open(d.path()).unwrap().local_generation();
    assert!(
        dispatch(d.path(), "purchase_post", order)
            .unwrap_err()
            .to_string()
            .contains("changed since review")
    );
    assert_eq!(before, Store::open(d.path()).unwrap().local_generation());
}

#[test]
fn partial_receipts_billing_and_both_supplier_returns_conserve_original_costs() {
    let (d, c, p) = fixture();
    let mut order = ctx(&c, &p, "partial-po");
    order["supplier_id"] = json!("SU-A");
    order["discount"] = json!("2");
    order["lines"] = json!([{"id":"partial-po-line","item_id":"ITEM-A","warehouse_id":"W1","unit_id":"PIECE","quantity":"3","unit_price":"4","line_discount":"0"}]);
    assert_eq!(
        call(d.path(), "purchase_preview", order.clone())["total"],
        "10"
    );
    let order = call(d.path(), "purchase_post", order);
    let options = call(
        d.path(),
        "operation_options",
        json!({"company_id":c,"record_id":order["record_id"],"operation":"receive"}),
    );
    assert_eq!(options["lines"][0]["remaining_quantity"], "3000");
    let mut first = ctx(&c, &p, "partial-receipt-1");
    first["order_id"] = order["record_id"].clone();
    first["lines"] = json!([{"id":"partial-po-line","quantity":"1"}]);
    assert_eq!(
        call(d.path(), "receive_preview", first.clone())["total"],
        "3"
    );
    let result = call(d.path(), "receive_post", first.clone());
    assert_eq!(call(d.path(), "receive_post", first.clone()), result);
    let before = Store::open(d.path()).unwrap().local_generation();
    first["lines"][0]["quantity"] = json!("2");
    assert!(dispatch(d.path(), "receive_post", first).is_err());
    assert_eq!(before, Store::open(d.path()).unwrap().local_generation());
    let mut second = ctx(&c, &p, "partial-receipt-2");
    second["order_id"] = order["record_id"].clone();
    assert_eq!(
        call(d.path(), "receive_preview", second.clone())["total"],
        "7"
    );
    let second = call(d.path(), "receive_post", second);
    let options = call(
        d.path(),
        "operation_options",
        json!({"company_id":c,"record_id":second["record_id"],"operation":"bill"}),
    );
    let line = options["lines"][0]["id"].clone();
    let mut bill = ctx(&c, &p, "partial-bill");
    bill["receipt_id"] = second["record_id"].clone();
    bill["lines"] = json!([{"id":line,"quantity":"1"}]);
    assert_eq!(call(d.path(), "bill_preview", bill.clone())["total"], "4");
    let bill_result = call(d.path(), "bill_post", bill.clone());
    assert_eq!(call(d.path(), "bill_post", bill.clone()), bill_result);
    bill["lines"][0]["quantity"] = json!("2");
    assert!(dispatch(d.path(), "bill_post", bill).is_err());
    let mut ret = ctx(&c, &p, "partial-unbilled-return");
    ret["record_id"] = second["record_id"].clone();
    assert_eq!(
        call(d.path(), "purchase_return_preview", ret.clone())["total"],
        "3"
    );
    let result = call(d.path(), "purchase_return_post", ret.clone());
    assert_eq!(call(d.path(), "purchase_return_post", ret), result);
    let mut ret = ctx(&c, &p, "partial-billed-return");
    ret["record_id"] = bill_result["record_id"].clone();
    assert_eq!(
        call(d.path(), "purchase_return_preview", ret.clone())["total"],
        "4"
    );
    let result = call(d.path(), "purchase_return_post", ret.clone());
    assert_eq!(call(d.path(), "purchase_return_post", ret), result);
    let report = call(
        d.path(),
        "report",
        json!({"company_id":c,"from":"2026-03-21","to":"2026-10-08"}),
    );
    assert_eq!(report["position"]["payables"], "149");
    assert_eq!(report["reconciliation"]["reconciled"], true);
    let stock = report["reconciliation"]["stock"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["warehouse_id"] == "W1")
        .unwrap();
    assert_eq!(stock["quantity_thousandths"], "25000");
    assert_eq!(stock["value"], "243");
    // Only the unbilled first receipt remains; returns never reopen quantities.
    assert_eq!(
        call(
            d.path(),
            "operation_options",
            json!({"company_id":c,"record_id":second["record_id"],"operation":"bill"})
        )["lines"],
        json!([])
    );
}

#[test]
fn partial_sales_and_original_cost_credits_preserve_rial_remainders() {
    let (d, c, p) = fixture();
    let setup = Store::open(d.path()).unwrap().state.accounting.core.setups[0]
        .id
        .clone();
    let draft = json!({"id":"partial-draft","revision":"0","company_id":c,"period_id":p,"setup_id":setup,"customer_id":"CU-A","reference":"partial-sale","document_date":"2026-10-08","financial_date":"2026-10-08","date_display":"Iso","display_unit":"Rial","document_discount":"1","supplied_total":"39","reason":"Partial delivery test","quotation":null,"lines":[{"id":"partial-sale-line","item_id":"ITEM-A","warehouse_id":"W1","unit_id":"PIECE","quantity":"2","unit_price":"20","line_discount":"0","price_reference":"Manual","price_date":"2026-10-08","override_reason":""}]});
    call(
        d.path(),
        "draft_save",
        json!({"draft":draft,"expected_revision":null}),
    );
    let order = call(
        d.path(),
        "draft_post",
        json!({"id":"partial-draft","revision":"1","request_key":"partial-order-post"}),
    );
    let mut invoice = ctx(&c, &p, "partial-invoice-1");
    invoice["order_id"] = order["record_id"].clone();
    invoice["lines"] = json!([{"id":"partial-sale-line","quantity":"1"}]);
    assert_eq!(
        call(d.path(), "fulfill_preview", invoice.clone())["total"],
        "20"
    );
    let first = call(d.path(), "fulfill_post", invoice.clone());
    assert_eq!(call(d.path(), "fulfill_post", invoice.clone()), first);
    invoice["lines"][0]["quantity"] = json!("2");
    assert!(dispatch(d.path(), "fulfill_post", invoice).is_err());
    let mut invoice = ctx(&c, &p, "partial-invoice-2");
    invoice["order_id"] = order["record_id"].clone();
    assert_eq!(
        call(d.path(), "fulfill_preview", invoice.clone())["total"],
        "19"
    );
    call(d.path(), "fulfill_post", invoice);
    let mut ret = ctx(&c, &p, "partial-sale-return");
    ret["record_id"] = first["record_id"].clone();
    let options = call(
        d.path(),
        "operation_options",
        json!({"company_id":c,"record_id":first["record_id"],"operation":"sale_return"}),
    );
    ret["lines"] = json!([{"id":options["lines"][0]["id"],"quantity":"1"}]);
    let preview = call(d.path(), "sale_return_preview", ret.clone());
    assert_eq!(preview["total"], "20");
    assert_eq!(preview["cost"], "10");
    let result = call(d.path(), "sale_return_post", ret.clone());
    assert_eq!(call(d.path(), "sale_return_post", ret.clone()), result);
    let before = Store::open(d.path()).unwrap().local_generation();
    ret["lines"][0]["quantity"] = json!("0.5");
    assert!(dispatch(d.path(), "sale_return_post", ret).is_err());
    assert_eq!(before, Store::open(d.path()).unwrap().local_generation());
    let report = call(
        d.path(),
        "report",
        json!({"company_id":c,"from":"2026-03-21","to":"2026-10-08"}),
    );
    assert_eq!(report["position"]["receivables"], "219");
    assert_eq!(report["profit_loss"]["profit"], "9");
    assert_eq!(report["reconciliation"]["reconciled"], true);
}

#[test]
fn transfers_are_readonly_on_preview_and_preserve_stock_value_on_post_and_retry() {
    let (d, c, p) = fixture();
    let mut transfer = ctx(&c, &p, "bridge-transfer");
    transfer["item_id"] = json!("ITEM-A");
    transfer["source_warehouse_id"] = json!("W1");
    transfer["target_warehouse_id"] = json!("W2");
    transfer["unit_id"] = json!("PIECE");
    transfer["quantity"] = json!("5");
    let before = Store::open(d.path()).unwrap().local_generation();
    let preview = call(d.path(), "transfer_preview", transfer.clone());
    assert_eq!(preview["total"], "50");
    assert_eq!(preview["nonfinancial"], true);
    assert_eq!(before, Store::open(d.path()).unwrap().local_generation());
    let result = call(d.path(), "transfer_post", transfer.clone());
    assert_eq!(call(d.path(), "transfer_post", transfer.clone()), result);
    transfer["quantity"] = json!("6");
    assert!(dispatch(d.path(), "transfer_post", transfer).is_err());
    let report = call(
        d.path(),
        "report",
        json!({"company_id":c,"from":"2026-03-21","to":"2026-10-08"}),
    );
    assert_eq!(report["reconciliation"]["reconciled"], true);
    let rows = report["reconciliation"]["stock"].as_array().unwrap();
    assert_eq!(
        rows.iter().find(|x| x["warehouse_id"] == "W1").unwrap()["value"],
        "190"
    );
    assert_eq!(
        rows.iter().find(|x| x["warehouse_id"] == "W2").unwrap()["value"],
        "110"
    );
}

#[test]
fn quantity_selection_rejects_unknown_zero_fractional_and_foreign_sources_without_writes() {
    let (d, c, p) = fixture();
    let mut order = ctx(&c, &p, "selection-guards-order");
    order["supplier_id"] = json!("SU-A");
    order["lines"] = json!([{"id":"selection-guards-line","item_id":"ITEM-A","warehouse_id":"W1","unit_id":"PIECE","quantity":"3","unit_price":"4","line_discount":"0"}]);
    let order = call(d.path(), "purchase_post", order);
    let other = call(
        d.path(),
        "setup_company",
        json!({"name":"Isolated selection guard company","fiscal_name":"2026","start_date":"2026-01-01","end_date":"2026-12-31"}),
    );
    let before = Store::open(d.path()).unwrap().local_generation();
    assert!(dispatch(d.path(),"operation_options",json!({"company_id":other["company_id"],"record_id":order["record_id"],"operation":"receive"})).is_err());
    let mut receipt = ctx(&c, &p, "selection-guards-receipt");
    receipt["order_id"] = order["record_id"].clone();
    for rows in [
        json!([{"id":"unknown-line","quantity":"1"}]),
        json!([{"id":"selection-guards-line","quantity":"0"}]),
        json!([{"id":"selection-guards-line","quantity":"0.5"}]),
        json!([{"id":"selection-guards-line","quantity":"1"},{"id":"selection-guards-line","quantity":"1"}]),
        json!([{"id":"selection-guards-line","quantity":"4"}]),
    ] {
        receipt["lines"] = rows;
        assert!(dispatch(d.path(), "receive_preview", receipt.clone()).is_err());
        assert!(dispatch(d.path(), "receive_post", receipt.clone()).is_err());
        assert_eq!(before, Store::open(d.path()).unwrap().local_generation());
    }
    // Malformed return requests must report validation errors rather than panic.
    assert!(
        dispatch(
            d.path(),
            "purchase_return_post",
            json!({"company_id":c,"period_id":p})
        )
        .is_err()
    );
    assert_eq!(before, Store::open(d.path()).unwrap().local_generation());
}
