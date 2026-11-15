use super::harness::{assert_no_danger, blow, books_validate, click, count, make_test_runtime, type_into};
use crate::app::{Page, State};
use crate::data;

#[test]
fn test_settings_segmented_controls() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Settings.key());
    assert!(rt.find("settings-tabs").is_some());
    assert!(rt.find("settings-theme").is_some());
    assert!(rt.find("settings-lang").is_some());
    assert!(rt.find("settings-theme-switch").is_some());
}

#[test]
fn test_export_writes_csv_and_language_persists() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Reports.key());
    click(&mut rt, "btn-Export CSV");
    let exports = rt.state.engine.dir.join("exports");
    let mut names = std::fs::read_dir(&exports)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<Vec<_>>();
    names.sort();
    assert!(
        names
            .iter()
            .any(|n| n.to_string_lossy().contains("csv") || n.to_string_lossy().ends_with(".csv")),
        "{names:?}"
    );
    let body = std::fs::read_to_string(exports.join(names[0].clone())).unwrap();
    assert!(body.contains(',') || body.starts_with('\u{feff}'), "csv content");

    rt.state.set_lang(crate::i18n::Lang::En);
    click(&mut rt, Page::Settings.key());
    click(&mut rt, "btn-Save preferences");
    let dir = rt.state.engine.dir.clone();
    let again = State::with_engine(data::Engine::with_dir(&dir), true);
    assert_eq!(again.lang(), crate::i18n::Lang::En);
}

#[test]
fn test_backup_and_law_notice() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Settings.key());
    click(&mut rt, "btn-Backup");
    let backups = std::fs::read_dir(&rt.state.engine.dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().starts_with("sarv-backup-"));
    assert!(backups, "backup file written");
    click(&mut rt, "btn-Law data");
    assert!(!rt.state.settings.law_notice.is_empty(), "law notice is shown, not posted");
}

#[test]
fn backup_then_restore_returns_the_books_to_the_snapshot() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let pre_count = count(&rt, "journal_count");

    click(&mut rt, Page::Settings.key());
    click(&mut rt, "btn-Backup");

    let accounts = crate::ui::pickers::account_options(&rt.state);
    assert!(accounts.len() >= 2);
    click(&mut rt, Page::Accounting.key());
    click(&mut rt, "btn-New journal");
    rt.state.accounting.set_line_account(0, accounts[0].0.clone());
    rt.state.accounting.set_line_account(1, accounts[1].0.clone());
    type_into(&mut rt, "journal-debit-0", "12000");
    type_into(&mut rt, "journal-credit-1", "12000");
    type_into(&mut rt, "journal-reason", "Before restore");
    click(&mut rt, "btn-Save");
    assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "post journal"));
    assert_eq!(count(&rt, "journal_count"), pre_count + 1);

    click(&mut rt, Page::Settings.key());
    click(&mut rt, "btn-Restore backup");
    let filename = rt.state.settings.backups["backups"]
        .as_array()
        .and_then(|rows| rows.first())
        .map(|row| data::s(&row["filename"]))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| panic!("{}", blow(&rt, "backup list")));
    click(&mut rt, &format!("btn-backup-{filename}"));
    click(&mut rt, "btn-restore-confirm");
    assert!(!rt.state.settings.restore_open, "{}", blow(&rt, "restore"));

    assert_eq!(count(&rt, "journal_count"), pre_count, "restore must return books to snapshot");
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_recovery_screen_restores_a_backup() {
    let mut rt = make_test_runtime();
    let pre_count = count(&rt, "journal_count");

    let created = rt.state.run("backup_create", serde_json::json!({}), None).expect("create backup");
    let filename = data::s(&created["filename"]);
    assert!(!filename.is_empty(), "backup filename should be returned");

    let dir = rt.state.engine.dir.clone();
    drop(rt);
    sarv_quote::store::Store::corrupt_for_test(&dir).expect("corrupt db");

    let env = rune::Env {
        system_dark: true,
        scale: 2.0,
    };
    let mut rt = rune::Runtime::new(
        State::with_engine(data::Engine::with_dir(&dir), true),
        crate::ui::root_view,
        (1180.0, 780.0),
        env,
    );
    rt.frame(0.0);

    assert!(rt.state.books.load_failed(), "books should report load_failed");
    assert!(rt.find("recovery-screen").is_some(), "recovery screen should render");

    click(&mut rt, &format!("btn-backup-{filename}"));
    assert!(!rt.state.recovery.preview_note.is_empty(), "{}", blow(&rt, "preview backup"));
    click(&mut rt, "btn-Restore this backup");

    assert!(!rt.state.books.load_failed(), "{}", blow(&rt, "recover from backup"));
    assert!(
        rt.find("recovery-screen").is_none(),
        "recovery screen should disappear after restore"
    );
    assert_eq!(count(&rt, "journal_count"), pre_count);
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_theme_toggle_persists_across_restart() {
    let mut rt = make_test_runtime();
    let initial_dark = rt.state.dark;
    let initial_lang = rt.state.lang();

    click(&mut rt, "theme-toggle");
    click(&mut rt, "lang-toggle");
    assert_eq!(rt.state.dark, !initial_dark);
    assert_eq!(rt.state.lang(), initial_lang.other());
    assert_no_danger(&rt);

    let dir = rt.state.engine.dir.clone();
    let again = State::with_engine(data::Engine::with_dir(&dir), true);
    assert_eq!(again.dark, !initial_dark, "theme toggle must persist across restart");
    assert_eq!(again.lang(), initial_lang.other(), "language toggle must persist across restart");
}

#[test]
fn test_language_switch_keeps_digit_choice() {
    let mut rt = make_test_runtime();
    assert_eq!(rt.state.lang(), crate::i18n::Lang::Fa);
    assert!(rt.state.fmt.persian_digits);

    // Navigate to Settings first, then switch language to English via sidebar toggle; Persian digits must stay on.
    click(&mut rt, Page::Settings.key());
    click(&mut rt, "lang-toggle");
    assert_eq!(rt.state.lang(), crate::i18n::Lang::En);
    assert!(rt.state.fmt.persian_digits, "switching language must not overwrite persian_digits");

    // Turn off Persian digits in Settings, then switch back to Persian; Latin digits must stay.
    click(&mut rt, "settings-digits-switch");
    assert!(!rt.state.fmt.persian_digits);

    click(&mut rt, "lang-toggle");
    assert_eq!(rt.state.lang(), crate::i18n::Lang::Fa);
    assert!(!rt.state.fmt.persian_digits, "switching back to Persian must keep Latin digits");
    assert_no_danger(&rt);

    let dir = rt.state.engine.dir.clone();
    let again = State::with_engine(data::Engine::with_dir(&dir), true);
    assert_eq!(again.lang(), crate::i18n::Lang::Fa);
    assert!(!again.fmt.persian_digits);
}

#[test]
fn test_add_second_company_from_settings() {
    let mut rt = make_test_runtime();
    let first_id = rt.state.books.company_id.clone();

    click(&mut rt, Page::Settings.key());
    assert!(rt.find("company-switch").is_some(), "company list must show even with one company");
    click(&mut rt, "btn-Add company");
    assert!(rt.find("setup-screen").is_some(), "{}", blow(&rt, "setup-screen"));
    assert!(rt.find("sidebar-search").is_none(), "sidebar must hide while adding company");
    assert!(rt.find("btn-Cancel").is_some());

    type_into(&mut rt, "setup-name", "Second Business");
    click(&mut rt, "btn-Create a company and fiscal period");
    assert!(rt.find("setup-opening").is_some(), "{}", blow(&rt, "setup-opening"));

    click(&mut rt, "btn-Start fresh");
    assert_eq!(rt.state.books.companies().len(), 2);
    assert_ne!(rt.state.books.company_id, first_id);
    assert!(rt.state.books.initialized());
    assert!(!rt.state.setup.adding_company);
    assert!(rt.find("sidebar-search").is_some(), "sidebar must return after opening");
    assert_no_danger(&rt);
}

#[test]
fn test_switch_company_from_sidebar() {
    let mut rt = make_test_runtime();
    let first_id = rt.state.books.company_id.clone();
    let first_sales = count(&rt, "sales_count");

    click(&mut rt, Page::Settings.key());
    click(&mut rt, "btn-Add company");
    type_into(&mut rt, "setup-name", "Branch Two");
    click(&mut rt, "btn-Create a company and fiscal period");
    click(&mut rt, "btn-Start fresh");
    let second_id = rt.state.books.company_id.clone();
    assert_ne!(second_id, first_id);
    assert_eq!(count(&rt, "sales_count"), 0);

    click(&mut rt, "org-selector");
    assert!(rt.find("company-switch-dialog").is_some(), "{}", blow(&rt, "company-switch-dialog"));
    click(&mut rt, &format!("switch-company-{first_id}"));
    assert!(!rt.state.company_switch_open);
    assert_eq!(rt.state.books.company_id, first_id);
    assert_eq!(count(&rt, "sales_count"), first_sales);
    assert_no_danger(&rt);
}

#[test]
fn test_cancel_adding_company_returns_to_books() {
    let mut rt = make_test_runtime();
    let first_id = rt.state.books.company_id.clone();

    click(&mut rt, Page::Settings.key());
    click(&mut rt, "btn-Add company");
    assert!(rt.find("setup-screen").is_some());
    assert!(rt.find("sidebar-search").is_none());

    click(&mut rt, "btn-Cancel");
    assert!(!rt.state.setup.adding_company);
    assert_eq!(rt.state.books.company_id, first_id);
    assert!(rt.find("sidebar-search").is_some(), "sidebar must return on cancel");
    assert_no_danger(&rt);
}
