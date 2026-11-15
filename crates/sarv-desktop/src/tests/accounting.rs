use super::harness::{blow, click, make_test_runtime};
use crate::app::Page;

#[test]
fn test_accounting_tabs_and_tables_headlessly() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Accounting.key());
    assert!(rt.find("accounting-tabs").is_some());
    assert!(rt.find("journals-table").is_some());

    // Direct state edits need an interaction to rebuild the tree.
    rt.state.accounting.tab_idx = 1;
    rt.state.accounting.table = rune_kit::TableState::new();
    click(&mut rt, "refresh-books");
    assert_eq!(rt.state.accounting.tab_idx, 1);
    assert!(rt.find("coa-table").is_some());

    rt.state.accounting.tab_idx = 2;
    rt.state.accounting.table = rune_kit::TableState::new();
    click(&mut rt, "refresh-books");
    assert!(rt.find("periods-table").is_some());
}

#[test]
fn test_journal_post_uses_preview_generation() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2, "need two postable accounts");
    let debit = accounts[0].0.clone();
    let credit = accounts[1].0.clone();
    let before = rt.state.books.count("journal_count");
    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    assert!(!rt.state.accounting.post_key.is_empty(), "dialog mints a stable request key");
    rt.state.accounting.debit_account = debit;
    rt.state.accounting.credit_account = credit;
    rt.state.forms.set("journal-amount", "1000");
    rt.state.forms.set("journal-reason", "Quality loop journal");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "journal post"));
    assert!(rt.state.books.count("journal_count") > before, "journal posted");
}

#[test]
fn test_fiscal_period_follows_the_jalali_year() {
    use chrono::NaiveDate;
    let (year, start, end) =
        crate::ui::views::setup::fiscal_period(crate::i18n::CalendarKind::Jalali, NaiveDate::from_ymd_opt(2026, 11, 11).unwrap());
    assert_eq!(year, "1405");
    assert_eq!(start, "2026-03-21");
    assert_eq!(end, "2027-03-20");
}

#[test]
fn test_period_label_is_locked_or_open() {
    assert_eq!(crate::ui::views::accounting::period_status_key(true), "Locked");
    assert_eq!(crate::ui::views::accounting::period_status_key(false), "Open");
}

#[test]
fn test_accounting_and_banking_copy_helpers() {
    assert_eq!(
        crate::ui::views::accounting::tab_empty_copy(0),
        ("Every entry behind your numbers.", "No journals")
    );
    assert_eq!(crate::ui::views::accounting::tab_empty_copy(1).1, "No accounts");
    assert_eq!(crate::ui::views::accounting::tab_empty_copy(2).1, "No fiscal periods");
    assert_eq!(crate::ui::views::banking::move_dialog_title(0), "Transfer");
    assert_eq!(crate::ui::views::banking::move_dialog_title(1), "Money in");
    assert_eq!(crate::ui::views::banking::move_dialog_title(2), "Money out");
}
