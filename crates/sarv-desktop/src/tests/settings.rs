use super::harness::{click, make_test_runtime};
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
