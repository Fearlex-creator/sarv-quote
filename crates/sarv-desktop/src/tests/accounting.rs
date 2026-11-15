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
    rt.state.accounting.set_line_account(0, debit);
    rt.state.accounting.set_line_account(1, credit);
    rt.state.forms.set("journal-debit-0", "1000");
    rt.state.forms.set("journal-credit-1", "1000");
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
    rt.state.accounting.set_line_account(0, debit.clone());
    rt.state.accounting.set_line_account(1, credit.clone());
    type_into(&mut rt, "journal-debit-0", "15000");
    type_into(&mut rt, "journal-credit-1", "15000");
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
    rt.state.accounting.set_line_account(0, debit);
    rt.state.accounting.set_line_account(1, credit);
    type_into(&mut rt, "journal-debit-0", "15000");
    type_into(&mut rt, "journal-credit-1", "15000");
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
    rt.state.accounting.set_line_account(0, debit.clone());
    rt.state.accounting.set_line_account(1, credit.clone());
    type_into(&mut rt, "journal-debit-0", "25000");
    type_into(&mut rt, "journal-credit-1", "25000");
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
    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    type_into(&mut rt, "journal-debit-0", "10000");
    type_into(&mut rt, "journal-credit-1", "10000");
    type_into(&mut rt, "journal-reason", "First journal");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "first journal"));

    click(&mut rt, Page::Overview.key());
    click(&mut rt, "btn-New journal");
    assert!(rt.state.accounting.modal_open, "{}", blow(&rt, "overview new journal"));
    rt.state.accounting.set_line_account(0, accounts[1].0.clone());
    rt.state.accounting.set_line_account(1, accounts[2].0.clone());
    type_into(&mut rt, "journal-debit-0", "20000");
    type_into(&mut rt, "journal-credit-1", "20000");
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
        &crate::data::today(),
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
    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    type_into(&mut rt, "journal-debit-0", "15000");
    type_into(&mut rt, "journal-credit-1", "15000");
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

#[test]
fn test_journal_posts_on_the_typed_jalali_date() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2);

    let (start, _) = rt.state.books.range();
    let target_iso = chrono::NaiveDate::parse_from_str(&start, "%Y-%m-%d")
        .unwrap()
        .checked_add_days(chrono::Days::new(3))
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
    let typed_jalali = rt.state.fmt.date_input(&target_iso);

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    type_into(&mut rt, "journal-debit-0", "25000");
    type_into(&mut rt, "journal-credit-1", "25000");
    type_into(&mut rt, "journal-date", &typed_jalali);
    type_into(&mut rt, "journal-reason", "Jalali dated journal");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "jalali journal post"));

    let posted = rt
        .state
        .books
        .list("journals")
        .into_iter()
        .find(|j| data::s(&j["reason"]) == "Jalali dated journal")
        .unwrap_or_else(|| panic!("{}", blow(&rt, "find jalali journal")));
    assert_eq!(data::s(&posted["date"]), target_iso);
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_invalid_date_blocks_the_post() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2);
    let before = rt.state.books.list("journals").len();

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    type_into(&mut rt, "journal-debit-0", "25000");
    type_into(&mut rt, "journal-credit-1", "25000");
    type_into(&mut rt, "journal-date", "1405/13/99");
    type_into(&mut rt, "journal-reason", "Should not post");
    click(&mut rt, "btn-Save");

    assert!(rt.state.accounting.modal_open, "invalid date must keep modal open");
    assert!(!rt.state.accounting.posting, "posting flag must reset after invalid date");
    assert!(
        rt.state.toasts.iter().any(|t| t.tone == crate::app::Tone::Danger),
        "invalid date must show a danger toast"
    );
    assert_eq!(rt.state.books.list("journals").len(), before);
}

#[test]
fn test_accounting_coa_sorts_by_code() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Accounting.key());
    rt.state.accounting.tab_idx = 1;
    rt.state.accounting.rows_stamp = 0;
    crate::ui::views::accounting::ensure(&mut rt.state);
    rt.frame(1.0 / 60.0);
    assert!(rt.state.accounting.rows.len() >= 2);

    crate::ui::views::accounting::apply_sort(&mut rt.state, "code", rune_kit::SortDir::Descending);
    rt.frame(1.0 / 60.0);
    let codes: Vec<&str> = rt.state.accounting.rows.iter().map(|r| r.cells[0].as_str()).collect();
    assert!(
        codes.windows(2).all(|w| w[0].to_lowercase() >= w[1].to_lowercase()),
        "expected descending codes: {codes:?}"
    );

    crate::ui::views::accounting::apply_sort(&mut rt.state, "code", rune_kit::SortDir::Ascending);
    rt.frame(1.0 / 60.0);
    let codes: Vec<&str> = rt.state.accounting.rows.iter().map(|r| r.cells[0].as_str()).collect();
    assert!(
        codes.windows(2).all(|w| w[0].to_lowercase() <= w[1].to_lowercase()),
        "expected ascending codes: {codes:?}"
    );
}

#[test]
fn test_open_next_fiscal_year_and_post_in_it() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2);
    let initial_periods = rt.state.books.workspace["periods"].as_array().map(|a| a.len()).unwrap_or(0);
    assert_eq!(initial_periods, 1);

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New fiscal year");
    assert!(rt.find("period-add-dialog").is_some(), "period-add dialog must open");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.period_add_open, "{}", blow(&rt, "open next fiscal year"));

    let periods = rt.state.books.workspace["periods"].as_array().expect("periods array");
    assert_eq!(periods.len(), 2, "expected two periods after opening next fiscal year");

    let new_period_id = rt.state.books.period_id.clone();
    let (new_start, _) = rt.state.books.range();
    let target_iso = chrono::NaiveDate::parse_from_str(&new_start, "%Y-%m-%d")
        .unwrap()
        .checked_add_days(chrono::Days::new(2))
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
    let typed_date = rt.state.fmt.date_input(&target_iso);

    click(&mut rt, "btn-New journal");
    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    type_into(&mut rt, "journal-debit-0", "30000");
    type_into(&mut rt, "journal-credit-1", "30000");
    type_into(&mut rt, "journal-date", &typed_date);
    type_into(&mut rt, "journal-reason", "New fiscal year journal");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "post in new fiscal year"));

    let posted = rt
        .state
        .books
        .list("journals")
        .into_iter()
        .find(|j| data::s(&j["reason"]) == "New fiscal year journal")
        .unwrap_or_else(|| panic!("{}", blow(&rt, "find new year journal")));
    assert_eq!(data::s(&posted["period_id"]), new_period_id);
    assert_eq!(data::s(&posted["date"]), target_iso);
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_switching_fiscal_year_changes_the_report_range() {
    let mut rt = make_test_runtime();
    let first_period_id = rt.state.books.period_id.clone();
    let first_range = rt.state.books.range();

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New fiscal year");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.period_add_open, "{}", blow(&rt, "open second year"));

    let second_period_id = rt.state.books.period_id.clone();
    assert_ne!(second_period_id, first_period_id);
    let second_range = rt.state.books.range();
    assert_ne!(second_range, first_range);

    click(&mut rt, "fiscal-year-switch");
    assert!(rt.find("period-switch-dialog").is_some(), "period switcher dialog must open");
    click(&mut rt, &format!("switch-period-{first_period_id}"));
    assert_eq!(rt.state.books.period_id, first_period_id);
    assert_eq!(rt.state.books.range(), first_range);

    click(&mut rt, "fiscal-year-switch");
    click(&mut rt, &format!("switch-period-{second_period_id}"));
    assert_eq!(rt.state.books.period_id, second_period_id);
    assert_eq!(rt.state.books.range(), second_range);
    assert_no_danger(&rt);
}

#[test]
fn test_new_expense_account_appears_in_chart_and_pickers() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "segment1");
    for _ in 0..4 {
        rt.frame(1.0 / 60.0);
    }

    click(&mut rt, "btn-New account");
    assert!(rt.find("acct-dialog").is_some(), "new account dialog must open");

    click(&mut rt, "segment4"); // Expense
    type_into(&mut rt, "acct-code", "5910");
    type_into(&mut rt, "acct-name", "Office Rent");
    click(&mut rt, "btn-Save");

    assert!(!rt.state.accounting.acct_open, "{}", blow(&rt, "new account save"));
    let rent = rt.state.books.workspace["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| data::s(&a["code"]) == "5910" && data::s(&a["name"]) == "Office Rent")
        .expect("Office Rent account must be in workspace.accounts");
    let rent_id = data::s(&rent["id"]);
    assert_eq!(data::s(&rent["classification"]), "Expense");
    assert_eq!(data::s(&rent["normal_balance"]), "Debit");
    assert!(
        rt.find(&format!("acc-{rent_id}")).is_some(),
        "new account must appear in chart table"
    );

    let picker_ids: Vec<String> = crate::ui::pickers::account_options(&rt.state)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert!(picker_ids.contains(&rent_id), "new account must appear in account pickers");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_archived_account_leaves_the_pickers() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "segment1");
    for _ in 0..4 {
        rt.frame(1.0 / 60.0);
    }

    click(&mut rt, "btn-New account");
    click(&mut rt, "segment4"); // Expense
    type_into(&mut rt, "acct-code", "5920");
    type_into(&mut rt, "acct-name", "Temporary Expense");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.acct_open, "{}", blow(&rt, "create temp account"));

    let temp_id = rt.state.books.workspace["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| data::s(&a["code"]) == "5920")
        .map(|a| data::s(&a["id"]))
        .unwrap();

    assert!(
        crate::ui::pickers::account_options(&rt.state).iter().any(|(id, _)| id == &temp_id),
        "new account should be in pickers before archiving"
    );

    click(&mut rt, &format!("acc-{temp_id}"));
    click(&mut rt, "btn-Archive account");
    assert!(rt.find("acct-archive-dialog").is_some(), "archive dialog must open");
    type_into(&mut rt, "acct-archive-reason", "No longer used");
    click(&mut rt, "btn-Archive");

    assert!(!rt.state.accounting.acct_archive_open, "{}", blow(&rt, "archive account"));
    assert!(
        !crate::ui::pickers::account_options(&rt.state).iter().any(|(id, _)| id == &temp_id),
        "archived account must leave account pickers"
    );
    assert!(
        rt.find(&format!("acc-{temp_id}")).is_none(),
        "archived account is hidden when Show archived is off"
    );

    click(&mut rt, "acct-show-archived");
    assert!(rt.state.accounting.acct_show_archived);
    assert!(
        rt.find(&format!("acc-{temp_id}")).is_some(),
        "archived account appears when Show archived is toggled on"
    );
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_duplicate_account_code_shows_one_error_toast() {
    let mut rt = make_test_runtime();
    let existing_code = data::s(&rt.state.books.workspace["accounts"][0]["code"]);

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "segment1");
    for _ in 0..4 {
        rt.frame(1.0 / 60.0);
    }

    click(&mut rt, "btn-New account");
    type_into(&mut rt, "acct-code", &existing_code);
    type_into(&mut rt, "acct-name", "Duplicate Code Account");
    click(&mut rt, "btn-Save");

    assert!(rt.state.accounting.acct_open, "dialog stays open on validation error");
    assert_eq!(danger_toasts(&rt).len(), 1, "expected exactly one danger toast");
}

#[test]
fn test_load_older_appends_records() {
    let mut rt = make_test_runtime();
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2, "need two postable accounts");
    let debit = accounts[0].0.clone();
    let credit = accounts[1].0.clone();
    let company_id = rt.state.books.company_id.clone();
    let period_id = rt.state.books.period_id.clone();
    let date = data::today();

    for i in 0..105 {
        let payload = serde_json::json!({
            "company_id": company_id,
            "period_id": period_id,
            "date": date,
            "source_id": format!("paging-{i}"),
            "reason": format!("Paging journal {i}"),
            "request_key": format!("paging-req-{i}"),
            "lines": [
                { "account_id": debit, "debit": "100", "credit": "0", "description": "D" },
                { "account_id": credit, "debit": "0", "credit": "100", "description": "C" },
            ],
        });
        rt.state.engine.call("journal_post", payload).expect("seed journal_post");
    }
    rt.state.books.refresh(&rt.state.engine);
    rt.state.stamp += 1;

    click(&mut rt, Page::Accounting.key());
    assert_eq!(rt.state.books.list("journals").len(), 100);
    assert!(rt.find("journals-load-more").is_some(), "footer must be visible when >100 journals");
    assert!(rt.find("btn-Load older").is_some());

    click(&mut rt, "btn-Load older");
    assert_no_danger(&rt);
    assert!(rt.state.books.list("journals").len() > 100, "load older must append journals");
    assert!(
        rt.state.accounting.rows.len() > 100,
        "cached accounting rows must rebuild after load older"
    );
}

#[test]
fn test_invoice_detail_shows_lines_and_journal() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.fmt.unit = crate::i18n::Unit::Toman;
    rt.state.sales.customer_id = "CU-A".into();
    rt.state.sales.pay_method = 0;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.forms.set("sale-qty-0", "1");
    rt.state.forms.set("sale-price-0", "10000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "complete sale"));

    let sale_id = rt
        .state
        .books
        .list("sales")
        .into_iter()
        .find(|r| r["command"].get("Fulfill").is_some() || r["data"].get("Fulfill").is_some())
        .map(|r| data::s(&r["id"]))
        .expect("completed sale should produce a fulfilled invoice");

    rt.state.sales.selected = sale_id;
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Open");

    assert_no_danger(&rt);
    assert!(rt.state.detail.open, "detail dialog must open");
    assert!(rt.find("detail-dialog").is_some());
    assert!(rt.find("detail-lines").is_some(), "invoice detail must show lines");
    assert!(rt.find("detail-journal").is_some(), "invoice detail must show journal");
}

#[test]
fn test_ledger_shows_running_balance() {
    let mut rt = make_test_runtime();
    let acc_id = rt.state.books.report["trial"]["rows"]
        .as_array()
        .and_then(|rows| rows.first())
        .map(|r| data::s(&r["account_id"]))
        .expect("demo fixture trial balance should have at least one posted account");

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "segment1");
    rt.state.accounting.selected = format!("acc-{acc_id}");
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Ledger");

    assert_no_danger(&rt);
    assert!(rt.state.accounting.ledger_open);
    assert!(rt.find("ledger-dialog").is_some());
    assert!(rt.find("ledger-opening-row").is_some());
    assert!(rt.find("ledger-table").is_some());
    assert!(!rt.state.accounting.ledger_rows.is_empty());
    assert!(
        !data::s(&rt.state.accounting.ledger_rows[0]["balance"]).is_empty(),
        "ledger row must include running balance"
    );
}

#[test]
fn test_trial_balance_row_opens_the_ledger() {
    let mut rt = make_test_runtime();
    let acc_id = rt.state.books.report["trial"]["rows"]
        .as_array()
        .and_then(|rows| rows.first())
        .map(|r| data::s(&r["account_id"]))
        .expect("demo fixture trial balance should have an account");

    click(&mut rt, Page::Reports.key());
    click(&mut rt, "segment1");
    rt.state.reports.selected_account_id = acc_id;
    rt.frame(1.0 / 60.0);
    click(&mut rt, "btn-Ledger");

    assert_no_danger(&rt);
    assert!(rt.state.accounting.ledger_open);
    assert!(rt.find("ledger-dialog").is_some());
}

#[test]
fn test_three_line_journal_posts_balanced() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 3, "need three postable accounts");
    let before = count(&rt, "journal_count");

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    click(&mut rt, "btn-Add line");
    assert_eq!(rt.state.accounting.lines.len(), 3);

    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    rt.state.accounting.set_line_account(2, accounts[2].0.clone());
    type_into(&mut rt, "journal-debit-0", "300");
    type_into(&mut rt, "journal-credit-1", "100");
    type_into(&mut rt, "journal-credit-2", "200");
    type_into(&mut rt, "journal-reason", "Split three-line journal");
    click(&mut rt, "btn-Save");

    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "three line journal"));
    assert_eq!(count(&rt, "journal_count"), before + 1);
    let posted = rt
        .state
        .books
        .list("journals")
        .into_iter()
        .find(|j| data::s(&j["reason"]) == "Split three-line journal")
        .unwrap_or_else(|| panic!("{}", blow(&rt, "find three line journal")));
    let lines = posted["lines"].as_array().expect("journal lines");
    assert_eq!(lines.len(), 3, "stored journal must have 3 lines");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_unbalanced_journal_is_blocked_with_one_toast() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2);
    let before = count(&rt, "journal_count");

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    type_into(&mut rt, "journal-debit-0", "1000");
    type_into(&mut rt, "journal-credit-1", "500");
    rt.update(|s, _| s.toasts.clear());
    click(&mut rt, "btn-Save");

    assert!(rt.state.accounting.modal_open, "unbalanced journal must keep dialog open");
    assert!(!rt.state.accounting.posting, "posting flag must reset");
    assert_eq!(count(&rt, "journal_count"), before);
    let dangers = danger_toasts(&rt);
    assert_eq!(dangers.len(), 1, "expected 1 danger toast, got {dangers:?}");
}

#[test]
fn test_removing_a_journal_line_shifts_the_fields() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 3);

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    click(&mut rt, "btn-Add line");
    assert_eq!(rt.state.accounting.lines.len(), 3);

    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    rt.state.accounting.set_line_account(2, accounts[2].0.clone());
    type_into(&mut rt, "journal-debit-0", "100");
    type_into(&mut rt, "journal-desc-0", "first");
    type_into(&mut rt, "journal-credit-1", "50");
    type_into(&mut rt, "journal-desc-1", "middle");
    type_into(&mut rt, "journal-credit-2", "100");
    type_into(&mut rt, "journal-desc-2", "last");

    click(&mut rt, "btn-journal-remove-1");
    assert_eq!(rt.state.accounting.lines.len(), 2);
    assert_eq!(rt.state.accounting.lines[0].account_id, accounts[0].0);
    assert_eq!(rt.state.accounting.lines[1].account_id, accounts[2].0);
    assert_eq!(rt.state.forms.get("journal-credit-1"), "100");
    assert_eq!(rt.state.forms.get("journal-desc-1"), "last");
    assert_eq!(rt.state.forms.get("journal-credit-2"), "");
    assert_eq!(rt.state.forms.get("journal-desc-2"), "");
}

#[test]
fn test_two_line_journal_mirrors_the_amount() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2);
    let before = count(&rt, "journal_count");

    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    type_into(&mut rt, "journal-debit-0", "4500");
    type_into(&mut rt, "journal-reason", "Mirrored two-line journal");
    click(&mut rt, "btn-Save");

    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "mirrored two line journal"));
    assert_eq!(count(&rt, "journal_count"), before + 1);
    assert_no_danger(&rt);
    books_validate(&rt);
}
