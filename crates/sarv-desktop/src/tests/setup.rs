use super::harness::{blow, click};
use crate::app::{Page, State};
use crate::{data, ui};
use rune::{Env, Runtime};

#[test]
fn test_first_run_is_a_welcome_then_the_books() {
    let dir = std::env::temp_dir().join(format!("sarv-fresh-{}", std::process::id()));
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
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_bad_opening_amount_does_not_open_empty_books() {
    let dir = std::env::temp_dir().join(format!("sarv-bad-open-{}", std::process::id()));
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
    let _ = std::fs::remove_dir_all(&dir);
}
