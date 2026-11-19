use super::harness::{blow, click};
use crate::app::{Page, State};
use crate::{data, ui};
use rune::{Env, Runtime};

#[test]
fn test_first_run_is_a_welcome_then_the_books() {
    let dir = tempfile::tempdir().unwrap();
    let env = Env {
        system_dark: true,
        scale: 2.0,
    };
    let mut rt = Runtime::new(
        State::with_engine(data::Engine::with_dir(dir.path()), true),
        ui::root_view,
        (1180.0, 780.0),
        env,
    );
    rt.frame(0.0);
    assert!(rt.find("setup-welcome").is_some());
    assert!(rt.find("sidebar-brand").is_none());
    click(&mut rt, "btn-Get started");
    assert!(rt.find("setup-screen").is_some());
    click(&mut rt, "btn-Back");
    assert!(rt.find("setup-welcome").is_some());
    click(&mut rt, "btn-Get started");
    rt.state.forms.set("setup-name", "Sarv Cafe");
    click(&mut rt, "btn-Create a company and fiscal period");
    assert!(rt.find("setup-opening").is_some(), "{}", blow(&rt, "opening"));
    click(&mut rt, "btn-Start fresh");
    assert_eq!(rt.state.page, Page::Overview, "{}", blow(&rt, "opened"));
    assert!(rt.state.books.initialized());
    assert!(rt.find("sidebar-brand").is_none());
    assert!(rt.find("sidebar-search").is_some());
    assert!(rt.find("org-selector").is_some());
}

#[test]
fn test_bad_opening_amount_does_not_open_empty_books() {
    let dir = tempfile::tempdir().unwrap();
    let env = Env {
        system_dark: true,
        scale: 2.0,
    };
    let mut rt = Runtime::new(
        State::with_engine(data::Engine::with_dir(dir.path()), true),
        ui::root_view,
        (1180.0, 780.0),
        env,
    );
    rt.frame(0.0);
    click(&mut rt, "btn-Get started");
    rt.state.forms.set("setup-name", "Sarv Cafe");
    click(&mut rt, "btn-Create a company and fiscal period");
    assert!(rt.find("setup-opening").is_some(), "{}", blow(&rt, "opening"));

    rt.state.forms.set("opening-cash", "12.345.6");
    click(&mut rt, "btn-Start fresh");
    assert!(!rt.state.books.initialized(), "bad opening amount must not initialize books");
    assert!(
        rt.state.toasts.iter().any(|t| t.tone == crate::app::Tone::Danger),
        "expected Danger toast for invalid opening amount"
    );
}

#[test]
fn test_opening_cash_plus_bank_overflow_is_rejected_cleanly() {
    let dir = std::env::temp_dir().join(format!("sarv-overflow-open-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let env = Env {
        system_dark: true,
        scale: 2.0,
    };
    let mut rt = Runtime::new(
        State::with_engine(data::Engine::with_dir(&dir), true),
        ui::root_view,
        (1180.0, 780.0),
        env,
    );
    rt.frame(0.0);
    click(&mut rt, "btn-Get started");
    rt.state.forms.set("setup-name", "Overflow Co");
    click(&mut rt, "btn-Create a company and fiscal period");
    assert!(rt.find("setup-opening").is_some(), "{}", blow(&rt, "opening"));

    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    rt.state.forms.set("opening-cash", &i64::MAX.to_string());
    rt.state.forms.set("opening-bank", "1");
    click(&mut rt, "btn-Start fresh");
    assert!(!rt.state.books.initialized(), "overflow must not initialize books");
    assert!(!rt.state.setup.opening_posting, "opening_posting must reset on error");
    assert!(
        rt.state.toasts.iter().any(|t| t.tone == crate::app::Tone::Danger),
        "expected Danger toast for cash + bank overflow"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_setup_default_fiscal_year_uses_sarv_today() {
    if std::env::var_os("SARV_TODAY").is_none() {
        // Matches Engine::demo()'s pinned clock so parallel tests stay deterministic.
        unsafe { std::env::set_var("SARV_TODAY", "2026-10-08") };
    }
    let expected_today = chrono::NaiveDate::parse_from_str(&data::today(), "%Y-%m-%d").unwrap();
    assert_eq!(crate::ui::views::setup::today_naive(), expected_today);

    let dir = std::env::temp_dir().join(format!("sarv-setup-today-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let env = Env {
        system_dark: true,
        scale: 2.0,
    };
    let mut rt = Runtime::new(
        State::with_engine(data::Engine::with_dir(&dir), true),
        ui::root_view,
        (1180.0, 780.0),
        env,
    );
    rt.frame(0.0);
    click(&mut rt, "btn-Get started");
    rt.state.forms.set("setup-name", "Sarv Clock Co");
    let expected_fiscal = crate::ui::views::setup::fiscal_period(rt.state.fmt.calendar, expected_today);
    click(&mut rt, "btn-Create a company and fiscal period");
    assert!(rt.find("setup-opening").is_some(), "{}", blow(&rt, "opening"));
    assert_eq!(rt.state.books.period_name(), expected_fiscal.0);
    assert_eq!(rt.state.books.range(), (expected_fiscal.1, expected_fiscal.2));
    let _ = std::fs::remove_dir_all(&dir);
}
