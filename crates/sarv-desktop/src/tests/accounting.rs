use super::harness::{
    assert_no_danger, blow, books_validate, click, count, danger_toasts, make_test_runtime, trial_balance_is_balanced, type_into,
};
use crate::app::Page;
use crate::data;

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

fn period_is_locked(rt: &rune::Runtime<crate::app::State>, period: &str) -> bool {
    rt.state.books.workspace["periods"]
        .as_array()
        .and_then(|rows| rows.iter().find(|p| data::s(&p["id"]) == period))
        .and_then(|p| p["locked"].as_bool())
        .unwrap_or(false)
}

fn account_net_balance(rt: &rune::Runtime<crate::app::State>, account_id: &str) -> i64 {
    rt.state.books.report["trial"]["rows"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| data::s(&r["account_id"]) == account_id))
        .map(|r| {
            let d = data::s(&r["debit"]).parse::<i64>().unwrap_or(0);
            let c = data::s(&r["credit"]).parse::<i64>().unwrap_or(0);
            d - c
        })
        .unwrap_or(0)
}

#[test]
fn locked_period_refuses_a_journal() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2);
    let debit = accounts[0].0.clone();
    let credit = accounts[1].0.clone();

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "segment2");
    for _ in 0..8 {
        rt.frame(1.0 / 60.0);
    }
    let period = rt.state.books.period_id.clone();
    click(&mut rt, &format!("period-{period}"));
    click(&mut rt, "btn-Lock fiscal period");
    click(&mut rt, "btn-Lock period");
    assert!(period_is_locked(&rt, &period), "{}", blow(&rt, "lock period"));

    click(&mut rt, "segment0");
    for _ in 0..8 {
        rt.frame(1.0 / 60.0);
    }
    let before_count = count(&rt, "journal_count");
    rt.update(|s, _| s.toasts.clear());
    rt.frame(1.0 / 60.0);

    click(&mut rt, "btn-New journal");
    rt.state.accounting.debit_account = debit.clone();
    rt.state.accounting.credit_account = credit.clone();
    type_into(&mut rt, "journal-amount", "15000");
    type_into(&mut rt, "journal-reason", "Blocked in locked period");
    click(&mut rt, "btn-Save");

    assert_eq!(
        danger_toasts(&rt).len(),
        1,
        "expected exactly one danger toast: {:?}",
        rt.state.toasts
    );
    assert_eq!(count(&rt, "journal_count"), before_count, "locked period must refuse journal");
    click(&mut rt, "btn-Cancel");
    click(&mut rt, "btn-Discard");

    click(&mut rt, "segment2");
    for _ in 0..8 {
        rt.frame(1.0 / 60.0);
    }
    click(&mut rt, &format!("period-{period}"));
    click(&mut rt, "btn-Reopen fiscal period");
    click(&mut rt, "btn-Reopen period");
    assert!(!period_is_locked(&rt, &period), "{}", blow(&rt, "reopen period"));

    rt.update(|s, _| s.toasts.clear());
    click(&mut rt, "segment0");
    for _ in 0..8 {
        rt.frame(1.0 / 60.0);
    }
    click(&mut rt, "btn-New journal");
    rt.state.accounting.debit_account = debit;
    rt.state.accounting.credit_account = credit;
    type_into(&mut rt, "journal-amount", "15000");
    type_into(&mut rt, "journal-reason", "Allowed after reopen");
    click(&mut rt, "btn-Save");

    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "post after reopen"));
    assert_eq!(count(&rt, "journal_count"), before_count + 1);
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn reverse_creates_a_mirror_journal() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2);
    let debit = accounts[0].0.clone();
    let credit = accounts[1].0.clone();

    let debit_before = account_net_balance(&rt, &debit);
    let credit_before = account_net_balance(&rt, &credit);

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    rt.state.accounting.debit_account = debit.clone();
    rt.state.accounting.credit_account = credit.clone();
    type_into(&mut rt, "journal-amount", "25000");
    type_into(&mut rt, "journal-reason", "Mirror test");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "journal post"));

    let after_post_count = count(&rt, "journal_count");
    let journal_id = rt
        .state
        .books
        .list("journals")
        .into_iter()
        .find(|j| data::s(&j["reason"]) == "Mirror test")
        .map(|j| data::s(&j["id"]))
        .unwrap_or_else(|| panic!("{}", blow(&rt, "journal id")));

    click(&mut rt, &format!("journal-{journal_id}"));
    click(&mut rt, "btn-Reverse");
    click(&mut rt, "btn-Reverse entry");

    assert_eq!(count(&rt, "journal_count"), after_post_count + 1);
    assert_eq!(account_net_balance(&rt, &debit), debit_before);
    assert_eq!(account_net_balance(&rt, &credit), credit_before);
    assert!(trial_balance_is_balanced(&rt));
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_second_journal_from_overview_posts() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 3);
    let before = count(&rt, "journal_count");

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    rt.state.accounting.debit_account = accounts[0].0.clone();
    rt.state.accounting.credit_account = accounts[1].0.clone();
    type_into(&mut rt, "journal-amount", "10000");
    type_into(&mut rt, "journal-reason", "First journal");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "first journal"));

    click(&mut rt, Page::Overview.key());
    click(&mut rt, "btn-New journal");
    assert!(rt.state.accounting.modal_open, "{}", blow(&rt, "overview new journal"));
    rt.state.accounting.debit_account = accounts[1].0.clone();
    rt.state.accounting.credit_account = accounts[2].0.clone();
    type_into(&mut rt, "journal-amount", "20000");
    type_into(&mut rt, "journal-reason", "Second journal from overview");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "second journal"));

    assert_eq!(count(&rt, "journal_count"), before + 2);
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_engine_error_toasts_once() {
    let mut rt = make_test_runtime();

    rt.update(|s, _| s.toasts.clear());
    let res = crate::ui::ops::post_operation(
        &mut rt.state,
        "nonexistent-record",
        "fulfill",
        "REF-ERR",
        "Test error toast",
        "err-key-1",
    );
    assert!(res.is_err());
    let danger_count = rt.state.toasts.iter().filter(|t| t.tone == crate::app::Tone::Danger).count();
    assert_eq!(
        danger_count, 1,
        "expected exactly 1 danger toast from post_operation, got {:?}",
        rt.state.toasts
    );

    rt.update(|s, _| s.toasts.clear());
    let res = rt.state.create_party(crate::ui::resources::ResourceKind::Customer, "", "", "", "");
    assert!(res.is_err());
    let danger_count = rt.state.toasts.iter().filter(|t| t.tone == crate::app::Tone::Danger).count();
    assert_eq!(
        danger_count, 1,
        "expected exactly 1 danger toast from create_party, got {:?}",
        rt.state.toasts
    );
}

#[test]
fn test_reverse_requires_confirmation() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2);

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    rt.state.accounting.debit_account = accounts[0].0.clone();
    rt.state.accounting.credit_account = accounts[1].0.clone();
    type_into(&mut rt, "journal-amount", "15000");
    type_into(&mut rt, "journal-reason", "Confirm reverse test");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "journal post"));

    let after_post_count = count(&rt, "journal_count");
    let journal_id = rt
        .state
        .books
        .list("journals")
        .into_iter()
        .find(|j| data::s(&j["reason"]) == "Confirm reverse test")
        .map(|j| data::s(&j["id"]))
        .unwrap_or_else(|| panic!("{}", blow(&rt, "journal id")));

    click(&mut rt, &format!("journal-{journal_id}"));
    click(&mut rt, "btn-Reverse");
    assert!(rt.state.confirm.is_some(), "reverse must open confirmation dialog");
    assert_eq!(
        count(&rt, "journal_count"),
        after_post_count,
        "journal count unchanged before confirm"
    );

    click(&mut rt, "btn-Reverse entry");
    assert!(rt.state.confirm.is_none());
    assert_eq!(count(&rt, "journal_count"), after_post_count + 1, "reversal posted after confirm");
    assert_no_danger(&rt);
    books_validate(&rt);
}
