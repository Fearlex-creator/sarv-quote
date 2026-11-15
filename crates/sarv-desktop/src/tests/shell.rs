use super::harness::{click, make_test_runtime};
use crate::app::{Page, State};
use crate::{data, handle_key, i18n, ui};
use rune::{Env, Runtime};

#[test]
fn test_initial_dashboard_view_renders_cleanly() {
    let mut rt = make_test_runtime();
    assert_eq!(rt.state.page, Page::Overview);
    assert!(rt.state.sidebar_open());
    assert!(rt.find("sidebar-brand").is_none());
    assert!(rt.find("sidebar-search").is_some());
    assert!(rt.find("toolbar").is_some());
    assert!(rt.find("sidebar-toggle").is_some());
    assert!(rt.find("quick-actions").is_some());
    assert!(rt.find("overview-metrics").is_some());
    assert!(rt.find("metric-cash").is_some());
    assert!(rt.find("btn-New sales invoice").is_some());
    assert!(rt.find("overview-summary").is_some());
    assert!(rt.find("setup-guide-card").is_some());
    assert!(rt.find("recent-entries-panel").is_some());
    if rt.find("setup-step-profile").is_none() {
        click(&mut rt, "btn-Setup guide");
    }
    assert!(rt.find("setup-step-profile").is_some());
    assert!(rt.find("setup-step-opening").is_some());
    assert!(rt.find("setup-step-warehouses").is_some());
    assert!(rt.find("setup-step-products").is_some());
    assert!(rt.find("setup-step-contacts").is_some());
    let (_, sale_y) = rt.find("btn-New sales invoice").unwrap();
    let (_, pay_y) = rt.find("btn-Record a payment").unwrap();
    assert!(
        (sale_y - pay_y).abs() < 12.0,
        "actions should share one row at 1180: {sale_y} {pay_y}"
    );
    let (_, cash_y) = rt.find("metric-cash").unwrap();
    let (_, profit_y) = rt.find("metric-profit").unwrap();
    assert!(
        (cash_y - profit_y).abs() < 12.0,
        "figures should share one row at 1180: {cash_y} {profit_y}"
    );
}

#[test]
fn test_overview_actions_wrap_in_a_narrow_window() {
    let env = Env {
        system_dark: true,
        scale: 2.0,
    };
    let mut rt = Runtime::new(State::with_engine(data::Engine::demo(), true), ui::root_view, (720.0, 780.0), env);
    rt.frame(0.0);
    let (_, sale_y) = rt.find("btn-New sales invoice").unwrap();
    let (_, pay_y) = rt.find("btn-Record a payment").unwrap();
    assert!(pay_y > sale_y + 16.0, "actions should wrap at 720: {sale_y} {pay_y}");
    assert!(rt.find("metric-profit").is_some());
    assert!(rt.find("overview-summary").is_some());
}

#[test]
fn test_finished_setup_guide_unmounts_in_auto_mode() {
    use crate::ui::views::overview::GuideMode;
    use serde_json::json;

    let mut rt = make_test_runtime();
    assert!(rt.find("setup-guide-card").is_some(), "demo fixture stays at 4 of 5");

    // Force all five guide predicates without posting real books.
    rt.state.books.workspace["initialized"] = json!(true);
    rt.state.books.workspace["business_profile"]["phone"] = json!("02100000000");
    if rt.state.books.workspace["resources"]["warehouses"]
        .as_array()
        .map(|a| a.is_empty())
        .unwrap_or(true)
    {
        rt.state.books.workspace["resources"]["warehouses"] = json!([{ "id": "w1", "name": "Main" }]);
    }
    if rt.state.books.workspace["resources"]["units"]
        .as_array()
        .map(|a| a.is_empty())
        .unwrap_or(true)
    {
        rt.state.books.workspace["resources"]["units"] = json!([{ "id": "u1", "name": "Piece" }]);
    }
    if rt.state.books.workspace["resources"]["items"]
        .as_array()
        .map(|a| a.is_empty())
        .unwrap_or(true)
    {
        rt.state.books.workspace["resources"]["items"] = json!([{ "id": "i1", "name": "Tea" }]);
    }
    if rt.state.books.workspace["resources"]["parties"]
        .as_array()
        .map(|a| a.is_empty())
        .unwrap_or(true)
    {
        rt.state.books.workspace["resources"]["parties"] = json!([{ "id": "p1", "name": "Ada", "customer": true, "supplier": false }]);
    }
    rt.state.books.workspace["sales_count"] = json!(1);
    rt.state.overview.guide = GuideMode::Auto;
    rt.frame(0.0);
    assert!(
        rt.find("setup-guide-card").is_none(),
        "Auto mode must unmount the guide once every step is done"
    );

    rt.state.overview.guide = GuideMode::Open;
    rt.frame(0.0);
    assert!(
        rt.find("setup-guide-card").is_some(),
        "Open still shows the finished guide when the user asks"
    );
}

#[test]
fn test_navigation_across_all_groups_headlessly() {
    let mut rt = make_test_runtime();
    for page in [
        Page::Sales,
        Page::Contacts,
        Page::Inventory,
        Page::Banking,
        Page::Settings,
        Page::Overview,
    ] {
        click(&mut rt, page.key());
        assert_eq!(rt.state.page, page);
    }
}

#[test]
fn test_sidebar_collapse_headlessly() {
    let mut rt = make_test_runtime();
    assert!(rt.state.sidebar_open());
    click(&mut rt, "sidebar-toggle");
    assert!(!rt.state.sidebar_open());
    // Let the pane finish closing so the toolbar rail lays out.
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    assert!(rt.find(&format!("rail-{}", Page::Sales.key())).is_some());
    click(&mut rt, &format!("rail-{}", Page::Inventory.key()));
    assert_eq!(rt.state.page, Page::Inventory);
    click(&mut rt, "sidebar-toggle");
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    assert!(rt.state.sidebar_open());
    assert!(rt.find(Page::Sales.key()).is_some());
}

#[test]
fn test_theme_toggle_headlessly() {
    let mut rt = make_test_runtime();
    assert!(rt.state.dark);
    click(&mut rt, "theme-toggle");
    assert!(!rt.state.dark, "Switched to light mode");
    click(&mut rt, "theme-toggle");
    assert!(rt.state.dark, "Switched back to dark mode");
}

#[test]
fn test_rtl_language_toggle() {
    let mut rt = make_test_runtime();
    assert_eq!(rt.state.lang(), i18n::Lang::Fa);
    assert!(rt.state.fmt.persian_digits);
    click(&mut rt, "lang-toggle");
    assert_eq!(rt.state.lang(), i18n::Lang::En);
}

#[test]
fn test_command_palette_and_section_screens() {
    let mut rt = make_test_runtime();
    click(&mut rt, "global-search");
    assert!(rt.state.palette_open);
    assert!(rt.find("palette-results").is_some());

    // Prefer a near-top palette row — scrolled-off rows can miss headless hit tests.
    click(&mut rt, &format!("palette-{}", Page::Sales.key()));
    assert_eq!(rt.state.page, Page::Sales);
    assert!(!rt.state.palette_open);
    assert!(rt.find("sales-screen").is_some());

    for page in [Page::Reports, Page::Purchases, Page::Payments, Page::Accounting, Page::Settings] {
        click(&mut rt, page.key());
        assert_eq!(rt.state.page, page);
    }
}

#[test]
fn test_all_pages_render_headlessly() {
    let mut rt = make_test_runtime();
    assert!(!rt.state.show_pos, "POS stays off until Settings turns it on");
    assert!(rt.find(Page::Pos.key()).is_none());
    for page in Page::ALL {
        if page == Page::Pos {
            rt.update(|s, _| {
                s.show_pos = true;
                s.nav(Page::Pos);
            });
            rt.frame(1.0 / 60.0);
            assert!(rt.find(Page::Pos.key()).is_some(), "POS appears under Sales once enabled");
        } else {
            click(&mut rt, page.key());
        }
        assert_eq!(rt.state.page, page);
        assert!(rt.find("toolbar").is_some());
        assert!(rt.find("sidebar").is_some());
    }
}

#[test]
fn test_keyboard_sidebar_and_theme() {
    let mut rt = make_test_runtime();
    rt.on_key = Some(handle_key);
    rt.key(rune::KeyEvent {
        key: rune::Key::Command('b'),
        mods: rune::Mods::default(),
    });
    rt.frame(1.0 / 60.0);
    assert!(!rt.state.sidebar_open());

    rt.key(rune::KeyEvent {
        key: rune::Key::Command('d'),
        mods: rune::Mods::default(),
    });
    rt.frame(1.0 / 60.0);
    assert!(!rt.state.dark);
}

#[test]
fn test_contacts_search_filters_the_table() {
    let mut rt = make_test_runtime();
    let party = rt.state.books.workspace["resources"]["parties"]
        .as_array()
        .and_then(|rows| rows.iter().find(|p| crate::data::s(&p["id"]) == "CU-A"))
        .cloned()
        .expect("demo customer CU-A");
    assert!(crate::ui::views::contacts::party_matches(&rt.state, &party, ""));
    assert!(crate::ui::views::contacts::party_matches(&rt.state, &party, "CU-A"));
    assert!(!crate::ui::views::contacts::party_matches(
        &rt.state,
        &party,
        "___no-such-contact___"
    ));
    click(&mut rt, Page::Contacts.key());
    assert!(rt.find("field-contact-search").is_some());
    assert!(rt.find("contact-CU-A").is_some());
    rt.state.forms.set("contact-search", "___no-such-contact___");
    rt.frame(1.0 / 60.0);
    assert!(rt.find("contact-CU-A").is_none(), "search should hide unmatched contacts");
}

#[test]
fn test_demo_clock_is_inside_the_fixture_year() {
    let rt = make_test_runtime();
    let (from, to) = rt.state.books.range();
    let today = data::today();
    assert!(from <= today && today <= to, "today={today} outside [{from}, {to}]");
}

#[test]
fn test_engine_sees_writes_from_another_store() {
    let mut rt = make_test_runtime();
    let mut next = rt.state.books.workspace["settings"].clone();
    let was = next["show_pos"].as_bool().unwrap_or(false);
    next["show_pos"] = serde_json::json!(!was);
    for (k, d) in [
        ("next_number", "1"),
        ("default_validity_days", "14"),
        ("default_discount_rial", "0"),
        ("tax_basis_points", "0"),
    ] {
        if next[k].as_str().is_none() {
            next[k] = serde_json::json!(next[k].as_u64().map(|n| n.to_string()).unwrap_or_else(|| d.into()));
        }
    }
    sarv_quote::desktop::dispatch(&rt.state.engine.dir, "settings", next).expect("external settings write");
    rt.state.refresh();
    assert_eq!(rt.state.books.workspace["settings"]["show_pos"].as_bool().unwrap_or(false), !was);
}

#[test]
fn test_integrity_status_survives_between_calls() {
    let rt = make_test_runtime();
    let a = rt.state.query("integrity", serde_json::json!({})).expect("first integrity query");
    let b = rt.state.query("integrity", serde_json::json!({})).expect("second integrity query");
    assert_eq!(a["status"], b["status"]);
}
