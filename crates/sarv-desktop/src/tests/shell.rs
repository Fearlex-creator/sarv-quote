use super::harness::{assert_no_danger, blow, books_validate, click, make_test_runtime, type_into};
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

#[test]
fn refresh_after_external_write_shows_the_new_data() {
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Contacts.key());
    assert!(rt.find("contact-EXT-CUST").is_none());

    sarv_quote::desktop::dispatch(
        &rt.state.engine.dir,
        "resource",
        serde_json::json!({
            "company_id": rt.state.books.company_id,
            "period_id": rt.state.books.period_id,
            "date": data::today(),
            "source_id": "ext-1",
            "request_key": "ext-1",
            "kind": "customer",
            "id": "EXT-CUST",
            "name": "External Buyer",
            "profile": {
                "phone": "09120009999",
                "display_name": "External Buyer"
            },
            "reason": "External write"
        }),
    )
    .expect("external resource write");

    click(&mut rt, "refresh-books");
    assert!(
        rt.find("contact-EXT-CUST").is_some(),
        "UI should display contact written externally after refresh"
    );
    assert_no_danger(&rt);
    books_validate(&rt);
}

#[test]
fn test_refresh_reports_engine_failure() {
    let mut rt = make_test_runtime();
    let db_path = rt.state.engine.dir.join("sarv.sqlite3");
    std::fs::write(&db_path, b"not a sqlite database").expect("corrupt sqlite file");

    rt.state.refresh();
    assert!(!rt.state.books.error.is_empty(), "refresh should record engine error");

    click(&mut rt, "refresh-books");
    assert!(
        rt.state.toasts.iter().any(|t| matches!(t.tone, crate::app::Tone::Danger)),
        "refresh button should surface a danger toast when books fail to load: {:?}",
        rt.state.toasts
    );
}

#[test]
fn test_contacts_table_sorts_by_name_and_keeps_search() {
    use crate::ui::rows::SortKey;
    let mut rt = make_test_runtime();
    click(&mut rt, Page::Contacts.key());
    assert!(rt.state.contacts.rows.len() >= 2);

    crate::ui::views::contacts::apply_sort(&mut rt.state, "name", rune_kit::SortDir::Descending);
    rt.frame(1.0 / 60.0);
    let names: Vec<String> = rt
        .state
        .contacts
        .rows
        .iter()
        .filter_map(|r| match r.keys.first() {
            Some(SortKey::Text(s)) => Some(s.clone()),
            _ => None,
        })
        .collect();
    assert!(names.windows(2).all(|w| w[0] >= w[1]), "expected descending names: {names:?}");

    rt.state.forms.set("contact-search", "CU-A");
    rt.frame(1.0 / 60.0);
    assert!(
        rt.find("contact-CU-A").is_some(),
        "CU-A should still be visible after sort + search"
    );
}

#[test]
fn test_create_shortcut_opens_the_sale_dialog() {
    let mut rt = make_test_runtime();
    rt.on_key = Some(handle_key);
    rt.key(rune::KeyEvent {
        key: rune::Key::Command('n'),
        mods: rune::Mods {
            cmd: true,
            alt: true,
            ..Default::default()
        },
    });
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.page, Page::Sales);
    assert!(rt.state.sales.modal_open);
}

#[test]
fn test_create_shortcut_is_ignored_while_typing() {
    let mut rt = make_test_runtime();
    rt.on_key = Some(handle_key);
    rt.state.forms.edit("contact-search").focused = true;
    rt.key(rune::KeyEvent {
        key: rune::Key::Command('n'),
        mods: rune::Mods {
            cmd: true,
            alt: true,
            ..Default::default()
        },
    });
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.page, Page::Overview);
    assert!(!rt.state.sales.modal_open);
}

#[test]
fn test_palette_arrow_and_enter_run_the_selection() {
    let mut rt = make_test_runtime();
    rt.on_key = Some(handle_key);
    rt.key(rune::KeyEvent::from(rune::Key::Command('k')));
    rt.frame(1.0 / 60.0);
    assert!(rt.state.palette_open);
    assert_eq!(rt.state.palette_cursor, 0);

    rt.key(rune::KeyEvent::from(rune::Key::Down));
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.palette_cursor, 1);

    rt.key(rune::KeyEvent::from(rune::Key::Enter));
    rt.frame(1.0 / 60.0);
    assert!(!rt.state.palette_open);
    assert_eq!(rt.state.page, Page::Sales);
}

#[test]
fn test_palette_opens_the_contact_profile() {
    let mut rt = make_test_runtime();
    rt.on_key = Some(handle_key);
    rt.key(rune::KeyEvent::from(rune::Key::Command('k')));
    rt.frame(1.0 / 60.0);
    rt.state.forms.set("palette-query", "CU-A");
    rt.frame(1.0 / 60.0);

    rt.key(rune::KeyEvent::from(rune::Key::Enter));
    rt.frame(1.0 / 60.0);
    assert!(!rt.state.palette_open);
    assert_eq!(rt.state.page, Page::Contacts);
    assert_eq!(rt.state.contacts.open_id, "CU-A");
}

#[test]
fn test_search_folds_arabic_yeh_and_kaf() {
    let mut rt = make_test_runtime();
    sarv_quote::desktop::dispatch(
        &rt.state.engine.dir,
        "resource",
        serde_json::json!({
            "company_id": rt.state.books.company_id,
            "period_id": rt.state.books.period_id,
            "date": data::today(),
            "source_id": "fold-1",
            "request_key": "fold-1",
            "kind": "customer",
            "id": "CU-KIMIA",
            "name": "کیمیا",
            "profile": { "display_name": "کیمیا" },
            "reason": "Arabic fold test"
        }),
    )
    .expect("create Persian contact");
    rt.state.refresh();

    click(&mut rt, Page::Contacts.key());
    assert!(rt.find("contact-CU-KIMIA").is_some());
    rt.state.forms.set("contact-search", "كيميا");
    rt.frame(1.0 / 60.0);
    assert!(
        rt.find("contact-CU-KIMIA").is_some(),
        "Arabic kaf/yeh query should match Persian contact"
    );
}

#[test]
fn test_escape_closes_the_open_dialog() {
    let mut rt = make_test_runtime();
    rt.on_key = Some(handle_key);
    rt.key(rune::KeyEvent {
        key: rune::Key::Command('n'),
        mods: rune::Mods {
            cmd: true,
            alt: true,
            ..Default::default()
        },
    });
    rt.frame(1.0 / 60.0);
    assert!(rt.state.sales.modal_open);

    rt.key(rune::KeyEvent::from(rune::Key::Escape));
    rt.frame(1.0 / 60.0);
    assert!(!rt.state.sales.modal_open);
    assert!(!rt.state.request_close_top_dialog());
}

#[test]
fn test_overview_counts_cheques_due_within_a_week() {
    let mut rt = make_test_runtime();
    rt.state.fmt.unit = crate::i18n::Unit::Rial;
    let party = rt
        .state
        .create_party(crate::ui::resources::ResourceKind::Customer, "Week Cheque Buyer", "", "", "")
        .unwrap_or_else(|e| panic!("{e}"));

    click(&mut rt, Page::Sales.key());
    click(&mut rt, "btn-New sales invoice");
    rt.state.sales.customer_id = party.clone();
    rt.state.sales.pay_method = 2;
    rt.state.sales.lines[0].item_id = "ITEM-A".into();
    rt.state.sales.lines[0].qty = "1".into();
    rt.state.sales.lines[0].price = "130000".into();
    type_into(&mut rt, "sale-qty-0", "1");
    type_into(&mut rt, "sale-price-0", "130000");
    click(&mut rt, "btn-Complete sale");
    assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale"));

    let base = chrono::NaiveDate::parse_from_str(&data::today(), "%Y-%m-%d").unwrap();
    let d_within = base.checked_add_days(chrono::Days::new(3)).unwrap().format("%Y-%m-%d").to_string();
    let d_later = base.checked_add_days(chrono::Days::new(14)).unwrap().format("%Y-%m-%d").to_string();

    for (num, sayad, amt, due_iso) in [
        ("CHQ-WK-1", "3333444455556661", "50000", &d_within),
        ("CHQ-WK-2", "3333444455556662", "80000", &d_later),
    ] {
        click(&mut rt, Page::Payments.key());
        click(&mut rt, "btn-Receive money");
        rt.state.payments.party_id = party.clone();
        rt.state.payments.funds = 2;
        rt.frame(1.0 / 60.0);
        type_into(&mut rt, "pay-amount", amt);
        type_into(&mut rt, "pay-sayad", sayad);
        type_into(&mut rt, "pay-cheque-no", num);
        type_into(&mut rt, "pay-bank", "Melli");
        let due_in = rt.state.fmt.date_input(due_iso);
        type_into(&mut rt, "pay-due", &due_in);
        click(&mut rt, "btn-Save");
        assert!(!rt.state.payments.modal_open, "{}", blow(&rt, num));
    }

    click(&mut rt, Page::Overview.key());
    rt.frame(1.0 / 60.0);
    assert!(rt.find("overview-cheques-due").is_some());
    click(&mut rt, "overview-cheques-due");
    assert_eq!(rt.state.page, Page::Payments);
    assert_eq!(rt.state.payments.tab_idx, 1);
}

#[test]
fn test_startup_summary_dialog_and_sidebar_shortcuts() {
    let env = rune::Env {
        system_dark: true,
        scale: 2.0,
    };
    let mut rt = rune::Runtime::new(
        State::with_engine(crate::data::Engine::demo(), true),
        crate::ui::root_view,
        (1180.0, 780.0),
        env,
    );
    rt.frame(0.0);

    assert!(rt.state.startup_summary_open);
    assert!(rt.find("summary-cash").is_some());
    assert!(rt.find("summary-cheques-in-hand").is_some());
    assert!(rt.find("summary-cheques-in-collection").is_some());
    assert!(rt.find("summary-cheques-payable").is_some());

    click(&mut rt, "btn-Enter app");
    assert!(!rt.state.startup_summary_open);
    assert!(rt.find("summary-cash").is_none());

    click(&mut rt, "nav-cheque-search");
    assert_eq!(rt.state.page, Page::Payments);
    assert_eq!(rt.state.payments.tab_idx, 1);

    click(&mut rt, "nav-account-card");
    assert_eq!(rt.state.page, Page::Accounting);
    assert_eq!(rt.state.accounting.tab_idx, 1);

    click(&mut rt, "nav-trial-balance");
    assert_eq!(rt.state.page, Page::Reports);
    assert_eq!(rt.state.reports.tab_idx, 1);

    click(&mut rt, "nav-crm");
    assert_eq!(rt.state.page, Page::Contacts);

    click(&mut rt, "nav-security");
    assert_eq!(rt.state.page, Page::Settings);
}
