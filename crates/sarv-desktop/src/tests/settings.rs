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
    rt.state.accounting.debit_account = accounts[0].0.clone();
    rt.state.accounting.credit_account = accounts[1].0.clone();
    type_into(&mut rt, "journal-amount", "12000");
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
