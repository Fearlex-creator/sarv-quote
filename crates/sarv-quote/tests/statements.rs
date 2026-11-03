//! Balance sheet and aging: read-only statements, checked against hand-calculated figures
//! from the synthetic fixture (open items are all due 2026-04-01).
use sarv_quote::{
    accounting::{AccountClass, AccountingState, Money},
    core::CoreRole,
    payments::{PaymentAllocation, PaymentCommand, PaymentContext, PaymentInput, PaymentSide},
    statements::{aging, balance_sheet},
};

fn irr(v: i64) -> Money {
    Money::from_rials(v)
}
fn books() -> AccountingState {
    let v: serde_json::Value = serde_json::from_str(include_str!("../../../docs/assets/accounting-m3/restore/core-v4.json")).unwrap();
    let s: AccountingState = serde_json::from_value(v["state"]["accounting"].clone()).unwrap();
    s.validate().unwrap();
    s
}
fn company(s: &AccountingState) -> String {
    s.core.openings[0].command.company_id.clone()
}
fn receive(s: &AccountingState, key: &str, open_item: &str, amount: i64) -> PaymentCommand {
    let c = &s.core.openings[0].command;
    PaymentCommand::Payment(PaymentInput {
        context: PaymentContext {
            company_id: c.company_id.clone(),
            period_id: c.period_id.clone(),
            setup_id: c.setup_id.clone(),
            source_id: key.into(),
            source_revision: 1,
            date: "2026-03-25".into(),
            request_key: key.into(),
            actor: "t".into(),
            reason: "t".into(),
        },
        side: PaymentSide::Receive,
        party_id: "CU-A".into(),
        funds_role: CoreRole::Bank,
        reference: key.into(),
        allocations: vec![PaymentAllocation { open_item_id: open_item.into(), amount: irr(amount) }],
        advance_amount: irr(0),
    })
}
fn age(s: &AccountingState, side: PaymentSide, as_of: &str) -> sarv_quote::statements::Aging {
    aging(s, &company(s), side, as_of).unwrap()
}

#[test]
fn nothing_is_overdue_before_the_due_date_or_on_it() {
    let s = books();
    for date in ["2026-03-25", "2026-04-01"] {
        let a = age(&s, PaymentSide::Receive, date);
        assert_eq!((a.totals.not_due.rials(), a.totals.total.rials()), (200, 200), "{date}");
        assert_eq!(a.totals.days_1_30, Money::ZERO);
        let p = age(&s, PaymentSide::Make, date);
        assert_eq!((p.totals.not_due.rials(), p.totals.total.rials()), (149, 149), "{date}");
    }
}

#[test]
fn every_bucket_boundary_lands_on_the_right_side() {
    let s = books();
    // (as-of date, days past 2026-04-01, bucket)
    let cases = [
        ("2026-04-02", 1, "1-30"),
        ("2026-05-01", 30, "1-30"),
        ("2026-05-02", 31, "31-60"),
        ("2026-05-31", 60, "31-60"),
        ("2026-06-01", 61, "61-90"),
        ("2026-06-30", 90, "61-90"),
        ("2026-07-01", 91, "90+"),
        ("2026-09-01", 153, "90+"),
    ];
    for (date, days, bucket) in cases {
        let t = age(&s, PaymentSide::Receive, date).totals;
        let (in_bucket, others) = match bucket {
            "1-30" => (t.days_1_30, [t.not_due, t.days_31_60, t.days_61_90, t.days_over_90]),
            "31-60" => (t.days_31_60, [t.not_due, t.days_1_30, t.days_61_90, t.days_over_90]),
            "61-90" => (t.days_61_90, [t.not_due, t.days_1_30, t.days_31_60, t.days_over_90]),
            _ => (t.days_over_90, [t.not_due, t.days_1_30, t.days_31_60, t.days_61_90]),
        };
        assert_eq!(in_bucket.rials(), 200, "{date} ({days} days) should sit in {bucket}");
        assert!(others.iter().all(|m| *m == Money::ZERO), "{date}: nothing leaks into other buckets");
    }
}

#[test]
fn rows_are_per_party_and_partial_payments_reduce_the_remainder() {
    let mut s = books();
    s.post_payment_command(receive(&s, "AGE1", "AR-125", 100)).unwrap();
    let a = age(&s, PaymentSide::Receive, "2026-05-02"); // 31 days overdue
    let by: Vec<_> = a.rows.iter().map(|r| (r.party_id.as_str(), r.days_31_60.rials(), r.total.rials())).collect();
    assert_eq!(by, [("CU-A", 25, 25), ("CU-B", 75, 75)]);
    assert_eq!(a.totals.total.rials(), 100);
    assert_eq!(a.rows.iter().map(|r| r.total.rials()).sum::<i64>(), a.totals.total.rials());
    // A fully settled item disappears.
    s.post_payment_command(receive(&s, "AGE2", "AR-125", 25)).unwrap();
    let a = age(&s, PaymentSide::Receive, "2026-05-02");
    assert_eq!(a.rows.iter().map(|r| r.party_id.as_str()).collect::<Vec<_>>(), ["CU-B"]);
    assert_eq!(a.side, "receivable");
    assert_eq!(age(&s, PaymentSide::Make, "2026-05-02").side, "payable");
}

#[test]
fn balance_sheet_satisfies_the_accounting_identity_and_matches_the_ledger() {
    let s = books();
    let sheet = balance_sheet(&s, &company(&s), "9999-12-31").unwrap();
    assert!(sheet.balanced, "assets {:?} = liabilities {:?} + equity {:?}", sheet.total_assets, sheet.total_liabilities, sheet.total_equity);
    assert!(sheet.total_assets.rials() > 0);
    assert_eq!(sheet.assets.iter().map(|r| r.amount.rials()).sum::<i64>(), sheet.total_assets.rials());
    assert_eq!(sheet.liabilities.iter().map(|r| r.amount.rials()).sum::<i64>(), sheet.total_liabilities.rials());
    // Independent recomputation straight from the trial balance and account classes.
    let tb = s.trial_balance(&company(&s), "9999-12-31").unwrap();
    let assets: i64 = tb
        .rows
        .iter()
        .filter(|r| s.accounts.iter().any(|a| a.id == r.account_id && a.classification == AccountClass::Asset))
        .map(|r| r.debit.rials() - r.credit.rials())
        .sum();
    assert_eq!(assets, sheet.total_assets.rials());
    assert!(sheet.assets.iter().all(|r| r.amount != Money::ZERO), "zero-balance accounts are omitted");
}

#[test]
fn a_customer_receipt_moves_value_between_assets_without_changing_the_total() {
    let mut s = books();
    let before = balance_sheet(&s, &company(&s), "9999-12-31").unwrap();
    s.post_payment_command(receive(&s, "BS1", "AR-125", 125)).unwrap();
    let after = balance_sheet(&s, &company(&s), "9999-12-31").unwrap();
    assert!(after.balanced);
    assert_eq!(after.total_assets, before.total_assets, "bank up 125, receivables down 125");
    assert_eq!(after.total_liabilities, before.total_liabilities);
    let bank = s.core.openings[0].frozen_setup.account_id(CoreRole::Bank).unwrap();
    let amount = |sheet: &sarv_quote::statements::BalanceSheet| sheet.assets.iter().find(|r| r.account_id == bank).map_or(0, |r| r.amount.rials());
    assert_eq!(amount(&after), amount(&before) + 125);
    // As of before the opening date there are no posted balances to report.
    assert!(balance_sheet(&s, &company(&s), "2026-03-20").map_or(true, |b| b.total_assets == Money::ZERO));
}

#[test]
fn report_action_carries_both_statements() {
    use sarv_quote::{desktop::dispatch_in, domain::State, store::Store};
    use serde_json::json;
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let accounting = books();
    let company = company(&accounting);
    store.change(|s| { *s = State { accounting, ..State::default() }; Ok(()) }).unwrap();
    let r = dispatch_in(&mut store, "report", json!({"company_id": company, "from": "2026-03-21", "to": "2026-05-02"})).unwrap();
    assert_eq!(r["balance_sheet"]["balanced"], true);
    assert_eq!(r["aging"]["receivable"]["side"], "receivable");
    assert_eq!(r["aging"]["receivable"]["totals"]["days_31_60"], "200");
    assert_eq!(r["aging"]["payable"]["totals"]["total"], "149");
}
