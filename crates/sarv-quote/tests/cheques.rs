//! ADR-0005: cheque register wired into the ledger. Every lifecycle step must post balanced
//! entries through an audited source, keep every control account reconciled, and survive a full
//! replay (`State::validate`) and a serialise/reload round trip unchanged.
use sarv_quote::{
    accounting::{AccountingState, Money},
    cheques::{ChequeAct, ChequeDetails, ChequeRegister, CHEQUE_CLEAR_EVENT},
    core::CoreRole,
    desktop::dispatch_in,
    domain::State,
    iran::cheque::{self as machine, Direction},
    payments::{PaymentAllocation, PaymentContext, PaymentInput, PaymentSide},
    store::Store,
};
use serde_json::json;

const FAR: &str = "9999-12-31";

fn irr(v: i64) -> Money {
    Money::from_rials(v)
}
fn fixture() -> State {
    let v: serde_json::Value = serde_json::from_str(include_str!("../../../docs/assets/accounting-m3/restore/core-v4.json")).unwrap();
    let accounting: AccountingState = serde_json::from_value(v["state"]["accounting"].clone()).unwrap();
    let state = State { accounting, ..State::default() };
    state.validate().expect("fixture is a valid book");
    state
}
fn company(s: &State) -> String {
    s.accounting.core.openings[0].command.company_id.clone()
}
fn ctx(s: &State, key: &str, date: &str) -> PaymentContext {
    let c = &s.accounting.core.openings[0].command;
    PaymentContext {
        company_id: c.company_id.clone(),
        period_id: c.period_id.clone(),
        setup_id: c.setup_id.clone(),
        source_id: key.into(),
        source_revision: 1,
        date: date.into(),
        request_key: format!("key-{key}"),
        actor: "cheque-test".into(),
        reason: "synthetic cheque test".into(),
    }
}
fn details(sayad: &str) -> ChequeDetails {
    ChequeDetails {
        sayad_id: sayad.into(),
        number: "123456".into(),
        bank: "Melli".into(),
        branch: "Valiasr".into(),
        due_date: "2026-05-01".into(),
        drawer_national_id: "0084575948".into(),
        purpose: "invoice settlement".into(),
        notes: String::new(),
    }
}
fn pay(s: &State, key: &str, side: PaymentSide, party: &str, open_item: &str, amount: i64) -> PaymentInput {
    PaymentInput {
        context: ctx(s, key, "2026-03-25"),
        side,
        party_id: party.into(),
        // The register overrides the funds role; Cash here proves it cannot be smuggled in.
        funds_role: CoreRole::Cash,
        reference: format!("CHQ-{key}"),
        allocations: vec![PaymentAllocation { open_item_id: open_item.into(), amount: irr(amount) }],
        advance_amount: irr(0),
    }
}
fn account(s: &State, role: CoreRole) -> String {
    s.accounting.core.openings[0].frozen_setup.account_id(role).unwrap().to_owned()
}
/// Debit-normal balance of a role's account (credit-normal roles are negative).
fn net(s: &State, role: CoreRole) -> i64 {
    let id = account(s, role);
    let tb = s.accounting.trial_balance(&company(s), FAR).unwrap();
    tb.rows.iter().find(|r| r.account_id == id).map_or(0, |r| r.debit.rials() - r.credit.rials())
}
fn receivable(s: &State, id: &str) -> i64 {
    s.accounting.sales_receivables(&company(s), FAR).unwrap().into_iter().find(|r| r.id == id).map_or(0, |r| r.remaining.rials())
}
fn payable(s: &State, id: &str) -> i64 {
    s.accounting.procurement_open_payables(&company(s), FAR).unwrap().into_iter().find(|r| r.id == id).map_or(0, |r| r.remaining.rials())
}

/// The invariant every cheque step must leave behind.
fn assert_sound(s: &State, what: &str) {
    let tb = s.accounting.trial_balance(&company(s), FAR).unwrap();
    assert_eq!(tb.total_debit, tb.total_credit, "{what}: trial balance must balance");
    let rec = s.accounting.procurement_reconcile(&company(s), FAR).unwrap();
    assert!(rec.reconciled, "{what}: control accounts must reconcile: {:?}", rec.controls);
    s.validate().unwrap_or_else(|e| panic!("{what}: full replay rejected the book: {e}"));
    // Serialise → reload: identical value, still valid.
    let reloaded: State = serde_json::from_str(&serde_json::to_string(s).unwrap()).unwrap();
    assert_eq!(&reloaded, s, "{what}: round trip must be lossless");
    reloaded.validate().unwrap();
}

#[test]
fn received_cheque_deposit_then_clear() {
    let mut s = fixture();
    let (bank0, notes0) = (net(&s, CoreRole::Bank), net(&s, CoreRole::ChequesReceivable));
    let input = pay(&s, "R1", PaymentSide::Receive, "CU-A", "AR-125", 125);
    let cheque = s.receive_cheque(details("1234567890123456"), input).unwrap();
    assert_sound(&s, "received");
    assert_eq!(receivable(&s, "AR-125"), 0, "the cheque settles the invoice");
    assert_eq!(net(&s, CoreRole::ChequesReceivable), notes0 + 125);
    assert_eq!(s.cheques.status(&cheque).unwrap(), machine::State::InHand);

    let journals = s.accounting.journals.len();
    s.deposit_cheque(&ctx(&s, "D1", "2026-03-26"), &cheque).unwrap();
    assert_eq!(s.accounting.journals.len(), journals, "deposit is status-only (spec: optional step)");
    assert_eq!(s.cheques.status(&cheque).unwrap(), machine::State::InCollection);
    assert_sound(&s, "deposited");

    let bank_account = account(&s, CoreRole::Bank);
    let journal = s.clear_cheque(&ctx(&s, "C1", "2026-03-27"), &cheque, &bank_account).unwrap();
    assert_eq!(s.cheques.status(&cheque).unwrap(), machine::State::Cleared);
    assert_eq!(net(&s, CoreRole::Bank), bank0 + 125, "cash reached the bank");
    assert_eq!(net(&s, CoreRole::ChequesReceivable), notes0, "the notes account is back to its opening balance");
    assert_sound(&s, "cleared");

    // The posted entry is exactly what the pure state machine prescribes for this step.
    let rec = s.accounting.banking.iter().find(|b| b.journal_id == journal).unwrap();
    assert_eq!(rec.command.event, CHEQUE_CLEAR_EVENT);
    let (_, lines) = machine::apply(Direction::Received, machine::State::InCollection, machine::Action::Clear, 125).unwrap();
    for l in &lines {
        let role = match l.role {
            machine::Role::Bank => CoreRole::Bank,
            _ => CoreRole::ChequesReceivable,
        };
        let posted = rec.command.lines.iter().find(|p| p.account_id == account(&s, role)).unwrap();
        assert_eq!((posted.debit.rials(), posted.credit.rials()), (l.debit_rial, l.credit_rial), "{:?}", l.role);
    }
}

#[test]
fn bounced_cheque_reopens_the_invoice() {
    let mut s = fixture();
    let notes0 = net(&s, CoreRole::ChequesReceivable);
    let cheque = s.receive_cheque(details("2234567890123456"), pay(&s, "R2", PaymentSide::Receive, "CU-A", "AR-125", 125)).unwrap();
    s.deposit_cheque(&ctx(&s, "D2", "2026-03-26"), &cheque).unwrap();
    s.reverse_cheque(&ctx(&s, "B2", "2026-03-30"), &cheque, ChequeAct::Bounced).unwrap();
    assert_eq!(s.cheques.status(&cheque).unwrap(), machine::State::Bounced);
    assert_eq!(receivable(&s, "AR-125"), 125, "the customer owes the invoice again");
    assert_eq!(net(&s, CoreRole::ChequesReceivable), notes0);
    assert_sound(&s, "bounced");
}

#[test]
fn cheque_returned_to_the_customer_while_in_hand() {
    let mut s = fixture();
    let cheque = s.receive_cheque(details("3234567890123456"), pay(&s, "R3", PaymentSide::Receive, "CU-B", "AR-75", 75)).unwrap();
    s.reverse_cheque(&ctx(&s, "T3", "2026-03-28"), &cheque, ChequeAct::Returned).unwrap();
    assert_eq!((s.cheques.status(&cheque).unwrap(), receivable(&s, "AR-75")), (machine::State::Returned, 75));
    assert_sound(&s, "returned");
}

#[test]
fn received_cheque_endorsed_to_a_supplier() {
    let mut s = fixture();
    let notes0 = net(&s, CoreRole::ChequesReceivable);
    let cheque = s.receive_cheque(details("4234567890123456"), pay(&s, "R4", PaymentSide::Receive, "CU-B", "AR-75", 75)).unwrap();
    let spend = pay(&s, "S4", PaymentSide::Make, "SU-A", "AP-100", 75);
    s.spend_cheque(&cheque, spend).unwrap();
    assert_eq!(s.cheques.status(&cheque).unwrap(), machine::State::Spent);
    assert_eq!(payable(&s, "AP-100"), 25, "the supplier is paid 75 with the customer's cheque");
    assert_eq!(net(&s, CoreRole::ChequesReceivable), notes0, "the cheque left our hands");
    assert_sound(&s, "spent");
    // A spent cheque can no longer be deposited, cleared or returned.
    let mut again = s.clone();
    assert!(again.deposit_cheque(&ctx(&s, "X1", "2026-04-01"), &cheque).is_err());
    assert!(again.reverse_cheque(&ctx(&s, "X2", "2026-04-01"), &cheque, ChequeAct::Returned).is_err());
    assert_eq!(again, s, "a refused step changes nothing");
}

#[test]
fn issued_cheque_clears_out_of_the_bank() {
    let mut s = fixture();
    let (bank0, payable0) = (net(&s, CoreRole::Bank), net(&s, CoreRole::ChequesPayable));
    let cheque = s.issue_cheque(details("5234567890123456"), pay(&s, "I5", PaymentSide::Make, "SU-A", "AP-100", 100)).unwrap();
    assert_eq!(payable(&s, "AP-100"), 0);
    assert_eq!(net(&s, CoreRole::ChequesPayable), payable0 - 100, "liability to the bank/holder rises (credit)");
    assert_sound(&s, "issued");
    let bank_account = account(&s, CoreRole::Bank);
    s.clear_cheque(&ctx(&s, "C5", "2026-04-02"), &cheque, &bank_account).unwrap();
    assert_eq!(net(&s, CoreRole::Bank), bank0 - 100);
    assert_eq!(net(&s, CoreRole::ChequesPayable), payable0, "liability settled");
    assert_sound(&s, "issued+cleared");
}

#[test]
fn issued_cheque_voided_restores_the_supplier_payable() {
    let mut s = fixture();
    let cheque = s.issue_cheque(details("6234567890123456"), pay(&s, "I6", PaymentSide::Make, "SU-A", "AP-100", 100)).unwrap();
    s.reverse_cheque(&ctx(&s, "V6", "2026-03-30"), &cheque, ChequeAct::Voided).unwrap();
    assert_eq!((s.cheques.status(&cheque).unwrap(), payable(&s, "AP-100")), (machine::State::Voided, 100));
    assert_sound(&s, "voided");
}

#[test]
fn opening_cheque_joins_the_lifecycle_but_cannot_be_reversed() {
    let mut s = fixture();
    let (bank0, notes0) = (net(&s, CoreRole::Bank), net(&s, CoreRole::ChequesReceivable));
    let cheque = s.register_opening_cheque(&ctx(&s, "O1", "2026-03-22"), "CR1", details("")).unwrap();
    assert_sound(&s, "opening registered");
    // No originating payment exists, so bounce / return would break the invoice subledger.
    assert!(s.clone().reverse_cheque(&ctx(&s, "O2", "2026-03-23"), &cheque, ChequeAct::Returned).is_err());
    assert!(s.register_opening_cheque(&ctx(&s, "O3", "2026-03-22"), "CR1", details("")).is_err(), "registered once");
    let bank_account = account(&s, CoreRole::Bank);
    s.clear_cheque(&ctx(&s, "O4", "2026-03-25"), &cheque, &bank_account).unwrap();
    assert_eq!(net(&s, CoreRole::Bank), bank0 + 200);
    assert_eq!(net(&s, CoreRole::ChequesReceivable), notes0 - 200);
    assert_sound(&s, "opening cleared");
}

#[test]
fn refused_operations_leave_the_book_untouched() {
    let mut s = fixture();
    let cheque = s.receive_cheque(details("7234567890123456"), pay(&s, "R7", PaymentSide::Receive, "CU-A", "AR-125", 125)).unwrap();
    let bank_account = account(&s, CoreRole::Bank);
    s.clear_cheque(&ctx(&s, "C7", "2026-03-27"), &cheque, &bank_account).unwrap();
    let before = s.clone();
    let c = ctx(&s, "N7", "2026-03-28");
    assert!(s.clear_cheque(&c, &cheque, &bank_account).is_err(), "cleared twice");
    assert!(s.deposit_cheque(&c, &cheque).is_err());
    assert!(s.reverse_cheque(&c, &cheque, ChequeAct::Bounced).is_err());
    assert!(s.clear_cheque(&ctx(&s, "N8", "2026-03-28"), "no-such-cheque", &bank_account).is_err());
    assert!(s.reverse_cheque(&ctx(&s, "N9", "2026-03-28"), &cheque, ChequeAct::Cleared).is_err(), "not a reversal");
    assert_eq!(s, before);
}

#[test]
fn registration_guards() {
    let mut s = fixture();
    let ok = s.receive_cheque(details("8234567890123456"), pay(&s, "G1", PaymentSide::Receive, "CU-A", "AR-125", 50)).unwrap();
    let before = s.clone();
    // Duplicate Sayad ID, malformed Sayad ID, wrong side, bad due date.
    assert!(s.receive_cheque(details("8234567890123456"), pay(&s, "G2", PaymentSide::Receive, "CU-B", "AR-75", 75)).is_err());
    assert!(s.receive_cheque(details("123"), pay(&s, "G3", PaymentSide::Receive, "CU-B", "AR-75", 75)).is_err());
    assert!(s.receive_cheque(details("9234567890123456"), pay(&s, "G4", PaymentSide::Make, "SU-A", "AP-100", 10)).is_err());
    let mut late = details("9234567890123457");
    late.due_date = "1405/02/01".into();
    assert!(s.receive_cheque(late, pay(&s, "G5", PaymentSide::Receive, "CU-B", "AR-75", 75)).is_err());
    assert_eq!(s, before);
    let _ = ok;
}

#[test]
fn retrying_a_request_key_is_idempotent() {
    let mut s = fixture();
    let first = s.receive_cheque(details("1034567890123456"), pay(&s, "K1", PaymentSide::Receive, "CU-A", "AR-125", 125)).unwrap();
    let snapshot = s.clone();
    let again = s.receive_cheque(details("1034567890123456"), pay(&s, "K1", PaymentSide::Receive, "CU-A", "AR-125", 125)).unwrap();
    assert_eq!(first, again);
    assert_eq!(s, snapshot);
    let bank_account = account(&s, CoreRole::Bank);
    let j1 = s.clear_cheque(&ctx(&s, "K2", "2026-03-27"), &first, &bank_account).unwrap();
    let after_clear = s.clone();
    assert_eq!(s.clear_cheque(&ctx(&s, "K2", "2026-03-27"), &first, &bank_account).unwrap(), j1);
    assert_eq!(s, after_clear);
}

#[test]
fn tampering_with_the_register_is_caught_by_replay() {
    let mut s = fixture();
    let cheque = s.receive_cheque(details("1134567890123456"), pay(&s, "T1", PaymentSide::Receive, "CU-A", "AR-125", 125)).unwrap();
    let bank_account = account(&s, CoreRole::Bank);
    s.clear_cheque(&ctx(&s, "T2", "2026-03-27"), &cheque, &bank_account).unwrap();
    s.validate().unwrap();

    type Tamper = (&'static str, fn(&mut State));
    let tampers: &[Tamper] = &[
        ("amount changed", |s| s.cheques.cheques[0].amount = Money::from_rials(999)),
        ("clearing journal removed from the event", |s| s.cheques.events[1].journal_id = None),
        ("clearing points at the receipt payment's journal", |s| {
            let pay = s.accounting.payments.records[0].journal_id.clone();
            s.cheques.events[1].journal_id = Some(pay);
        }),
        ("receipt payment link removed", |s| s.cheques.events[0].payment_id = None),
        ("a cheque cleared without ever being registered", |s| { s.cheques.events.remove(0); }),
        ("status forged to bounced without a reversal", |s| s.cheques.events[1].action = ChequeAct::Bounced),
        ("same journal claimed by a second cheque", |s| {
            let mut c = s.cheques.cheques[0].clone();
            c.id = "forged".into();
            c.details.sayad_id = "9999999999999999".into();
            s.cheques.cheques.push(c);
            let mut e0 = s.cheques.events[0].clone();
            e0.id = "e-f0".into();
            e0.cheque_id = "forged".into();
            e0.request_key = "forged-0".into();
            s.cheques.events.push(e0);
        }),
    ];
    for (name, tamper) in tampers {
        let mut bad = s.clone();
        tamper(&mut bad);
        assert!(bad.validate().is_err(), "tamper not detected: {name}");
    }
    // Append-only: history can neither shrink nor be rewritten through a save.
    let mut shorter: ChequeRegister = s.cheques.clone();
    shorter.events.pop();
    assert!(s.cheques.validate_transition(&shorter).is_err());
    let mut rewritten = s.cheques.clone();
    rewritten.events[0].actor = "someone-else".into();
    assert!(s.cheques.validate_transition(&rewritten).is_err());
    let mut longer = s.cheques.clone();
    longer.events.push(longer.events[0].clone());
    assert!(s.cheques.validate_transition(&longer).is_ok());
}

#[test]
fn bridge_actions_persist_and_the_workspace_lists_allowed_next_steps() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let seeded = fixture();
    let company = company(&seeded);
    let period = seeded.accounting.core.openings[0].command.period_id.clone();
    let bank = account(&seeded, CoreRole::Bank);
    store.change(|s| { *s = seeded; Ok(()) }).unwrap();

    let base = |key: &str, date: &str| json!({"company_id": company, "period_id": period, "source_id": key, "request_key": key, "date": date, "reason": "bridge cheque test"});
    let mut receive = base("br-1", "2026-03-25");
    receive["party_id"] = json!("CU-A");
    receive["reference"] = json!("CHQ-BR-1");
    receive["allocations"] = json!([{"open_item_id": "AR-125", "amount": "125"}]);
    receive["cheque"] = serde_json::to_value(details("1234500000000001")).unwrap();
    let out = dispatch_in(&mut store, "cheque_receive", receive).unwrap();
    let cheque_id = out["cheque_id"].as_str().unwrap().to_owned();

    let ws = dispatch_in(&mut store, "workspace", json!({"company_id": company})).unwrap();
    let row = &ws["cheques"][0];
    assert_eq!(row["status"], "in_hand");
    let allowed: Vec<_> = row["allowed"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(allowed, ["deposit", "clear", "spend", "return"], "bounce needs a deposit first; void is for issued cheques");

    let mut deposit = base("br-2", "2026-03-26");
    deposit["cheque_id"] = json!(cheque_id);
    dispatch_in(&mut store, "cheque_deposit", deposit).unwrap();
    let mut clear = base("br-3", "2026-03-27");
    clear["cheque_id"] = json!(cheque_id);
    clear["fund_account_id"] = json!(bank);
    dispatch_in(&mut store, "cheque_clear", clear).unwrap();
    // A refused action over the bridge fails and does not advance the generation.
    let generation = store.local_generation();
    let mut again = base("br-4", "2026-03-28");
    again["cheque_id"] = json!(cheque_id);
    assert!(dispatch_in(&mut store, "cheque_deposit", again).is_err());
    assert_eq!(store.local_generation(), generation);

    // Reopen from disk: the register persisted, replays, and reports "cleared" with no next steps.
    drop(store);
    let mut reopened = Store::open(dir.path()).unwrap();
    reopened.state.validate().unwrap();
    let ws = dispatch_in(&mut reopened, "workspace", json!({"company_id": company})).unwrap();
    assert_eq!(ws["cheques"][0]["status"], "cleared");
    assert!(ws["cheques"][0]["allowed"].as_array().unwrap().is_empty());
}

/// The ledger enforces the clearing shape itself; it does not rely on the state machine
/// being the only caller (replay and restore feed it stored commands too).
#[test]
fn ledger_rejects_malformed_cheque_clearing_movements() {
    use sarv_quote::accounting::{JournalKind, JournalLineInput, PostCommand};
    let s = fixture();
    let c = ctx(&s, "M1", "2026-03-27");
    let (bank, recv, pay, cash) = (
        account(&s, CoreRole::Bank),
        account(&s, CoreRole::ChequesReceivable),
        account(&s, CoreRole::ChequesPayable),
        account(&s, CoreRole::OperatingExpense),
    );
    let line = |account: &str, debit: i64, credit: i64| JournalLineInput {
        account_id: account.into(),
        debit: irr(debit),
        credit: irr(credit),
        description: "x".into(),
    
        dimension: String::new(),
};
    let cmd = |key: &str, lines: Vec<JournalLineInput>| PostCommand {
        company_id: c.company_id.clone(),
        period_id: c.period_id.clone(),
        date: c.date.clone(),
        source_id: key.into(),
        source_revision: 1,
        event: CHEQUE_CLEAR_EVENT.into(),
        request_key: key.into(),
        actor: "t".into(),
        reason: "t".into(),
        kind: JournalKind::Manual,
        lines,
    };
    let post = |command| s.clone().accounting.post_banking(command);
    // Correct shapes are accepted: received clears into the bank, issued clears out of it.
    assert!(post(cmd("ok1", vec![line(&bank, 50, 0), line(&recv, 0, 50)])).is_ok());
    assert!(post(cmd("ok2", vec![line(&pay, 50, 0), line(&bank, 0, 50)])).is_ok());
    // Wrong direction for each notes account.
    assert!(post(cmd("bad1", vec![line(&bank, 0, 50), line(&recv, 50, 0)])).is_err(), "received cheque paying out of the bank");
    assert!(post(cmd("bad2", vec![line(&pay, 0, 50), line(&bank, 50, 0)])).is_err(), "issued cheque paying into the bank");
    // Unequal amounts, same-side lines, a non-notes counterpart, and no fund line at all.
    assert!(post(cmd("bad3", vec![line(&bank, 50, 0), line(&recv, 0, 40)])).is_err());
    assert!(post(cmd("bad4", vec![line(&bank, 50, 0), line(&recv, 50, 0)])).is_err());
    assert!(post(cmd("bad5", vec![line(&bank, 50, 0), line(&cash, 0, 50)])).is_err());
    assert!(post(cmd("bad6", vec![line(&recv, 50, 0), line(&pay, 0, 50)])).is_err());
}

#[test]
fn a_cheque_payment_cannot_be_reversed_behind_the_registers_back() {
    use sarv_quote::payments::{PaymentCommand, PaymentReversalInput};
    let mut s = fixture();
    let cheque = s.receive_cheque(details("1234567800000001"), pay(&s, "Z1", PaymentSide::Receive, "CU-A", "AR-125", 125)).unwrap();
    let payment_id = s.cheques.events[0].payment_id.clone().unwrap();
    // The generic payment reversal posts fine at the accounting level…
    let mut sneaky = s.clone();
    sneaky
        .accounting
        .post_payment_command(PaymentCommand::Reversal(PaymentReversalInput { context: ctx(&s, "Z2", "2026-03-26"), payment_id }))
        .unwrap();
    // …but the book is no longer valid: the cheque would still claim to be in hand.
    assert!(sneaky.validate().is_err());
    // Through the register it is the supported, status-consistent path.
    s.reverse_cheque(&ctx(&s, "Z3", "2026-03-26"), &cheque, ChequeAct::Returned).unwrap();
    assert_sound(&s, "returned via register");
}

#[test]
fn law_data_action_lists_every_value_as_unreviewed() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let out = dispatch_in(&mut store, "law_data", json!({})).unwrap();
    assert!(out["notice_en"].as_str().unwrap().starts_with("Not yet reviewed"));
    let entries = out["entries"].as_array().unwrap();
    assert!(!entries.is_empty() && entries.iter().all(|e| e["reviewed"] == false));
}
