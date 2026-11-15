use super::harness::{assert_no_danger, blow, books_validate, click, count, make_test_runtime, trial_balance_is_balanced, type_into};
use crate::app::Page;
use crate::data;

fn account_balance(rt: &rune::Runtime<crate::app::State>, id: &str) -> i64 {
    rt.state.books.banking["accounts"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| data::s(&r["account"]["id"]) == id))
        .and_then(|r| data::s(&r["balance"]).parse::<i64>().ok())
        .unwrap_or_else(|| panic!("missing bank/cash account `{id}` in {}", rt.state.books.banking))
}

#[test]
fn money_in_posts_one_balanced_journal() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;

    let accounts = rt.state.books.banking["accounts"].as_array().cloned().unwrap_or_default();
    let bank_id = accounts
        .iter()
        .find(|r| data::s(&r["role"]).contains("bank") || data::s(&r["account"]["code"]).starts_with("112"))
        .or_else(|| accounts.first())
        .map(|r| data::s(&r["account"]["id"]))
        .expect("bank account in demo books");
    let counterpart_id = rt.state.books.banking["counterparts"]
        .as_array()
        .and_then(|rows| rows.first())
        .map(|r| data::s(&r["id"]))
        .expect("counterpart account in demo books");

    let before_journals = count(&rt, "journal_count");
    let before_bal = account_balance(&rt, &bank_id);

    click(&mut rt, Page::Banking.key());
    click(&mut rt, "btn-Money in");
    // Select pickers cannot be driven headlessly via dropdown item keys; set IDs directly.
    rt.state.banking.from_id = bank_id.clone();
    rt.state.banking.to_id = counterpart_id;
    type_into(&mut rt, "bank-amount", "50000");
    type_into(&mut rt, "bank-reason", "Owner deposit");
    click(&mut rt, "btn-Save");

    assert!(!rt.state.banking.modal_open, "{}", blow(&rt, "money in save"));
    assert_eq!(count(&rt, "journal_count"), before_journals + 1);
    assert_eq!(account_balance(&rt, &bank_id), before_bal + 50000);
    assert!(trial_balance_is_balanced(&rt), "trial balance must remain balanced");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn transfer_between_cash_and_bank_moves_the_amount() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;

    let accounts = rt.state.books.banking["accounts"].as_array().cloned().unwrap_or_default();
    assert!(accounts.len() >= 2, "need at least two cash/bank accounts: {accounts:?}");
    let cash_id = accounts
        .iter()
        .find(|r| data::s(&r["role"]).contains("cash") || data::s(&r["account"]["code"]).starts_with("111"))
        .unwrap_or(&accounts[0]);
    let cash_id = data::s(&cash_id["account"]["id"]);
    let bank_id = accounts
        .iter()
        .find(|r| data::s(&r["account"]["id"]) != cash_id)
        .map(|r| data::s(&r["account"]["id"]))
        .expect("second cash/bank account");

    let cash_before = account_balance(&rt, &cash_id);
    let bank_before = account_balance(&rt, &bank_id);
    let pos_before = rt.state.books.position("cash").parse::<i64>().unwrap();

    click(&mut rt, Page::Banking.key());
    click(&mut rt, "btn-Transfer");
    // Select pickers cannot be driven headlessly via dropdown item keys; set IDs directly.
    rt.state.banking.from_id = cash_id.clone();
    rt.state.banking.to_id = bank_id.clone();
    type_into(&mut rt, "bank-amount", "30000");
    type_into(&mut rt, "bank-reason", "Cash to bank");
    click(&mut rt, "btn-Save");

    assert!(!rt.state.banking.modal_open, "{}", blow(&rt, "transfer save"));
    assert_eq!(account_balance(&rt, &cash_id), cash_before - 30000);
    assert_eq!(account_balance(&rt, &bank_id), bank_before + 30000);
    assert_eq!(rt.state.books.position("cash").parse::<i64>().unwrap(), pos_before);
    assert_no_danger(&rt);
    books_validate(&rt);
}
