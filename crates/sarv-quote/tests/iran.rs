use chrono::NaiveDate;
use sarv_quote::iran::{
    cheque::{self, Action, Direction, Role, State},
    ids::{self, IdError},
    law, sheba, tax_id,
};
use serde_json::Value;
use std::collections::HashMap;

fn fixtures() -> Value {
    serde_json::from_str(include_str!("fixtures/iran_ids.json")).unwrap()
}
fn strings(v: &Value) -> Vec<&str> {
    v.as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect()
}
fn expected(reason: &str) -> IdError {
    match reason {
        "checksum" => IdError::Checksum,
        "repeated" => IdError::Repeated,
        "length" => IdError::Length,
        "digits" => IdError::NotDigits,
        other => panic!("unknown reason {other}"),
    }
}
fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn identifier_fixtures_hold_for_every_kind() {
    let f = fixtures();
    for (kind, check) in [
        ("national_id", ids::national_id as fn(&str) -> Result<(), IdError>),
        ("legal_id", ids::legal_id),
    ] {
        for v in strings(&f[kind]["valid"]) {
            assert_eq!(check(v), Ok(()), "{kind} {v}");
        }
        for case in f[kind]["invalid"].as_array().unwrap() {
            let (v, why) = (case["value"].as_str().unwrap(), case["reason"].as_str().unwrap());
            assert_eq!(check(v), Err(expected(why)), "{kind} {v}");
        }
    }
    for (kind, check) in [("postal_code", ids::postal_code as fn(&str) -> Result<(), IdError>), ("sayad_id", ids::sayad_id)] {
        assert!(strings(&f[kind]["valid"]).into_iter().all(|v| check(v).is_ok()), "{kind}");
        assert!(strings(&f[kind]["invalid"]).into_iter().all(|v| check(v).is_err()), "{kind}");
    }
}

#[test]
fn sheba_fixtures_and_generated_check_digits() {
    let f = fixtures();
    assert!(strings(&f["sheba"]["valid"]).into_iter().all(|v| sheba::validate(v).is_ok()));
    assert!(strings(&f["sheba"]["invalid"]).into_iter().all(|v| sheba::validate(v).is_err()));
    // Any 22-digit BBAN plus its computed check digits validates, and one wrong digit fails.
    for bban in ["0170000000123456789012", "0120000000000000000001", "0560000009876543210000"] {
        let iban = format!("IR{}{bban}", sheba::check_digits(bban).unwrap());
        assert!(sheba::validate(&iban).is_ok(), "{iban}");
        let mut bad = iban.clone().into_bytes();
        bad[25] = if bad[25] == b'9' { b'0' } else { bad[25] + 1 };
        assert!(sheba::validate(std::str::from_utf8(&bad).unwrap()).is_err());
    }
    let melli = format!("IR{}0170000000123456789012", sheba::check_digits("0170000000123456789012").unwrap());
    assert_eq!(sheba::bank_name(&melli), Some("بانک ملی ایران"));
    assert_eq!(sheba::bank_name("not a sheba"), None);
}

#[test]
fn verhoeff_matches_published_vectors() {
    assert_eq!(tax_id::verhoeff_check_digit("236"), Some(3));
    assert_eq!(tax_id::verhoeff_check_digit("12345"), Some(1));
    assert_eq!(tax_id::verhoeff_check_digit("142857"), Some(0));
    assert!(tax_id::verhoeff_valid("2363") && !tax_id::verhoeff_valid("2364"));
}

#[test]
fn moadian_tax_id_shape_roundtrip_and_tamper_detection() {
    let issued = date(2026, 11, 3);
    let id = tax_id::generate("a1b2c3", issued, 1).unwrap();
    assert_eq!(id.len(), 22);
    assert!(id.starts_with("A1B2C3"));
    let days = (issued - date(1970, 1, 1)).num_days();
    assert_eq!(&id[6..11], format!("{days:05X}"));
    assert_eq!(&id[11..21], "0000000001");
    assert!(tax_id::validate(&id));
    // Any single-character change is caught (Verhoeff detects all single-digit errors).
    for i in 0..22 {
        let mut chars: Vec<char> = id.chars().collect();
        chars[i] = if chars[i] == 'Z' { 'Y' } else if chars[i] == '9' { '8' } else { char::from_u32(chars[i] as u32 + 1).unwrap() };
        assert!(!tax_id::validate(&chars.into_iter().collect::<String>()), "position {i}");
    }
    assert_ne!(id, tax_id::generate("A1B2C3", issued, 2).unwrap(), "serial must change the ID");
    assert!(tax_id::generate("short", issued, 1).is_err());
    assert!(tax_id::generate("A1B2C3", date(1960, 1, 1), 1).is_err());
    assert!(tax_id::generate("A1B2C3", issued, 0x100_0000_0000).is_err());
}

#[test]
fn vat_is_ten_percent_for_1403_to_1405_and_unknown_elsewhere() {
    let today = law::standard_vat(date(2026, 11, 3)).unwrap(); // 1405-08-12
    assert_eq!((today.value, today.verify), (1000, false));
    assert!(law::standard_vat(date(2024, 3, 20)).is_some(), "1403-01-01");
    assert!(law::standard_vat(date(2027, 3, 20)).is_some(), "1405-12-29");
    assert!(law::standard_vat(date(2024, 3, 19)).is_none(), "1402-12-29 is outside the data pack");
    assert!(law::standard_vat(date(2027, 3, 21)).is_none(), "1406-01-01 is outside the data pack");
}

#[test]
fn salary_tax_1405_matches_hand_calculation() {
    const TOMAN: i64 = 10;
    const M: i64 = 1_000_000 * TOMAN; // one million toman, in rial
    assert!(law::salary_brackets(1405).unwrap().verify, "1405 brackets stay flagged for review");
    for (income, tax) in [
        (0, 0),
        (40 * M, 0),             // exactly the exemption
        (60 * M, 2 * M), // 20M toman above the exemption at 10% = 2M toman
        (100 * M, 7 * M), // 40M*10% + 20M*15% = 7M toman
        (200 * M, 34 * M), // 4 + 3 + 4 + 5 + 18 = 34M toman
    ] {
        assert_eq!(law::salary_tax(1405, income), Some(tax), "income {income}");
    }
    // Continuity: no jump at any bracket edge.
    for edge in [40, 80, 100, 120, 140] {
        let e = edge * M;
        let below = law::salary_tax(1405, e).unwrap();
        let above = law::salary_tax(1405, e + 1).unwrap();
        assert!(above - below <= 1, "jump at {edge}M toman");
    }
    assert_eq!(law::salary_tax(1404, 100 * M), None, "no data pack for 1404");
}

#[test]
fn insurance_and_payroll_golden() {
    let std = law::INSURANCE_STANDARD_VERIFY;
    let full = law::insurance(std, 1_000_000_000, None);
    assert_eq!((full.employee_rial, full.employer_rial, full.unemployment_rial), (70_000_000, 200_000_000, 30_000_000));
    let capped = law::insurance(std, 1_000_000_000, Some(500_000_000));
    assert_eq!((capped.base_rial, capped.employee_rial, capped.employer_rial), (500_000_000, 35_000_000, 100_000_000));
    // Gross 1,000M rial; taxable 930M after exempt allowances: tax = 40M + 15% of 130M = 59.5M.
    let r = law::payroll(law::PayrollInput {
        year: 1405,
        gross_rial: 1_000_000_000,
        insurable_rial: 1_000_000_000,
        taxable_rial: 930_000_000,
        insurance_ceiling_rial: None,
    })
    .unwrap();
    assert_eq!(r.salary_tax_rial, 59_500_000);
    assert_eq!(r.net_rial, 1_000_000_000 - 70_000_000 - 59_500_000);
    assert_eq!(r.employer_extra_rial, 230_000_000);
}

const ALL_STATES: [State; 8] = [State::InHand, State::InCollection, State::Cleared, State::Bounced, State::Spent, State::Returned, State::Issued, State::Voided];
const ALL_ACTIONS: [Action; 6] = [Action::Deposit, Action::Clear, Action::Bounce, Action::Spend, Action::ReturnToCustomer, Action::Void];

fn net(lines: &[cheque::Line]) -> HashMap<String, i64> {
    let mut m = HashMap::new();
    for l in lines {
        *m.entry(format!("{:?}", l.role)).or_insert(0) += l.debit_rial - l.credit_rial;
    }
    m
}

#[test]
fn every_cheque_transition_is_balanced_and_illegal_ones_are_rejected() {
    let mut legal = 0;
    for dir in [Direction::Received, Direction::Issued] {
        for state in ALL_STATES {
            for action in ALL_ACTIONS {
                let Ok((next, lines)) = cheque::apply(dir, state, action, 5_000_000) else { continue };
                legal += 1;
                assert_ne!(next, state, "{dir:?} {state:?} {action:?}");
                assert!(!lines.is_empty());
                assert_eq!(lines.iter().map(|l| l.debit_rial).sum::<i64>(), lines.iter().map(|l| l.credit_rial).sum::<i64>());
                assert!(lines.iter().all(|l| (l.debit_rial > 0) != (l.credit_rial > 0)), "exactly one side per line");
            }
        }
    }
    assert_eq!(legal, 9, "the spec defines exactly nine legal transitions");
    assert!(cheque::apply(Direction::Received, State::Cleared, Action::Deposit, 1).is_err(), "terminal state");
    assert!(cheque::apply(Direction::Issued, State::Issued, Action::Deposit, 1).is_err());
    assert!(cheque::apply(Direction::Received, State::InHand, Action::Bounce, 1).is_err(), "bounce happens via the bank");
    assert!(cheque::apply(Direction::Received, State::InHand, Action::Deposit, 0).is_err());
}

#[test]
fn received_cheque_lifecycle_nets_to_cash_in_bank_and_receivable_cleared() {
    let a = 12_500_000;
    let (s, mut all) = cheque::open(Direction::Received, a).unwrap();
    let (s, l) = cheque::apply(Direction::Received, s, Action::Deposit, a).unwrap();
    all.extend(l);
    let (s, l) = cheque::apply(Direction::Received, s, Action::Clear, a).unwrap();
    all.extend(l);
    assert_eq!(s, State::Cleared);
    let n = net(&all);
    assert_eq!(n["Bank"], a);
    assert_eq!(n["AccountsReceivable"], -a, "customer's debt settled");
    assert_eq!(n["NotesReceivable"], 0);
    assert_eq!(n["NotesInCollection"], 0);
}

#[test]
fn bounced_and_spent_and_issued_postings() {
    let a = 7_000_000;
    let (s, mut all) = cheque::open(Direction::Received, a).unwrap();
    let (s, l) = cheque::apply(Direction::Received, s, Action::Deposit, a).unwrap();
    all.extend(l);
    let (s, l) = cheque::apply(Direction::Received, s, Action::Bounce, a).unwrap();
    all.extend(l);
    assert_eq!(s, State::Bounced);
    assert!(net(&all).values().filter(|v| **v != 0).count() == 0, "a bounce restores AR: all roles net to zero");

    let (s, mut spent) = cheque::open(Direction::Received, a).unwrap();
    let (s, l) = cheque::apply(Direction::Received, s, Action::Spend, a).unwrap();
    spent.extend(l);
    assert_eq!(s, State::Spent);
    assert_eq!(net(&spent)["AccountsPayable"], a, "supplier debt reduced");

    let (s, mut issued) = cheque::open(Direction::Issued, a).unwrap();
    let (s, l) = cheque::apply(Direction::Issued, s, Action::Clear, a).unwrap();
    issued.extend(l);
    assert_eq!(s, State::Cleared);
    let n = net(&issued);
    assert_eq!((n["Bank"], n["AccountsPayable"], n["NotesPayable"]), (-a, a, 0));
    let _ = Role::Bank;
}

#[test]
fn legal_values_stay_unreviewed_and_gated_until_the_owner_signs_off() {
    // Every value in this build is unreviewed; the gate refuses all of them, including the
    // VAT rate whose own verify flag is clear (it still lacks owner sign-off).
    let vat = law::standard_vat(date(2026, 11, 3)).unwrap();
    let salary = law::salary_brackets(1405).unwrap();
    assert!(!vat.reviewed && !salary.reviewed);
    assert_eq!(law::require_reviewed(&vat), Err(law::NotReviewed));
    assert_eq!(law::require_reviewed(&salary), Err(law::NotReviewed));
    let mut signed = vat;
    signed.reviewed = true;
    assert_eq!(law::require_reviewed(&signed), Ok(()), "sign-off alone passes a value with no open verify flag");
    let mut signed_but_open = salary;
    signed_but_open.reviewed = true;
    assert_eq!(law::require_reviewed(&signed_but_open), Err(law::NotReviewed), "an open verify flag still blocks");

    let entries = law::entries();
    assert!(entries.len() >= 3 && entries.iter().all(|e| !e.reviewed));
    assert!(entries.iter().filter(|e| e.verify).count() >= 2);
    assert!(law::NOT_REVIEWED_NOTICE_EN.starts_with("Not yet reviewed"));
}
