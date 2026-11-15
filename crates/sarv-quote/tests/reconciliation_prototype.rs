use sarv_quote::{
    desktop::{dispatch, seed_fixture},
    store::Store,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

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

fn ctx(c: &str, p: &str, key: &str, date: &str) -> Value {
    json!({
        "company_id": c,
        "period_id": p,
        "source_id": key,
        "request_key": key,
        "date": date,
        "reason": "Reconciliation prototype test",
        "reference": key,
        "due_date": date,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct LineRef {
    journal_id: String,
    line_index: usize,
}

#[derive(Clone, Debug)]
struct ReconciliationEntry {
    line_ref: LineRef,
    debit: i128,
    credit: i128,
}

fn compute_reconciliation(
    previous_reconciled_balance: i128,
    statement_balance: i128,
    entries: &[ReconciliationEntry],
    matched: &[LineRef],
) -> (i128, i128) {
    let matched_set: BTreeSet<&LineRef> = matched.iter().collect();
    let matched_delta: i128 = entries
        .iter()
        .filter(|e| matched_set.contains(&e.line_ref))
        .map(|e| e.debit - e.credit)
        .sum();
    let cleared_balance = previous_reconciled_balance + matched_delta;
    let difference = statement_balance - cleared_balance;
    (cleared_balance, difference)
}

fn parse_i128(v: &Value) -> i128 {
    v.as_str().expect("wire integer as string").parse::<i128>().expect("valid i128")
}

#[test]
fn banking_and_ledger_agree_and_reconciliation_arithmetic_holds() {
    let (d, c, p) = fixture();
    let store = Store::open(d.path()).unwrap();
    let setup = &store.state.accounting.core.setups[0];
    let bank_id = setup
        .command
        .mappings
        .iter()
        .find(|m| m.role == sarv_quote::core::CoreRole::Bank)
        .unwrap()
        .account_id
        .clone();
    let equity_id = setup
        .command
        .mappings
        .iter()
        .find(|m| m.role == sarv_quote::core::CoreRole::OwnerCapital)
        .unwrap()
        .account_id
        .clone();
    let expense_id = store
        .state
        .accounting
        .accounts
        .iter()
        .find(|a| {
            a.company_id == c
                && a.classification == sarv_quote::accounting::AccountClass::Expense
                && a.postable
                && !a.control
                && !a.archived
        })
        .unwrap()
        .id
        .clone();
    drop(store);

    // 1. Verify initial banking and ledger agreement on the fixture.
    let init_banking = call(
        d.path(),
        "banking",
        json!({"company_id": c, "from": "2026-03-21", "to": "2026-10-31"}),
    );
    let init_bank_row = init_banking["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["account"]["id"] == bank_id)
        .unwrap();
    let init_ledger = call(
        d.path(),
        "ledger",
        json!({"company_id": c, "account_id": bank_id, "from": "2026-03-21", "to": "2026-10-31"}),
    );
    assert_eq!(parse_i128(&init_ledger["opening_balance"]), parse_i128(&init_bank_row["opening"]));
    let init_ledger_closing = init_ledger["rows"]
        .as_array()
        .unwrap()
        .last()
        .map(|r| parse_i128(&r["balance"]))
        .unwrap_or_else(|| parse_i128(&init_ledger["opening_balance"]));
    assert_eq!(init_ledger_closing, parse_i128(&init_bank_row["balance"]));

    // 2. Post banking movements via preview + banking_post, plus a reversal.
    let mut dep1 = ctx(&c, &p, "rec-dep-1", "2026-10-10");
    dep1["kind"] = json!("deposit");
    dep1["account_id"] = json!(bank_id);
    dep1["counterpart_id"] = json!(equity_id);
    dep1["amount"] = json!("500000");
    let prev1 = call(d.path(), "banking_preview", dep1.clone());
    dep1["expected_generation"] = prev1["_generation"].clone();
    let j_dep1 = call(d.path(), "banking_post", dep1).as_str().unwrap().to_string();

    let mut wd1 = ctx(&c, &p, "rec-wd-1", "2026-10-12");
    wd1["kind"] = json!("withdrawal");
    wd1["account_id"] = json!(bank_id);
    wd1["counterpart_id"] = json!(expense_id);
    wd1["amount"] = json!("120000");
    let prev2 = call(d.path(), "banking_preview", wd1.clone());
    wd1["expected_generation"] = prev2["_generation"].clone();
    let j_wd1 = call(d.path(), "banking_post", wd1).as_str().unwrap().to_string();

    let mut dep2 = ctx(&c, &p, "rec-dep-2", "2026-10-15");
    dep2["kind"] = json!("deposit");
    dep2["account_id"] = json!(bank_id);
    dep2["counterpart_id"] = json!(equity_id);
    dep2["amount"] = json!("250000");
    let prev3 = call(d.path(), "banking_preview", dep2.clone());
    dep2["expected_generation"] = prev3["_generation"].clone();
    let j_dep2 = call(d.path(), "banking_post", dep2).as_str().unwrap().to_string();

    let mut dep3 = ctx(&c, &p, "rec-dep-3-err", "2026-10-16");
    dep3["kind"] = json!("deposit");
    dep3["account_id"] = json!(bank_id);
    dep3["counterpart_id"] = json!(equity_id);
    dep3["amount"] = json!("80000");
    let prev4 = call(d.path(), "banking_preview", dep3.clone());
    dep3["expected_generation"] = prev4["_generation"].clone();
    let j_dep3 = call(d.path(), "banking_post", dep3).as_str().unwrap().to_string();

    let mut rev = ctx(&c, &p, "rec-dep-3-rev", "2026-10-17");
    rev["journal_id"] = json!(j_dep3);
    let prev_rev = call(d.path(), "journal_reverse_preview", rev.clone());
    rev["expected_generation"] = prev_rev["_generation"].clone();
    let j_rev = call(d.path(), "journal_reverse", rev).as_str().unwrap().to_string();

    // 3. Fetch movements via banking and ledger for 2026-10-10..=2026-10-31 and verify agreement.
    let banking = call(
        d.path(),
        "banking",
        json!({"company_id": c, "from": "2026-10-10", "to": "2026-10-31"}),
    );
    let bank_row = banking["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["account"]["id"] == bank_id)
        .unwrap();
    let ledger = call(
        d.path(),
        "ledger",
        json!({"company_id": c, "account_id": bank_id, "from": "2026-10-10", "to": "2026-10-31"}),
    );
    let open_bal = parse_i128(&ledger["opening_balance"]);
    assert_eq!(open_bal, parse_i128(&bank_row["opening"]));

    let ledger_rows = ledger["rows"].as_array().unwrap();
    assert_eq!(ledger_rows.len(), 5);
    let ledger_closing = parse_i128(&ledger_rows.last().unwrap()["balance"]);
    let banking_closing = parse_i128(&bank_row["balance"]);
    assert_eq!(ledger_closing, banking_closing);
    assert_eq!(banking_closing, open_bal + 630_000);
    assert_eq!(
        parse_i128(&bank_row["opening"]) + parse_i128(&bank_row["incoming"]) - parse_i128(&bank_row["outgoing"]),
        banking_closing
    );

    // Build ReconciliationEntry list from the store's immutable journals for the period.
    let store = Store::open(d.path()).unwrap();
    let mut entries = Vec::new();
    for j in store
        .state
        .accounting
        .journals
        .iter()
        .filter(|j| j.company_id == c && j.date.as_str() >= "2026-10-10" && j.date.as_str() <= "2026-10-31")
    {
        for (idx, line) in j.lines.iter().enumerate() {
            if line.account.id == bank_id {
                entries.push(ReconciliationEntry {
                    line_ref: LineRef {
                        journal_id: j.id.clone(),
                        line_index: idx,
                    },
                    debit: i128::from(line.debit.rials()),
                    credit: i128::from(line.credit.rials()),
                });
            }
        }
    }
    assert_eq!(entries.len(), 5);

    let ref_for = |jid: &str| entries.iter().find(|e| e.line_ref.journal_id == jid).unwrap().line_ref.clone();
    let r_dep1 = ref_for(&j_dep1);
    let r_wd1 = ref_for(&j_wd1);
    let r_dep2 = ref_for(&j_dep2);
    let r_dep3 = ref_for(&j_dep3);
    let r_rev = ref_for(&j_rev);

    // Scenario 1: All matched.
    // Hand-calculated: open_bal + 500_000 - 120_000 + 250_000 + 80_000 - 80_000 = open_bal + 630_000.
    let stmt_all = open_bal + 630_000;
    let (cleared, diff) = compute_reconciliation(
        open_bal,
        stmt_all,
        &entries,
        &[r_dep1.clone(), r_wd1.clone(), r_dep2.clone(), r_dep3.clone(), r_rev.clone()],
    );
    assert_eq!(cleared, open_bal + 630_000);
    assert_eq!(diff, 0);

    // Scenario 2: One outstanding deposit (j_dep2 = +250_000 not yet on bank statement).
    // Hand-calculated: statement = open_bal + 380_000; matched = dep1, wd1, dep3, rev => cleared = open_bal + 380_000, diff = 0.
    let stmt_outstanding = open_bal + 380_000;
    let (cleared, diff) = compute_reconciliation(
        open_bal,
        stmt_outstanding,
        &entries,
        &[r_dep1.clone(), r_wd1.clone(), r_dep3.clone(), r_rev.clone()],
    );
    assert_eq!(cleared, open_bal + 380_000);
    assert_eq!(diff, 0);
    // If dep1 is not yet ticked while statement is open_bal + 380_000:
    // cleared = open_bal - 120_000, diff = +500_000.
    let (cleared_partial, diff_partial) = compute_reconciliation(
        open_bal,
        stmt_outstanding,
        &entries,
        &[r_wd1.clone(), r_dep3.clone(), r_rev.clone()],
    );
    assert_eq!(cleared_partial, open_bal - 120_000);
    assert_eq!(diff_partial, 500_000);

    // Scenario 3: Reversed entry (j_dep3 = +80_000 and j_rev = -80_000 cancel out).
    // 3a: Neither matched (error corrected before hitting bank) => cleared = open_bal + 630_000, diff = 0.
    let (cleared_neither, diff_neither) =
        compute_reconciliation(open_bal, stmt_all, &entries, &[r_dep1.clone(), r_wd1.clone(), r_dep2.clone()]);
    assert_eq!(cleared_neither, open_bal + 630_000);
    assert_eq!(diff_neither, 0);
    // 3b: Both matched => cleared = open_bal + 630_000, diff = 0.
    let (cleared_both, diff_both) = compute_reconciliation(
        open_bal,
        stmt_all,
        &entries,
        &[r_dep1.clone(), r_wd1.clone(), r_dep2.clone(), r_dep3.clone(), r_rev],
    );
    assert_eq!(cleared_both, open_bal + 630_000);
    assert_eq!(diff_both, 0);
    // 3c: Only original matched (reversal omitted) => cleared = open_bal + 710_000, diff = -80_000.
    let (cleared_one, diff_one) = compute_reconciliation(open_bal, stmt_all, &entries, &[r_dep1, r_wd1, r_dep2, r_dep3]);
    assert_eq!(cleared_one, open_bal + 710_000);
    assert_eq!(diff_one, -80_000);
}
