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
    store
        .change(|s| {
            s.settings.fiscal_year_start_month = 9;
            Ok(())
        })
        .unwrap();
    let books = serde_json::to_value(&store.state.accounting).unwrap();
    drop(store);
    let workspace = call(d.path(), "workspace", json!({"company_id": c}));
    let mut settings = workspace["settings"].clone();
    assert_eq!(settings["fiscal_year_start_month"], "9");
    settings["appearance"] = json!("Light");
    call(d.path(), "settings", settings);
    let store = Store::open(d.path()).unwrap();
    assert_eq!(store.state.settings.fiscal_year_start_month, 9);
    assert_eq!(
        serde_json::to_value(&store.state.accounting).unwrap(),
        books
    );
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
fn manual_shop_catalog_generates_codes_and_first_unit_atomically() {
    let d = tempfile::tempdir().unwrap();
    let company = call(
        d.path(),
        "setup_company",
        json!({"name":"Small shop","fiscal_name":"2026","start_date":"2026-01-01","end_date":"2026-12-31"}),
    );
    let c = company["company_id"].as_str().unwrap();
    call(
        d.path(),
        "initialize_zero",
        json!({"company_id":c,"source_id":"start","request_key":"start","confirmed":true}),
    );
    let w = call(d.path(), "workspace", json!({"company_id":c}));
    let period = w["periods"][0]["id"].as_str().unwrap();
    for (id, kind, name) in [
        ("manual-customer", "customer", "Walk-in customer"),
        ("manual-product", "item", "Notebook"),
    ] {
        let mut payload = ctx(c, period, id);
        payload["id"] = json!(id);
        payload["kind"] = json!(kind);
        payload["name"] = json!(name);
        payload["default_unit_name"] = json!("عدد");
        call(d.path(), "resource", payload.clone());
        call(d.path(), "resource", payload);
    }
    let w = call(d.path(), "workspace", json!({"company_id":c}));
    assert_eq!(w["resources"]["parties"].as_array().unwrap().len(), 1);
    assert_eq!(w["resources"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(w["resources"]["units"].as_array().unwrap().len(), 1);
    assert_eq!(w["resources"]["units"][0]["name"], "عدد");
    assert_eq!(w["resources"]["items"][0]["sku"], "P-manual-product");
    assert_eq!(
        w["resources"]["items"][0]["base_unit_id"],
        w["resources"]["units"][0]["id"]
    );
    let before = Store::open(d.path()).unwrap().local_generation();
    let mut invalid = ctx(c, period, "invalid-product");
    invalid["id"] = json!("invalid-product");
    invalid["kind"] = json!("item");
    invalid["name"] = json!("Invalid");
    invalid["unit_id"] = json!("foreign-unit");
    assert!(dispatch(d.path(), "resource", invalid).is_err());
    assert_eq!(before, Store::open(d.path()).unwrap().local_generation());
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
    let product = call(
        d.path(),
        "product_detail",
        json!({"company_id":c,"item_id":"ITEM-A","as_of":"2026-10-08"}),
    );
    assert_eq!(product["stats"]["purchased_quantity"], "3000");
    assert_eq!(product["stats"]["net_purchase_value"], "33");
    assert_eq!(product["stats"]["last_purchase_price"], "11");
    assert_eq!(product["record_count"], "3");
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
    let contact = call(
        d.path(),
        "contact_detail",
        json!({"company_id":c,"party_id":"CU-A","as_of":"2026-10-08"}),
    );
    assert_eq!(contact["stats"]["net_sales"], "19");
    assert_eq!(contact["stats"]["gross_profit"], "9");
    assert_eq!(contact["stats"]["outstanding"], "144");
    assert_eq!(contact["stats"]["invoices"], "2");
    assert_eq!(contact["stats"]["returns"], "1");
    assert_eq!(contact["record_count"], "4");
    assert_eq!(contact["monthly"][0]["net"], "19");
    let product = call(
        d.path(),
        "product_detail",
        json!({"company_id":c,"item_id":"ITEM-A","as_of":"2026-10-08"}),
    );
    assert_eq!(product["stats"]["net_sales"], "19");
    assert_eq!(product["stats"]["cost_of_goods_sold"], "10");
    assert_eq!(product["stats"]["gross_profit"], "9");
    assert_eq!(product["stats"]["sold_quantity"], "1000");
    assert_eq!(product["stats"]["returned_quantity"], "1000");
    assert_eq!(product["stats"]["stock_quantity"], "29000");
    assert_eq!(product["stats"]["stock_value"], "290");
    assert_eq!(product["stats"]["average_stock_cost"], "10");
    assert_eq!(product["monthly"][0]["profit"], "9");

    let other_contact = call(
        d.path(),
        "contact_detail",
        json!({"company_id":c,"party_id":"CU-B","as_of":"2026-10-08"}),
    );
    assert_eq!(other_contact["stats"]["net_sales"], "0");
    assert_eq!(other_contact["stats"]["outstanding"], "75");

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

#[test]
fn business_profiles_are_company_scoped_and_survive_restore() {
    let d = tempfile::tempdir().unwrap();
    let first = call(
        d.path(),
        "setup_company",
        json!({"name":"First business","fiscal_name":"1405","start_date":"2026-03-21","end_date":"2027-03-20","locale":"fa","business_profile":{"phone":"021-12345678","registration_id":"REG-A","address":"تهران","invoice_prefix":"INV-A","terms":"Payment on delivery"}}),
    );
    let second = call(
        d.path(),
        "setup_company",
        json!({"name":"Second business","fiscal_name":"2026","start_date":"2026-01-01","end_date":"2026-12-31","business_profile":{"phone":"021-87654321","registration_id":"REG-B"}}),
    );
    let a = call(
        d.path(),
        "workspace",
        json!({"company_id":first["company_id"]}),
    );
    let b = call(
        d.path(),
        "workspace",
        json!({"company_id":second["company_id"]}),
    );
    assert_eq!(a["business_profile"]["registration_id"], "REG-A");
    assert_eq!(b["business_profile"]["registration_id"], "REG-B");
    assert_eq!(b["settings"]["date_display"], "Jalali");
    call(
        d.path(),
        "business_profile",
        json!({"company_id":first["company_id"],"profile":{"phone":"021-00000000","tax_id":"TAX-A"}}),
    );
    let store = Store::open(d.path()).unwrap();
    let backup = d.path().join("profiles.sarv");
    sarv_quote::store::backup_state(&store.state, &backup).unwrap();
    let restored = Store::restore_preview(&backup).unwrap();
    assert_eq!(
        restored.business_profiles[first["company_id"].as_str().unwrap()].tax_id,
        "TAX-A"
    );
    assert_eq!(
        restored.business_profiles[second["company_id"].as_str().unwrap()].registration_id,
        "REG-B"
    );
    assert!(
        dispatch(
            d.path(),
            "business_profile",
            json!({"company_id":"foreign","profile":{}})
        )
        .is_err()
    );
    let before = fs_state(d.path());
    assert!(
        dispatch(
            d.path(),
            "business_profile",
            json!({"company_id":first["company_id"],"profile":{"registration_id":"bad\nID"}})
        )
        .is_err()
    );
    assert_eq!(before, fs_state(d.path()));
}
fn fs_state(directory: &std::path::Path) -> serde_json::Value {
    serde_json::to_value(Store::open(directory).unwrap().state).unwrap()
}

#[test]
fn rich_contacts_are_atomic_scoped_editable_and_restore_with_books() {
    let (d, c, p) = fixture();
    let mut resource = ctx(&c, &p, "rich-contact");
    resource["kind"] = json!("customer");
    resource["id"] = json!("rich-contact");
    resource["name"] = json!("Sara Ahmadi");
    resource["profile"] = json!({"first_name":"Sara","last_name":"Ahmadi","phone":"02112345678","mobile":"09121234567","email":"sara@example.test","address":"Street 12","city":"Tehran","postal_code":"1234567890","national_id":"ID-1","tax_id":"TAX-1","organization":"Corner shop","website":"https://example.test","notes":"Preferred delivery: mornings"});
    let mut invalid = resource.clone();
    invalid["profile"]["email"] = json!("invalid");
    let before = fs_state(d.path());
    assert!(dispatch(d.path(), "resource", invalid).is_err());
    assert_eq!(before, fs_state(d.path()));
    call(d.path(), "resource", resource.clone());
    let detail = call(
        d.path(),
        "contact_detail",
        json!({"company_id":c,"party_id":"rich-contact","as_of":"2026-10-08"}),
    );
    assert_eq!(detail["profile"]["address"], "Street 12");
    assert_eq!(detail["profile"]["last_name"], "Ahmadi");
    let mut profile = detail["profile"].clone();
    profile["display_name"] = json!("Sara’s shop");
    profile["mobile"] = json!("09120000000");
    call(
        d.path(),
        "party_profile",
        json!({"company_id":c,"party_id":"rich-contact","profile":profile,"expected_generation":detail["_generation"]}),
    );
    assert!(dispatch(d.path(),"party_profile",json!({"company_id":c,"party_id":"rich-contact","profile":profile,"expected_generation":detail["_generation"]})).is_err());
    // Retrying original creation must not overwrite later edits.
    call(d.path(), "resource", resource);
    let w = call(d.path(), "workspace", json!({"company_id":c}));
    assert_eq!(w["party_profiles"]["rich-contact"]["mobile"], "09120000000");
    assert!(
        w["resources"]["parties"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "Sara’s shop")
    );
    let other = call(
        d.path(),
        "setup_company",
        json!({"name":"Other","fiscal_name":"2026","start_date":"2026-01-01","end_date":"2026-12-31"}),
    );
    assert!(
        dispatch(
            d.path(),
            "contact_detail",
            json!({"company_id":other["company_id"],"party_id":"rich-contact","as_of":"2026-10-08"})
        )
        .is_err()
    );
    let store = Store::open(d.path()).unwrap();
    let backup = d.path().join("contacts.sarv");
    sarv_quote::store::backup_state(&store.state, &backup).unwrap();
    let restored = Store::restore_preview(&backup).unwrap();
    assert_eq!(
        restored.party_profiles["rich-contact"].mobile,
        "09120000000"
    );
}

#[test]
fn banking_preview_post_transfer_expense_reverse_and_restore_conserve_funds() {
    use sarv_quote::accounting::AccountClass;
    let (d, c, p) = fixture();
    let store = Store::open(d.path()).unwrap();
    let data = call(
        d.path(),
        "banking",
        json!({"company_id":c,"from":"2026-10-01","to":"2026-10-08"}),
    );
    let bank = data["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["role"] == "Bank")
        .unwrap();
    let bank_id = bank["account"]["id"].clone();
    let bank_open: i64 = bank["balance"].as_str().unwrap().parse().unwrap();
    let cash = data["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["role"] == "Cash")
        .unwrap();
    let cash_id = cash["account"]["id"].clone();
    let cash_open: i64 = cash["balance"].as_str().unwrap().parse().unwrap();
    let equity = store
        .state
        .accounting
        .accounts
        .iter()
        .find(|a| {
            a.company_id == c
                && !a.control
                && a.postable
                && a.classification == AccountClass::Equity
        })
        .unwrap()
        .id
        .clone();
    let expense = Store::open(d.path())
        .unwrap()
        .change(|s| {
            s.accounting.add_account(
                sarv_quote::accounting::NewAccount {
                    company_id: c.clone(),
                    parent_id: None,
                    code: "EX-BANK-TEST".into(),
                    name: "Independent bank expense".into(),
                    classification: AccountClass::Expense,
                    normal_balance: sarv_quote::accounting::BalanceSide::Debit,
                    postable: true,
                    control: false,
                },
                "bank-test",
            )
        })
        .unwrap();
    let mut deposit = ctx(&c, &p, "bank-funding");
    deposit["kind"] = json!("deposit");
    deposit["account_id"] = bank_id.clone();
    deposit["counterpart_id"] = json!(equity);
    deposit["amount"] = json!("1000");
    let before = fs_state(d.path());
    let preview = call(d.path(), "banking_preview", deposit.clone());
    assert_eq!(before, fs_state(d.path()));
    deposit["expected_generation"] = preview["_generation"].clone();
    let id = call(d.path(), "banking_post", deposit.clone());
    assert_eq!(id, call(d.path(), "banking_post", deposit.clone()));
    let mut transfer = ctx(&c, &p, "bank-cash-transfer");
    transfer["kind"] = json!("transfer");
    transfer["account_id"] = bank_id.clone();
    transfer["counterpart_id"] = cash_id.clone();
    transfer["amount"] = json!("300");
    call(d.path(), "banking_post", transfer);
    let mut withdrawal = ctx(&c, &p, "cash-expense");
    withdrawal["kind"] = json!("withdrawal");
    withdrawal["account_id"] = cash_id;
    withdrawal["counterpart_id"] = json!(expense);
    withdrawal["amount"] = json!("100");
    let expense_id = call(d.path(), "banking_post", withdrawal.clone());
    let after = call(
        d.path(),
        "banking",
        json!({"company_id":c,"from":"2026-10-01","to":"2026-10-08"}),
    );
    for a in after["accounts"].as_array().unwrap() {
        assert_eq!(
            a["balance"],
            json!(
                (if a["role"] == "Cash" {
                    cash_open + 200
                } else {
                    bank_open + 700
                })
                .to_string()
            )
        );
    }
    let mut reversal = ctx(&c, &p, "undo-bank-expense");
    reversal["journal_id"] = expense_id;
    call(d.path(), "journal_reverse", reversal.clone());
    assert_eq!(
        call(d.path(), "journal_reverse", reversal.clone()),
        call(d.path(), "journal_reverse", reversal)
    );
    let mut alias = withdrawal.clone();
    alias["request_key"] = json!("expense-alias-before-archive");
    call(d.path(), "banking_post", alias);
    Store::open(d.path())
        .unwrap()
        .change(|s| {
            s.accounting
                .archive_account(&expense, "bank-test", "Retire expense category")
        })
        .unwrap();
    let mut alias = withdrawal;
    alias["request_key"] = json!("expense-alias-after-archive");
    call(d.path(), "banking_post", alias);
    let store = Store::open(d.path()).unwrap();
    assert_eq!(store.state.accounting.banking.len(), 4);
    store.state.validate().unwrap();
    let report = call(
        d.path(),
        "report",
        json!({"company_id":c,"from":"2026-03-21","to":"2026-10-08"}),
    );
    assert_eq!(report["reconciliation"]["reconciled"], true);
    let backup = d.path().join("bank.sarv");
    sarv_quote::store::backup_state(&store.state, &backup).unwrap();
    assert_eq!(
        Store::restore_preview(&backup).unwrap().accounting,
        store.state.accounting
    );
    let mut corrupt = store.state.clone();
    corrupt.accounting.banking.clear();
    assert!(corrupt.validate().is_err());
    let mut corrupt = store.state.clone();
    corrupt.accounting.banking[0].command.reason = "tampered".into();
    assert!(corrupt.validate().is_err());
    let mut changed = deposit;
    changed["amount"] = json!("1001");
    assert!(dispatch(d.path(), "banking_post", changed).is_err());
}

#[test]
fn banking_rejects_control_counterpart_stale_reviews_and_locked_periods() {
    let (d, c, p) = fixture();
    let store = Store::open(d.path()).unwrap();
    let setup = &store.state.accounting.core.setups[0];
    let bank = setup
        .command
        .mappings
        .iter()
        .find(|m| m.role == sarv_quote::core::CoreRole::Bank)
        .unwrap()
        .account_id
        .clone();
    let ar = setup
        .command
        .mappings
        .iter()
        .find(|m| m.role == sarv_quote::core::CoreRole::Receivables)
        .unwrap()
        .account_id
        .clone();
    let equity = store
        .state
        .accounting
        .accounts
        .iter()
        .find(|a| {
            !a.control
                && a.postable
                && a.classification == sarv_quote::accounting::AccountClass::Equity
        })
        .unwrap()
        .id
        .clone();
    let mut cmd = ctx(&c, &p, "bank-guards");
    cmd["kind"] = json!("deposit");
    cmd["account_id"] = json!(bank);
    cmd["counterpart_id"] = json!(ar);
    cmd["amount"] = json!("1");
    let before = fs_state(d.path());
    assert!(dispatch(d.path(), "banking_post", cmd.clone()).is_err());
    assert_eq!(before, fs_state(d.path()));
    cmd["counterpart_id"] = json!(equity);
    let preview = call(d.path(), "banking_preview", cmd.clone());
    cmd["expected_generation"] = preview["_generation"].clone();
    let mut change = ctx(&c, &p, "bank-review-change");
    change["kind"] = json!("unit");
    change["id"] = json!("bank-review-change");
    change["name"] = json!("Extra unit");
    call(d.path(), "resource", change);
    let before = fs_state(d.path());
    assert!(dispatch(d.path(), "banking_post", cmd.clone()).is_err());
    assert_eq!(before, fs_state(d.path()));
    cmd.as_object_mut().unwrap().remove("expected_generation");
    call(
        d.path(),
        "period_lock",
        json!({"company_id":c,"period_id":p,"locked":true,"reason":"Close year"}),
    );
    assert!(dispatch(d.path(), "banking_preview", cmd.clone()).is_err());
    assert!(dispatch(d.path(), "banking_post", cmd).is_err());
}

#[test]
fn product_prices_estimates_stock_thresholds_and_metadata_do_not_revalue_books() {
    let (d, c, p) = fixture();
    let detail = call(
        d.path(),
        "product_detail",
        json!({"company_id":c,"item_id":"ITEM-A","as_of":"2026-10-08"}),
    );
    assert!(detail["profile"]["purchase_price"].is_null());
    assert!(detail["stats"]["estimated_unit_profit"].is_null());
    let books = serde_json::to_value(Store::open(d.path()).unwrap().state.accounting).unwrap();
    let profile = json!({"purchase_price":"۱۲۵۰۰۰","sale_price":"200000","barcode":"EAN-123","category":"Stationery","brand":"Sarv","minimum_stock":"40","preferred_supplier_id":"SU-A","notes":"Keep dry"});
    call(
        d.path(),
        "product_profile",
        json!({"company_id":c,"item_id":"ITEM-A","profile":profile,"expected_generation":detail["_generation"]}),
    );
    let detail = call(
        d.path(),
        "product_detail",
        json!({"company_id":c,"item_id":"ITEM-A","as_of":"2026-10-08"}),
    );
    assert_eq!(detail["stats"]["estimated_unit_profit"], "75000");
    assert_eq!(detail["stats"]["margin_basis_points"], "3750");
    assert_eq!(detail["stats"]["markup_basis_points"], "6000");
    assert_eq!(detail["stats"]["low_stock"], true);
    assert_eq!(detail["stats"]["average_stock_cost"], "10");
    let store = Store::open(d.path()).unwrap();
    assert_eq!(
        serde_json::to_value(&store.state.accounting).unwrap(),
        books
    );
    let backup = d.path().join("products.sarv");
    sarv_quote::store::backup_state(&store.state, &backup).unwrap();
    let restored = Store::restore_preview(&backup).unwrap();
    assert_eq!(restored.product_profiles["ITEM-A"].barcode, "EAN-123");
    let mut loss = profile.clone();
    loss["purchase_price"] = json!("200000");
    loss["sale_price"] = json!("125000");
    loss["minimum_stock"] = json!("0.0");
    call(
        d.path(),
        "product_profile",
        json!({"company_id":c,"item_id":"ITEM-A","profile":loss}),
    );
    let detail = call(
        d.path(),
        "product_detail",
        json!({"company_id":c,"item_id":"ITEM-A","as_of":"2026-10-08"}),
    );
    assert_eq!(detail["stats"]["estimated_unit_profit"], "-75000");
    assert_eq!(detail["stats"]["margin_basis_points"], "-6000");
    assert_eq!(detail["stats"]["low_stock"], false);
    let before = fs_state(d.path());
    for field in ["purchase_price", "sale_price"] {
        let mut invalid = profile.clone();
        invalid[field] = json!("-1");
        assert!(
            dispatch(
                d.path(),
                "product_profile",
                json!({"company_id":c,"item_id":"ITEM-A","profile":invalid})
            )
            .is_err()
        );
        assert_eq!(before, fs_state(d.path()));
    }
    let mut invalid = profile.clone();
    invalid["preferred_supplier_id"] = json!("CU-A");
    assert!(
        dispatch(
            d.path(),
            "product_profile",
            json!({"company_id":c,"item_id":"ITEM-A","profile":invalid})
        )
        .is_err()
    );
    assert_eq!(before, fs_state(d.path()));
    assert!(
        dispatch(
            d.path(),
            "product_profile",
            json!({"company_id":c,"item_id":"ITEM-A","profile":profile,"expected_generation":"0"})
        )
        .is_err()
    );
    let other = call(
        d.path(),
        "setup_company",
        json!({"name":"Other product business","fiscal_name":"2026","start_date":"2026-01-01","end_date":"2026-12-31"}),
    );
    assert!(
        dispatch(
            d.path(),
            "product_detail",
            json!({"company_id":other["company_id"],"item_id":"ITEM-A","as_of":"2026-10-08"})
        )
        .is_err()
    );
    let mut resource = ctx(&c, &p, "rich-product");
    resource["kind"] = json!("item");
    resource["id"] = json!("rich-product");
    resource["name"] = json!("Notebook");
    resource["product_profile"] = profile;
    call(d.path(), "resource", resource.clone());
    let mut edited = resource["product_profile"].clone();
    edited["sale_price"] = json!("250000");
    call(
        d.path(),
        "product_profile",
        json!({"company_id":c,"item_id":"rich-product","profile":edited}),
    );
    call(d.path(), "resource", resource);
    assert_eq!(
        Store::open(d.path()).unwrap().state.product_profiles["rich-product"]
            .sale_price
            .unwrap()
            .rials(),
        250000
    );
}

#[test]
fn payment_settlement_settles_receivable_and_reverses_through_bridge() {
    let (d, c, p) = fixture();
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
    let before = Store::open(d.path()).unwrap().local_generation();
    let mut preview = call(
        d.path(),
        "payment_preview",
        json!({"company_id":c,"period_id":p,"source_id":"bridge-pay","request_key":"bridge-pay","date":"2026-10-08","reason":"Customer settlement","side":"receive","party_id":"CU-A","funds_role":"bank","reference":"RCP-1","advance_amount":"25","allocations":[{"open_item_id":"AR-125","amount":"100"}]}),
    );
    assert_eq!(preview["total"], "125");
    assert_eq!(before, Store::open(d.path()).unwrap().local_generation());
    let posted = call(
        d.path(),
        "payment_post",
        json!({"company_id":c,"period_id":p,"source_id":"bridge-pay","request_key":"bridge-pay","date":"2026-10-08","reason":"Customer settlement","side":"receive","party_id":"CU-A","funds_role":"bank","reference":"RCP-1","advance_amount":"25","allocations":[{"open_item_id":"AR-125","amount":"100"}],"expected_generation":preview["_generation"]}),
    );
    let record_id = posted["record_id"].as_str().unwrap().to_owned();
    assert!(posted["journal_id"].as_str().is_some());
    let w = call(d.path(), "workspace", json!({"company_id":c}));
    assert_eq!(w["payment_count"], "1");
    assert_eq!(w["capabilities"]["payments"], true);
    assert_eq!(w["payments"][0]["id"], record_id);
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
    assert_eq!(ar["remaining"], "25");
    let r = call(
        d.path(),
        "report",
        json!({"company_id":c,"from":"2026-10-01","to":"2026-10-08"}),
    );
    assert_eq!(r["party_balances"]["customer:CU-A"], "25");
    assert_eq!(r["reconciliation"]["reconciled"], true);
    // Typed reversal through the same bridge.
    let reversal_preview = call(
        d.path(),
        "payment_reverse_preview",
        json!({"company_id":c,"payment_id":record_id}),
    );
    assert_eq!(reversal_preview["total"], "125");
    call(
        d.path(),
        "payment_post",
        json!({"company_id":c,"period_id":p,"source_id":"bridge-rev","request_key":"bridge-rev","date":"2026-10-09","reason":"Reversal","payment_id":record_id,"expected_generation":reversal_preview["_generation"]}),
    );
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
    let w = call(d.path(), "workspace", json!({"company_id":c}));
    assert_eq!(w["payment_count"], "2");
    let r = call(
        d.path(),
        "report",
        json!({"company_id":c,"from":"2026-10-01","to":"2026-10-09"}),
    );
    assert_eq!(r["party_balances"]["customer:CU-A"], "125");
    assert_eq!(r["reconciliation"]["reconciled"], true);
}

#[test]
fn payment_rejects_overallocation_and_make_side_settles_payable() {
    let (d, c, p) = fixture();
    assert!(dispatch(
        d.path(),
        "payment_post",
        json!({"company_id":c,"period_id":p,"source_id":"bad","request_key":"bad","date":"2026-10-08","reason":"Too much","side":"receive","party_id":"CU-A","funds_role":"bank","reference":"X","advance_amount":"0","allocations":[{"open_item_id":"AR-125","amount":"1000"}]})
    )
    .is_err());
    assert!(dispatch(
        d.path(),
        "payment_post",
        json!({"company_id":c,"period_id":p,"source_id":"bad2","request_key":"bad2","date":"2026-10-08","reason":"Wrong side","side":"receive","party_id":"SU-A","funds_role":"bank","reference":"X","advance_amount":"0","allocations":[{"open_item_id":"AP-100","amount":"10"}]})
    )
    .is_err());
    let w = call(d.path(), "workspace", json!({"company_id":c}));
    assert_eq!(w["payment_count"], "0");
    call(
        d.path(),
        "payment_post",
        json!({"company_id":c,"period_id":p,"source_id":"pay-ap","request_key":"pay-ap","date":"2026-10-08","reason":"Supplier settlement","side":"make","party_id":"SU-A","funds_role":"cash","reference":"PMT-1","advance_amount":"0","allocations":[{"open_item_id":"AP-100","amount":"100"}]}),
    );
    let options = call(
        d.path(),
        "payment_options",
        json!({"company_id":c,"side":"make","party_id":"SU-A"}),
    );
    assert!(options["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|x| x["remaining"] != "100"));
    let r = call(
        d.path(),
        "report",
        json!({"company_id":c,"from":"2026-10-01","to":"2026-10-08"}),
    );
    assert_eq!(r["party_balances"]["vendor:SU-A"], "0");
    assert_eq!(r["reconciliation"]["reconciled"], true);
}

#[test]
fn quotation_issue_keeps_stable_numbers_and_prints_exact_totals() {
    let (d, c, p) = fixture();
    let s = Store::open(d.path()).unwrap();
    let setup = s.state.accounting.core.setups[0].id.clone();
    let next_number = s.state.settings.next_number;
    drop(s);
    let draft = json!({"id":"quote-draft","revision":"0","company_id":c,"period_id":p,"setup_id":setup,"customer_id":"CU-A","reference":"quote-ref","document_date":"2026-10-08","financial_date":"2026-10-08","date_display":"Iso","display_unit":"Rial","document_discount":"0","supplied_total":"30","reason":"Quotation test","quotation":null,"lines":[{"id":"q-line","item_id":"ITEM-A","warehouse_id":"W1","unit_id":"PIECE","quantity":"3","unit_price":"10","line_discount":"0","price_reference":"Manual","price_date":"2026-10-08","override_reason":""}]});
    let rev = call(
        d.path(),
        "draft_save",
        json!({"draft":draft,"expected_revision":null,"issue_quotation":true}),
    );
    assert_eq!(rev, "1");
    let w = call(d.path(), "workspace", json!({"company_id":c}));
    let stored = w["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == "quote-draft")
        .unwrap();
    let issue = &stored["quote_issue"];
    assert_eq!(
        issue["number"],
        json!(format!("Q-{next_number}"))
    );
    assert_eq!(issue["valid_until"], "2026-10-15");
    // Editing keeps the same quotation number without issuing again.
    let mut edited = stored.clone();
    edited["reference"] = json!("quote-ref-2");
    edited["revision"] = json!(rev);
    let rev2 = call(
        d.path(),
        "draft_save",
        json!({"draft":edited,"expected_revision":rev}),
    );
    assert_eq!(rev2, "2");
    let w = call(d.path(), "workspace", json!({"company_id":c}));
    let stored = w["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == "quote-draft")
        .unwrap();
    assert_eq!(stored["quote_issue"]["number"], issue["number"]);
    // The quotation still converts to a posted order with the exact total.
    let order = call(
        d.path(),
        "draft_post",
        json!({"id":"quote-draft","revision":"2","request_key":"quote-post"}),
    );
    assert!(order["record_id"].as_str().is_some());
    let s = Store::open(d.path()).unwrap();
    assert_eq!(s.state.settings.next_number, next_number + 1);
    assert!(s
        .state
        .sales_drafts
        .iter()
        .all(|x| x.quote_issue.is_some() || x.id != "quote-draft"));
}
