use sarv_quote::{
    accounting::{AccountingState, Money},
    core::CoreRole,
    payments::*,
};
fn irr(v: i64) -> Money {
    Money::from_rials(v)
}
fn opening() -> AccountingState {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/assets/accounting-m3/restore/core-v4.json"
    ))
    .unwrap();
    let s: AccountingState = serde_json::from_value(v["state"]["accounting"].clone()).unwrap();
    s.validate().unwrap();
    s
}
fn cx(s: &AccountingState, id: &str, date: &str) -> PaymentContext {
    let c = &s.core.openings[0].command;
    PaymentContext {
        company_id: c.company_id.clone(),
        period_id: c.period_id.clone(),
        setup_id: c.setup_id.clone(),
        source_id: id.into(),
        source_revision: 1,
        date: date.into(),
        request_key: format!("key-{id}"),
        actor: "payments-fixture".into(),
        reason: "independent synthetic fixture".into(),
    }
}
fn company(s: &AccountingState) -> String {
    s.core.openings[0].command.company_id.clone()
}
fn receive(
    s: &AccountingState,
    source: &str,
    date: &str,
    open_item: &str,
    amount: i64,
    advance: i64,
) -> PaymentCommand {
    PaymentCommand::Payment(PaymentInput {
        context: cx(s, source, date),
        side: PaymentSide::Receive,
        party_id: "CU-A".into(),
        funds_role: CoreRole::Bank,
        reference: format!("RCP-{source}"),
        allocations: vec![PaymentAllocation {
            open_item_id: open_item.into(),
            amount: irr(amount),
        }],
        advance_amount: irr(advance),
    })
}
fn make(
    s: &AccountingState,
    source: &str,
    date: &str,
    open_item: &str,
    amount: i64,
) -> PaymentCommand {
    PaymentCommand::Payment(PaymentInput {
        context: cx(s, source, date),
        side: PaymentSide::Make,
        party_id: "SU-A".into(),
        funds_role: CoreRole::Cash,
        reference: format!("PMT-{source}"),
        allocations: vec![PaymentAllocation {
            open_item_id: open_item.into(),
            amount: irr(amount),
        }],
        advance_amount: irr(0),
    })
}
fn receivable(s: &AccountingState, id: &str) -> i64 {
    s.sales_receivables(&company(s), "9999-12-31")
        .unwrap()
        .into_iter()
        .find(|r| r.id == id)
        .unwrap()
        .remaining
        .rials()
}
fn payable(s: &AccountingState, id: &str) -> i64 {
    s.procurement_open_payables(&company(s), "9999-12-31")
        .unwrap()
        .into_iter()
        .find(|r| r.id == id)
        .unwrap()
        .remaining
        .rials()
}

#[test]
fn customer_payment_settles_invoice_keeps_advance_and_stays_reconciled() {
    let mut s = opening();
    let result = s
        .post_payment_command(receive(&s, "PAY1", "2026-03-25", "AR-125", 100, 25))
        .unwrap();
    assert!(!result.journal_id.is_empty());
    // 125 invoice: 100 allocated, 25 surplus became an explicit advance.
    assert_eq!(receivable(&s, "AR-125"), 25);
    assert!(s
        .procurement_reconcile(&company(&s), "9999-12-31")
        .unwrap()
        .reconciled);
    s.validate().unwrap();
    // Bank detail includes the received funds.
    let setup = &s.core.openings[0].frozen_setup;
    let bank = setup.account_id(CoreRole::Bank).unwrap();
    assert_eq!(
        s.payment_fund_delta(&company(&s), bank, "9999-12-31").unwrap(),
        irr(125)
    );
    assert_eq!(
        s.payment_advance_delta(&company(&s), CoreRole::CustomerAdvances, "9999-12-31")
            .unwrap(),
        irr(25)
    );
    // Retry with the same request key is idempotent.
    let retry = s
        .post_payment_command(receive(&s, "PAY1", "2026-03-25", "AR-125", 100, 25))
        .unwrap();
    assert_eq!(retry.record_id, result.record_id);
    assert_eq!(s.payments.records.len(), 1);
    s.validate().unwrap();
}

#[test]
fn supplier_payment_settles_payable_and_reversal_restores_it() {
    let mut s = opening();
    s.post_payment_command(make(&s, "PAY2", "2026-03-26", "AP-100", 100))
        .unwrap();
    assert_eq!(payable(&s, "AP-100"), 0);
    assert!(s
        .procurement_reconcile(&company(&s), "9999-12-31")
        .unwrap()
        .reconciled);
    s.validate().unwrap();
    // Reversing through the generic journal path is rejected.
    let journal = s.payments.records[0].journal_id.clone();
    assert!(s
        .reverse(&journal, &cx(&s, "X", "2026-03-27").period_id, "2026-03-27", "rk-x", "a", "r")
        .is_err());
    // Typed reversal restores the payable and net funds.
    let reversal = s
        .post_payment_command(PaymentCommand::Reversal(PaymentReversalInput {
            context: cx(&s, "REV2", "2026-03-27"),
            payment_id: s.payments.records[0].id.clone(),
        }))
        .unwrap();
    assert_eq!(payable(&s, "AP-100"), 100);
    assert!(s
        .procurement_reconcile(&company(&s), "9999-12-31")
        .unwrap()
        .reconciled);
    s.validate().unwrap();
    // A payment can only be reversed once.
    assert!(s
        .post_payment_command(PaymentCommand::Reversal(PaymentReversalInput {
            context: cx(&s, "REV2B", "2026-03-28"),
            payment_id: s.payments.records[0].id.clone(),
        }))
        .is_err());
    assert_eq!(s.payments.records.len(), 2);
    let _ = reversal;
}

#[test]
fn over_allocation_wrong_party_and_zero_total_are_rejected_atomically() {
    let mut s = opening();
    // More than the open item remaining.
    assert!(s
        .post_payment_command(receive(&s, "BAD1", "2026-03-25", "AR-125", 200, 0))
        .is_err());
    // Allocating another party's open item is invisible to this party.
    assert!(s
        .post_payment_command(receive(&s, "BAD2", "2026-03-25", "AR-75", 10, 0))
        .is_err());
    // Zero total is not a payment.
    assert!(s
        .post_payment_command(receive(&s, "BAD3", "2026-03-25", "AR-125", 0, 0))
        .is_err());
    // Duplicate open item rows.
    let mut dup = receive(&s, "BAD4", "2026-03-25", "AR-125", 10, 0);
    if let PaymentCommand::Payment(x) = &mut dup {
        x.allocations.push(PaymentAllocation {
            open_item_id: "AR-125".into(),
            amount: irr(5),
        });
    }
    assert!(s.post_payment_command(dup).is_err());
    // Date before the opening.
    assert!(s
        .post_payment_command(receive(&s, "BAD5", "2026-03-01", "AR-125", 10, 0))
        .is_err());
    assert!(s.payments.records.is_empty());
    assert!(s
        .procurement_reconcile(&company(&s), "9999-12-31")
        .unwrap()
        .reconciled);
    s.validate().unwrap();
}

#[test]
fn payment_survives_serialization_and_full_audit_replay() {
    let mut s = opening();
    s.post_payment_command(receive(&s, "PAY3", "2026-03-25", "AR-125", 125, 0))
        .unwrap();
    s.post_payment_command(make(&s, "PAY4", "2026-03-26", "AP-100", 60))
        .unwrap();
    let bytes = serde_json::to_value(&s).unwrap();
    let mut restored: AccountingState = serde_json::from_value(bytes).unwrap();
    assert_eq!(restored, s);
    assert_eq!(receivable(&restored, "AR-125"), 0);
    assert_eq!(payable(&restored, "AP-100"), 40);
    restored.validate().unwrap();
    // Retry aliases do not create second effects.
    let before = restored.payments.records.len();
    let alias = restored
        .post_payment_command(make(&s, "PAY4", "2026-03-26", "AP-100", 60))
        .unwrap();
    assert_eq!(restored.payments.records.len(), before);
    assert_eq!(alias.record_id, restored.payments.records[1].id);
    restored.validate().unwrap();
}

#[test]
fn partial_payments_accumulate_and_bills_settle_with_change() {
    let mut s = opening();
    s.post_payment_command(receive(&s, "P1", "2026-03-25", "AR-125", 50, 0))
        .unwrap();
    s.post_payment_command(receive(&s, "P2", "2026-03-26", "AR-125", 75, 10))
        .unwrap();
    assert_eq!(receivable(&s, "AR-125"), 0);
    // 135 received for a 125 invoice: the extra 10 sits in customer advances.
    assert_eq!(
        s.payment_advance_delta(&company(&s), CoreRole::CustomerAdvances, "9999-12-31")
            .unwrap(),
        irr(10)
    );
    assert!(s
        .procurement_reconcile(&company(&s), "9999-12-31")
        .unwrap()
        .reconciled);
    // Further allocation against the settled invoice is impossible.
    assert!(s
        .post_payment_command(receive(&s, "P3", "2026-03-27", "AR-125", 1, 0))
        .is_err());
    s.validate().unwrap();
}
