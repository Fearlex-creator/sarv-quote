use sarv_quote::{
    core::CoreRole,
    desktop::{dispatch, seed_fixture},
    store::Store,
};
use serde_json::{Value, json};

fn call(dir: &std::path::Path, action: &str, p: Value) -> Value {
    dispatch(dir, action, p).unwrap_or_else(|e| panic!("action `{action}` failed: {e:#}"))
}

fn parse_i64(v: &Value) -> i64 {
    v.as_str()
        .unwrap_or_else(|| panic!("expected stringified i64, got {v}"))
        .parse::<i64>()
        .unwrap()
}

#[test]
fn year_end_closing_prototype() {
    let d = tempfile::tempdir().unwrap();
    seed_fixture(d.path()).unwrap();

    let store = Store::open(d.path()).unwrap();
    let company_id = store.state.accounting.companies[0].id.clone();
    let period1 = store.state.accounting.periods[0].clone();
    let setup = store
        .state
        .accounting
        .core
        .setups
        .iter()
        .find(|s| s.command.company_id == company_id)
        .expect("core setup")
        .clone();

    let role_acc = |role: CoreRole| setup.account_id(role).unwrap().to_string();
    let cash_id = role_acc(CoreRole::Cash);
    let sales_id = role_acc(CoreRole::Sales);
    let opex_id = role_acc(CoreRole::OperatingExpense);
    let bank_fees_id = role_acc(CoreRole::BankFees);
    let retained_id = role_acc(CoreRole::RetainedEarnings);
    let fixed_assets_id = role_acc(CoreRole::FixedAssets);

    let y1_start = period1.start_date.clone();
    let y1_end = period1.end_date.clone();

    // Baseline subledger snapshot in year 1 (before year-end close).
    let y1_pre_report = call(
        d.path(),
        "report",
        json!({"company_id": company_id, "from": y1_start, "to": y1_end}),
    );
    let y1_pre_stock = call(
        d.path(),
        "product_detail",
        json!({"company_id": company_id, "item_id": "ITEM-A", "as_of": y1_end}),
    );
    let y1_pre_ws = call(d.path(), "workspace", json!({"company_id": company_id}));
    let y1_pre_cheques = &y1_pre_ws["cheques"];

    println!("=== Baseline (seed_fixture) in Year 1 ({y1_start}..={y1_end}) ===");
    println!(
        "  P&L: revenue={}, expenses={}, profit={}",
        y1_pre_report["profit_loss"]["revenue"], y1_pre_report["profit_loss"]["expenses"], y1_pre_report["profit_loss"]["profit"]
    );
    println!(
        "  Open receivables={}, Open payables={}",
        y1_pre_report["reconciliation"]["receivables"].as_array().map_or(0, Vec::len),
        y1_pre_report["reconciliation"]["payables"].as_array().map_or(0, Vec::len),
    );
    println!(
        "  ITEM-A avg_stock_cost={}, stock_rows={}",
        y1_pre_stock["stats"]["average_stock_cost"],
        y1_pre_report["reconciliation"]["stock"].as_array().map_or(0, Vec::len)
    );
    println!("  Cheques count={}", y1_pre_cheques.as_array().map_or(0, Vec::len));

    // 1. Post income and expense journals in year 1 via journal_post (non-control accounts)
    //    and banking_post (cash <-> revenue/expense).
    call(
        d.path(),
        "journal_post",
        json!({
            "company_id": company_id,
            "period_id": period1.id,
            "date": "2026-10-08",
            "source_id": "proto-y1-inc",
            "request_key": "proto-y1-inc",
            "reason": "Year 1 service revenue",
            "lines": [
                {"account_id": fixed_assets_id, "debit": "500000", "credit": "0", "description": "Non-control asset debit"},
                {"account_id": sales_id, "debit": "0", "credit": "500000", "description": "Year 1 sales"}
            ]
        }),
    );
    call(
        d.path(),
        "journal_post",
        json!({
            "company_id": company_id,
            "period_id": period1.id,
            "date": "2026-11-15",
            "source_id": "proto-y1-exp",
            "request_key": "proto-y1-exp",
            "reason": "Year 1 operating expense & bank fees",
            "lines": [
                {"account_id": opex_id, "debit": "120000", "credit": "0", "description": "Rent"},
                {"account_id": bank_fees_id, "debit": "30000", "credit": "0", "description": "Fees"},
                {"account_id": fixed_assets_id, "debit": "0", "credit": "150000", "description": "Settled"}
            ]
        }),
    );
    // Also post a cash deposit from sales via banking_post to exercise cash control account.
    let generation = Store::open(d.path()).unwrap().local_generation().to_string();
    call(
        d.path(),
        "banking_post",
        json!({
            "company_id": company_id,
            "period_id": period1.id,
            "date": "2026-12-01",
            "source_id": "proto-y1-cash-sale",
            "request_key": "proto-y1-cash-sale",
            "reference": "proto-y1-cash-sale",
            "kind": "deposit",
            "account_id": cash_id,
            "counterpart_id": sales_id,
            "amount": "200000",
            "expected_generation": generation,
            "reason": "Cash sale deposit"
        }),
    );

    // 2. Compute year-1 net profit from `report`.
    let y1_before_close = call(
        d.path(),
        "report",
        json!({"company_id": company_id, "from": y1_start, "to": y1_end}),
    );
    let y1_revenue = parse_i64(&y1_before_close["profit_loss"]["revenue"]);
    let y1_expenses = parse_i64(&y1_before_close["profit_loss"]["expenses"]);
    let y1_profit = parse_i64(&y1_before_close["profit_loss"]["profit"]);
    assert_eq!(y1_profit, y1_revenue - y1_expenses);
    println!("\n=== Year 1 Before Closing ===");
    println!("  revenue={y1_revenue}, expenses={y1_expenses}, net_profit={y1_profit}");

    // 3. Build and post a closing journal through `journal_post` dated the last day of year 1.
    let mut closing_lines: Vec<Value> = Vec::new();
    for row in y1_before_close["profit_loss"]["rows"].as_array().unwrap() {
        let amt = parse_i64(&row["amount"]);
        if amt == 0 {
            continue;
        }
        let acc_id = row["account_id"].as_str().unwrap();
        let class = row["classification"].as_str().unwrap();
        match (class, amt > 0) {
            ("Revenue", true) => closing_lines.push(json!({
                "account_id": acc_id,
                "debit": amt.to_string(),
                "credit": "0",
                "description": "Close revenue"
            })),
            ("Revenue", false) => closing_lines.push(json!({
                "account_id": acc_id,
                "debit": "0",
                "credit": (-amt).to_string(),
                "description": "Close contra-revenue"
            })),
            ("Expense", true) => closing_lines.push(json!({
                "account_id": acc_id,
                "debit": "0",
                "credit": amt.to_string(),
                "description": "Close expense"
            })),
            ("Expense", false) => closing_lines.push(json!({
                "account_id": acc_id,
                "debit": (-amt).to_string(),
                "credit": "0",
                "description": "Close contra-expense"
            })),
            _ => unreachable!(),
        }
    }
    if y1_profit > 0 {
        closing_lines.push(json!({
            "account_id": retained_id,
            "debit": "0",
            "credit": y1_profit.to_string(),
            "description": "Net profit to Retained Earnings"
        }));
    } else if y1_profit < 0 {
        closing_lines.push(json!({
            "account_id": retained_id,
            "debit": (-y1_profit).to_string(),
            "credit": "0",
            "description": "Net loss to Retained Earnings"
        }));
    }

    call(
        d.path(),
        "journal_post",
        json!({
            "company_id": company_id,
            "period_id": period1.id,
            "date": y1_end,
            "source_id": "proto-y1-close",
            "request_key": "proto-y1-close",
            "reason": "Year 1 closing entry",
            "lines": closing_lines
        }),
    );

    // 4. Lock year 1.
    call(
        d.path(),
        "period_lock",
        json!({
            "company_id": company_id,
            "period_id": period1.id,
            "locked": true,
            "reason": "Close year 1"
        }),
    );

    // 5. Open year 2 via `period_add`.
    let generation = Store::open(d.path()).unwrap().local_generation().to_string();
    let y2_start = "2027-03-21";
    let y2_end = "2028-03-19";
    let p2_res = call(
        d.path(),
        "period_add",
        json!({
            "company_id": company_id,
            "name": "1406",
            "start_date": y2_start,
            "end_date": y2_end,
            "expected_generation": generation
        }),
    );
    let period2_id = p2_res["period_id"].as_str().unwrap().to_string();

    // 6. Post activity in year 2.
    call(
        d.path(),
        "journal_post",
        json!({
            "company_id": company_id,
            "period_id": period2_id,
            "date": "2027-04-10",
            "source_id": "proto-y2-inc",
            "request_key": "proto-y2-inc",
            "reason": "Year 2 revenue",
            "lines": [
                {"account_id": fixed_assets_id, "debit": "300000", "credit": "0", "description": "Y2 asset"},
                {"account_id": sales_id, "debit": "0", "credit": "300000", "description": "Y2 sales"}
            ]
        }),
    );
    call(
        d.path(),
        "journal_post",
        json!({
            "company_id": company_id,
            "period_id": period2_id,
            "date": "2027-05-01",
            "source_id": "proto-y2-exp",
            "request_key": "proto-y2-exp",
            "reason": "Year 2 expense",
            "lines": [
                {"account_id": opex_id, "debit": "80000", "credit": "0", "description": "Y2 opex"},
                {"account_id": fixed_assets_id, "debit": "0", "credit": "80000", "description": "Y2 settled"}
            ]
        }),
    );

    // 7. Observe and assert statements after close.
    let y1_after_close = call(
        d.path(),
        "report",
        json!({"company_id": company_id, "from": y1_start, "to": y1_end}),
    );
    let y2_report = call(
        d.path(),
        "report",
        json!({"company_id": company_id, "from": y2_start, "to": y2_end}),
    );
    let y2_stock = call(
        d.path(),
        "product_detail",
        json!({"company_id": company_id, "item_id": "ITEM-A", "as_of": y2_end}),
    );

    println!("\n=== Year 1 After Closing ===");
    println!(
        "  P&L: revenue={}, expenses={}, profit={}",
        y1_after_close["profit_loss"]["revenue"], y1_after_close["profit_loss"]["expenses"], y1_after_close["profit_loss"]["profit"]
    );
    // GAP: `report` sums all journals in `from..=to` without excluding closing entries,
    // so year-1 P&L reads 0 after posting the closing entry on the last day of year 1.
    assert_eq!(parse_i64(&y1_after_close["profit_loss"]["revenue"]), 0);
    assert_eq!(parse_i64(&y1_after_close["profit_loss"]["expenses"]), 0);
    assert_eq!(parse_i64(&y1_after_close["profit_loss"]["profit"]), 0);

    println!("\n=== Year 2 ({y2_start}..={y2_end}) ===");
    let y2_rev = parse_i64(&y2_report["profit_loss"]["revenue"]);
    let y2_exp = parse_i64(&y2_report["profit_loss"]["expenses"]);
    let y2_profit = parse_i64(&y2_report["profit_loss"]["profit"]);
    println!("  P&L: revenue={y2_rev}, expenses={y2_exp}, profit={y2_profit}");
    assert_eq!(y2_rev, 300_000);
    assert_eq!(y2_exp, 80_000);
    assert_eq!(y2_profit, 220_000);

    let bs = &y2_report["balance_sheet"];
    let retained_row = bs["equity"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["account_id"] == retained_id)
        .expect("RetainedEarnings row in Year 2 balance sheet");
    let retained_amt = parse_i64(&retained_row["amount"]);
    let current_earnings = parse_i64(&bs["current_earnings"]);
    println!(
        "  Balance sheet as_of={}: total_assets={}, total_liabilities={}, total_equity={}, balanced={}",
        bs["as_of"], bs["total_assets"], bs["total_liabilities"], bs["total_equity"], bs["balanced"]
    );
    println!("  Equity: retained_earnings={retained_amt}, current_earnings={current_earnings}");
    assert_eq!(bs["balanced"], true);
    assert_eq!(retained_amt, y1_profit);
    assert_eq!(current_earnings, y2_profit);

    let tb = &y2_report["trial"];
    let tb_debit = parse_i64(&tb["total_debit"]);
    let tb_credit = parse_i64(&tb["total_credit"]);
    let tb_rows = tb["rows"].as_array().map_or(0, Vec::len);
    println!(
        "  Trial balance as_of={}: rows={tb_rows}, total_debit={tb_debit}, total_credit={tb_credit}",
        tb["as_of"]
    );
    assert_eq!(tb_debit, tb_credit);

    // Subledger carry-forward verification across year boundary.
    assert_eq!(
        y2_report["reconciliation"]["receivables"],
        y1_pre_report["reconciliation"]["receivables"]
    );
    assert_eq!(y2_report["reconciliation"]["payables"], y1_pre_report["reconciliation"]["payables"]);
    assert_eq!(y2_report["reconciliation"]["stock"], y1_pre_report["reconciliation"]["stock"]);
    assert_eq!(y2_stock["stats"]["average_stock_cost"], y1_pre_stock["stats"]["average_stock_cost"]);
    let y2_ws = call(d.path(), "workspace", json!({"company_id": company_id}));
    let y2_cheques = &y2_ws["cheques"];
    assert_eq!(y2_cheques, y1_pre_cheques);
    println!(
        "  Subledgers carried forward: receivables={}, payables={}, stock_rows={}, avg_stock_cost={}, cheques={}",
        y2_report["reconciliation"]["receivables"].as_array().map_or(0, Vec::len),
        y2_report["reconciliation"]["payables"].as_array().map_or(0, Vec::len),
        y2_report["reconciliation"]["stock"].as_array().map_or(0, Vec::len),
        y2_stock["stats"]["average_stock_cost"],
        y2_cheques.as_array().map_or(0, Vec::len)
    );

    // 8. Store reopen & full validation.
    let reopened = Store::open(d.path()).unwrap();
    reopened.validate_full().unwrap();
    println!("\n=== Store::open + validate_full() OK ===");
}
