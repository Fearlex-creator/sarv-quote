//! Sarv Desktop — native Rune + rune-kit UI for Sarv accounting.

pub mod app;
pub mod data;
pub mod fa_messages;
pub mod i18n;
pub mod ui;

use app::{Page, State};
use rune::{App, AppSpec, Chrome};
use rune_kit::shell::is_sidebar_toggle;

const VAZIRMATN_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Vazirmatn-Regular.ttf");
const VAZIRMATN_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Vazirmatn-Medium.ttf");
const VAZIRMATN_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Vazirmatn-SemiBold.ttf");
const VAZIRMATN_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Vazirmatn-Bold.ttf");

fn handle_key(s: &mut State, ev: &rune::KeyEvent, _tx: &mut rune::Tx) -> bool {
    use rune::Key;
    if matches!(ev.key, Key::Escape) {
        if s.palette_open {
            s.palette_open = false;
            return true;
        }
        if s.sidebar.is_peeking() {
            s.sidebar.end_peek();
            return true;
        }
        return false;
    }
    if ev.is_command('k') {
        if s.palette_open {
            s.palette_open = false;
        } else {
            s.open_palette();
        }
        return true;
    }
    // Sarv: ⌘B. rune-kit shell also accepts ⇧⌘B / ⌃⌘S.
    if ev.is_command('b') || is_sidebar_toggle(ev) {
        s.toggle_sidebar();
        return true;
    }
    if ev.is_command('d') {
        s.dark = !s.dark;
        return true;
    }
    if ev.is_command('l') {
        let next = s.lang().other();
        s.set_lang(next);
        return true;
    }
    false
}

fn main() {
    // Traffic lights centred on the framed toolbar band (rune-kit shell example).
    let chrome = Chrome {
        full_size_content: true,
        traffic_lights: Some((16.0, 25.0)),
        drag_regions: true,
    };
    App::new(AppSpec::<State> {
        fonts: &[VAZIRMATN_REGULAR, VAZIRMATN_MEDIUM, VAZIRMATN_SEMIBOLD, VAZIRMATN_BOLD],
        families: ("Vazirmatn", "monospace"),
        scenes: &[
            ("overview", |s| s.nav(Page::Overview)),
            ("sales", |s| s.nav(Page::Sales)),
            ("pos", |s| {
                let _ = s.run(
                    "product_profile",
                    serde_json::json!({
                        "item_id": "ITEM-A",
                        "profile": {
                            "barcode": "",
                            "category": "",
                            "notes": "",
                            "sale_price": "250000",
                            "minimum_stock": "0"
                        }
                    }),
                    None,
                );
                s.nav(Page::Pos);
                s.pos.lines.push(ui::views::pos::PosLine {
                    item_id: "ITEM-A".into(),
                    name: "کالای الف".into(),
                    qty: 2,
                    price_rials: 250_000,
                });
            }),
            ("invoice", |s| {
                s.nav(Page::Sales);
                s.sales.start_new();
                s.sales.customer_combo.step(rune_kit::ComboboxEvent::Toggle { current: None });
            }),
            ("purchases", |s| s.nav(Page::Purchases)),
            ("payments", |s| s.nav(Page::Payments)),
            ("banking", |s| s.nav(Page::Banking)),
            ("contacts", |s| s.nav(Page::Contacts)),
            ("inventory", |s| s.nav(Page::Inventory)),
            ("accounting", |s| s.nav(Page::Accounting)),
            ("reports", |s| s.nav(Page::Reports)),
            ("sheets", |s| s.nav(Page::Sheets)),
            ("settings", |s| s.nav(Page::Settings)),
            ("light", |s| s.dark = false),
            ("dark", |s| s.dark = true),
            ("fa", |s| s.set_lang(i18n::Lang::Fa)),
            ("en", |s| s.set_lang(i18n::Lang::En)),
            ("collapse", |s| s.sidebar.set_open(false)),
            ("expand", |s| s.sidebar.set_open(true)),
        ],
        on_key: Some(handle_key),
        ..AppSpec::new("حسابداری سرو / Sarv Accounting", (1180.0, 780.0), State::open, ui::root_view)
    })
    .chrome(chrome)
    .run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Tone;
    use rune::{Env, Runtime};

    fn make_test_runtime() -> Runtime<State> {
        let env = Env {
            system_dark: true,
            scale: 2.0,
        };
        let mut rt = Runtime::new(State::with_engine(data::Engine::demo(), true), ui::root_view, (1180.0, 780.0), env);
        rt.frame(0.0);
        rt
    }

    fn click(rt: &mut Runtime<State>, key: &str) {
        let target = key.to_string();
        rt.update(|_, tx| tx.scroll_into_view(target));
        rt.frame(1.0 / 60.0);
        let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing element `{key}`"));
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
    }

    #[test]
    fn test_initial_dashboard_view_renders_cleanly() {
        let mut rt = make_test_runtime();
        assert_eq!(rt.state.page, Page::Overview);
        assert!(rt.state.sidebar_open());
        assert!(rt.find("sidebar-brand").is_some());
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
        assert!((sale_y - pay_y).abs() < 12.0, "actions should share one row at 1180: {sale_y} {pay_y}");
        let (_, cash_y) = rt.find("metric-cash").unwrap();
        let (_, profit_y) = rt.find("metric-profit").unwrap();
        assert!((cash_y - profit_y).abs() < 12.0, "figures should share one row at 1180: {cash_y} {profit_y}");
    }

    #[test]
    fn test_overview_actions_wrap_in_a_narrow_window() {
        let env = Env { system_dark: true, scale: 2.0 };
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
            rt.state.books.workspace["resources"]["parties"] =
                json!([{ "id": "p1", "name": "Ada", "customer": true, "supplier": false }]);
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
        for page in Page::ALL {
            click(&mut rt, page.key());
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
    fn test_settings_segmented_controls() {
        let mut rt = make_test_runtime();
        click(&mut rt, Page::Settings.key());
        assert!(rt.find("settings-tabs").is_some());
        assert!(rt.find("settings-theme").is_some());
        assert!(rt.find("settings-lang").is_some());
        assert!(rt.find("settings-theme-switch").is_some());
    }

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
    fn test_new_product_dialog_and_save_headlessly() {
        let mut rt = make_test_runtime();
        click(&mut rt, Page::Inventory.key());
        assert!(rt.find("inventory-table").is_some());
        click(&mut rt, "btn-New product");
        assert!(rt.state.inventory.modal_open);
        assert!(rt.find("field-item-name").is_some());
        assert!(rt.find("field-item-sku").is_some());
        assert!(rt.find("field-item-price").is_some());
        assert!(rt.find("item-unit").is_some(), "unit Select present");

        rt.state.forms.set("item-name", "Test Tea");
        rt.state.forms.set("item-sku", "TEA-1");
        rt.state.inventory.unit_id = "PIECE".into();
        rt.state.forms.set("item-price", "25000");
        rt.state.forms.set("item-barcode", "123");
        rt.state.forms.set("item-notes", "Green tea");
        click(&mut rt, "btn-Save");
        assert!(!rt.state.inventory.modal_open, "dialog closes after save");
        let items = rt.state.books.workspace["resources"]["items"].as_array().cloned().unwrap_or_default();
        assert!(
            items.iter().any(|i| i["name"] == "Test Tea" || i["sku"] == "TEA-1"),
            "product persisted in resources.items"
        );
    }

    #[test]
    fn test_sale_facture_pickers_headlessly() {
        let mut rt = make_test_runtime();
        click(&mut rt, Page::Sales.key());
        click(&mut rt, "btn-New sales invoice");
        assert!(rt.state.sales.modal_open);
        assert!(rt.find("sale-customer").is_some());
        assert!(rt.find("sale-item-0").is_some());
        assert!(rt.find("sale-pay-method").is_some());
        assert!(rt.find("sale-dialog").is_some());
        click(&mut rt, "sale-customer-trigger");
        for _ in 0..4 {
            rt.frame(1.0 / 60.0);
        }
        let (fx, fy) = rt.find("sale-customer-trigger").unwrap();
        let (mx, my) = rt.find("sale-customer-content").expect("customer menu");
        assert!((fx - mx).abs() < 48.0, "customer menu should share the field's column: field {fx} menu {mx}");
        assert!((fy - my).abs() > 12.0 && (fy - my).abs() < 280.0, "customer menu should sit on the field: field {fy} menu {my}");
        click(&mut rt, "sale-customer-trigger");
        // 10,000 toman must be stored as 100,000 rials, then delivered and settled.
        rt.state.fmt.unit = crate::i18n::Unit::Toman;
        rt.state.sales.customer_id = "CU-A".into();
        rt.state.sales.pay_method = 0;
        rt.state.sales.lines[0].item_id = "ITEM-A".into();
        rt.state.sales.lines[0].qty = "1".into();
        rt.state.sales.lines[0].price = "10000".into();
        rt.state.forms.set("sale-qty-0", "1");
        rt.state.forms.set("sale-price-0", "10000");
        let before = rt.state.books.list("sales").len();
        click(&mut rt, "btn-Complete sale");
        if rt.state.sales.modal_open {
            let msgs: Vec<_> = rt.state.toasts.iter().map(|t| format!("{:?}:{}", t.tone, t.message)).collect();
            panic!("sale still open; toasts={msgs:?}");
        }
        let sales = rt.state.books.list("sales");
        assert!(sales.len() > before, "fulfillment created a sales record");
        let dumped = serde_json::to_string(&sales[0]).unwrap_or_default();
        assert!(
            dumped.contains("100000") && !dumped.contains("1000000"),
            "toman price must not be multiplied twice: {dumped}"
        );
        assert!(
            rt.state.toasts.iter().any(|t| t.tone == Tone::Success),
            "posted only after delivery and payment"
        );
    }

    #[test]
    fn test_purchase_and_journal_dialogs_headlessly() {
        let mut rt = make_test_runtime();
        click(&mut rt, Page::Purchases.key());
        click(&mut rt, "btn-New purchase order");
        assert!(rt.state.purchases.modal_open);
        assert!(rt.find("buy-supplier").is_some());
        assert!(rt.find("buy-item-0").is_some());
        rt.state.purchases.modal_open = false;
        click(&mut rt, "refresh-books");

        click(&mut rt, Page::Accounting.key());
        click(&mut rt, "btn-New journal");
        assert!(rt.state.accounting.modal_open);
        assert!(rt.find("journal-debit").is_some());
        assert!(rt.find("journal-credit").is_some());
        click(&mut rt, "journal-debit-trigger");
        for _ in 0..4 {
            rt.frame(1.0 / 60.0);
        }
        let (fx, fy) = rt.find("journal-debit-trigger").unwrap();
        let (mx, my) = rt.find("journal-debit-content").expect("account menu");
        assert!((fx - mx).abs() < 160.0, "account menu should cover the field: field {fx} menu {mx}");
        assert!((fy - my).abs() > 12.0 && (fy - my).abs() < 280.0, "account menu should sit on the field: field {fy} menu {my}");
        rt.state.accounting.modal_open = false;
        rt.frame(1.0 / 60.0);
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
    fn test_display_quantity_is_not_thousandths() {
        assert_eq!(crate::ui::ops::qty_from_thousandths("1000"), "1");
        assert_eq!(crate::ui::ops::qty_from_thousandths("1500"), "1.5");
        assert_eq!(crate::ui::ops::typed_display_amount("۱۰٬۰۰۰"), "10000");
    }

    #[test]
    fn test_fiscal_period_follows_the_jalali_year() {
        use chrono::NaiveDate;
        let (year, start, end) = crate::ui::views::setup::fiscal_period(
            crate::i18n::CalendarKind::Jalali,
            NaiveDate::from_ymd_opt(2026, 11, 11).unwrap(),
        );
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
    fn test_cheque_strip_labels_are_translated_keys() {
        use crate::ui::views::payments::{cheque_action_label, cheque_status_label};
        assert_eq!(cheque_status_label("in_hand"), "In hand");
        assert_eq!(cheque_status_label("in_collection"), "In collection");
        assert_eq!(cheque_status_label("cleared"), "Cleared");
        assert_eq!(cheque_action_label("deposit"), Some("Deposit for collection"));
        assert_eq!(cheque_action_label("clear"), Some("Mark as cleared"));
        assert_eq!(cheque_action_label("bounce"), Some("Mark as bounced"));
        assert_eq!(cheque_action_label("return"), Some("Return to customer"));
        assert_eq!(cheque_action_label("void"), Some("Void cheque"));
        assert_eq!(cheque_action_label("spend"), None);
    }

    #[test]
    fn test_rial_sale_is_not_scaled() {
        let mut rt = make_test_runtime();
        rt.state.fmt.unit = crate::i18n::Unit::Rial;
        click(&mut rt, Page::Sales.key());
        click(&mut rt, "btn-New sales invoice");
        rt.state.sales.customer_id = "CU-A".into();
        rt.state.sales.pay_method = 2;
        rt.state.sales.lines[0].item_id = "ITEM-A".into();
        rt.state.sales.lines[0].qty = "1".into();
        rt.state.sales.lines[0].price = "100000".into();
        rt.state.forms.set("sale-qty-0", "1");
        rt.state.forms.set("sale-price-0", "100000");
        click(&mut rt, "btn-Complete sale");
        if rt.state.sales.modal_open {
            let msgs: Vec<_> = rt.state.toasts.iter().map(|t| format!("{:?}:{}", t.tone, t.message)).collect();
            panic!("rial sale still open; toasts={msgs:?}");
        }
        let dumped = serde_json::to_string(&rt.state.books.list("sales")[0]).unwrap_or_default();
        assert!(dumped.contains("100000"), "rial amount stored once: {dumped}");
    }

    #[test]
    fn test_payment_allocates_across_open_items() {
        let mut rt = make_test_runtime();
        rt.state.fmt.unit = crate::i18n::Unit::Rial;
        let party = rt
            .state
            .create_party(crate::ui::resources::ResourceKind::Customer, "Alloc Buyer", "", "", "")
            .unwrap_or_else(|e| panic!("{e}"));
        for price in ["40000", "60000"] {
            click(&mut rt, Page::Sales.key());
            click(&mut rt, "btn-New sales invoice");
            rt.state.sales.customer_id = party.clone();
            rt.state.sales.pay_method = 2;
            rt.state.sales.lines[0].item_id = "ITEM-A".into();
            rt.state.sales.lines[0].qty = "1".into();
            rt.state.sales.lines[0].price = price.into();
            rt.state.forms.set("sale-qty-0", "1");
            rt.state.forms.set("sale-price-0", price);
            click(&mut rt, "btn-Complete sale");
            if rt.state.sales.modal_open {
                let msgs: Vec<_> = rt.state.toasts.iter().map(|t| format!("{:?}:{}", t.tone, t.message)).collect();
                panic!("credit sale still open; toasts={msgs:?}");
            }
        }
        click(&mut rt, Page::Payments.key());
        click(&mut rt, "btn-Receive money");
        rt.state.payments.party_id = party.clone();
        rt.state.payments.funds = 0;
        rt.state.forms.set("pay-amount", "100000");
        click(&mut rt, "btn-Save");
        if rt.state.payments.modal_open {
            let msgs: Vec<_> = rt.state.toasts.iter().map(|t| format!("{:?}:{}", t.tone, t.message)).collect();
            panic!("payment still open; toasts={msgs:?}");
        }
        let open = rt
            .state
            .query("payment_options", serde_json::json!({ "side": "receive", "party_id": party }))
            .unwrap_or_else(|e| panic!("{e}"));
        let remaining: i64 = open["items"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|i| crate::data::s(&i["remaining"]).parse::<i64>().unwrap_or(0))
            .sum();
        assert_eq!(remaining, 0, "both invoices settled, leftover is not a second advance: {open}");
    }

    #[test]
    fn test_export_writes_csv_and_language_persists() {
        let mut rt = make_test_runtime();
        click(&mut rt, Page::Reports.key());
        click(&mut rt, "btn-Export CSV");
        let exports = rt.state.engine.dir.join("exports");
        let mut names = std::fs::read_dir(&exports).unwrap().map(|e| e.unwrap().file_name()).collect::<Vec<_>>();
        names.sort();
        assert!(names.iter().any(|n| n.to_string_lossy().contains("csv") || n.to_string_lossy().ends_with(".csv")), "{names:?}");
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

    fn blow(rt: &Runtime<State>, what: &str) -> String {
        let msgs: Vec<_> = rt.state.toasts.iter().map(|t| format!("{:?}:{}", t.tone, t.message)).collect();
        format!("{what}; toasts={msgs:?}")
    }

    #[test]
    fn test_owner_walks_profiles_documents_and_books() {
        let mut rt = make_test_runtime();
        rt.state.fmt.unit = crate::i18n::Unit::Rial;

        click(&mut rt, Page::Contacts.key());
        click(&mut rt, "contact-CU-A");
        click(&mut rt, "btn-Contact details");
        assert!(rt.find("contact-profile-dialog").is_some(), "{}", blow(&rt, "contact dialog"));
        rt.state.forms.set("profile-phone", "09120001122");
        click(&mut rt, "btn-Save");
        assert!(rt.state.contacts.open_id.is_empty(), "{}", blow(&rt, "contact save"));
        let detail = rt
            .state
            .query("contact_detail", serde_json::json!({ "party_id": "CU-A", "as_of": crate::data::today() }))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(crate::data::s(&detail["profile"]["phone"]), "09120001122");

        click(&mut rt, Page::Inventory.key());
        click(&mut rt, "item-ITEM-A");
        click(&mut rt, "btn-Product details");
        assert!(rt.find("product-profile-dialog").is_some(), "{}", blow(&rt, "product dialog"));
        rt.state.forms.set("prod-barcode", "6260001112223");
        rt.state.forms.set("prod-price", "150000");
        click(&mut rt, "btn-Save");
        assert!(!rt.state.inventory.profile_open, "{}", blow(&rt, "product save"));
        assert_eq!(
            crate::data::s(&rt.state.books.workspace["product_profiles"]["ITEM-A"]["barcode"]),
            "6260001112223"
        );

        click(&mut rt, Page::Sales.key());
        click(&mut rt, "btn-New sales invoice");
        rt.state.sales.customer_id = "CU-A".into();
        rt.state.sales.pay_method = 2;
        rt.state.sales.lines[0].item_id = "ITEM-A".into();
        rt.state.sales.lines[0].qty = "1".into();
        rt.state.sales.lines[0].price = "150000".into();
        rt.state.forms.set("sale-qty-0", "1");
        rt.state.forms.set("sale-price-0", "150000");
        click(&mut rt, "btn-Issue quotation");
        assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "quotation"));
        let draft_id = rt
            .state
            .books
            .workspace["drafts"]
            .as_array()
            .and_then(|rows| rows.iter().rev().find(|d| !d["quote_issue"].is_null()))
            .map(|d| crate::data::s(&d["id"]))
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| panic!("{}", blow(&rt, "draft id")));
        click(&mut rt, &format!("draft-{draft_id}"));
        click(&mut rt, "btn-Print / save PDF");
        let quote_pdf = rt.state.engine.dir.join("exports").join(format!("quotation-{draft_id}.pdf"));
        let quote_bytes = std::fs::read(&quote_pdf).unwrap_or_else(|e| panic!("{e}; {}", blow(&rt, "quotation pdf")));
        assert!(quote_bytes.starts_with(b"%PDF"), "quotation pdf");

        rt.state.sales.tab_idx = 0;
        rt.frame(1.0 / 60.0);
        click(&mut rt, "btn-New sales invoice");
        rt.state.sales.customer_id = "CU-A".into();
        rt.state.sales.pay_method = 2;
        rt.state.sales.lines[0].item_id = "ITEM-A".into();
        rt.state.sales.lines[0].qty = "1".into();
        rt.state.sales.lines[0].price = "150000".into();
        rt.state.forms.set("sale-qty-0", "1");
        rt.state.forms.set("sale-price-0", "150000");
        click(&mut rt, "btn-Complete sale");
        assert!(!rt.state.sales.modal_open, "{}", blow(&rt, "credit sale"));
        let invoice_id = rt
            .state
            .books
            .list("sales")
            .into_iter()
            .find(|r| r["command"].get("Fulfill").is_some())
            .map(|r| crate::data::s(&r["id"]))
            .unwrap_or_else(|| panic!("{}", blow(&rt, "invoice id")));
        click(&mut rt, &format!("sale-{invoice_id}"));
        click(&mut rt, "btn-Print / save PDF");
        let invoice_pdf = rt.state.engine.dir.join("exports").join(format!("invoice-{invoice_id}.pdf"));
        let invoice_bytes = std::fs::read(&invoice_pdf).unwrap_or_else(|e| panic!("{e}; {}", blow(&rt, "invoice pdf")));
        assert!(invoice_bytes.starts_with(b"%PDF"), "invoice pdf");
        click(&mut rt, "btn-Return");
        let returned = rt.state.books.list("sales").into_iter().any(|r| r["command"].get("Return").is_some());
        assert!(returned, "{}", blow(&rt, "sale return"));

        let supplier = rt
            .state
            .books
            .workspace["resources"]["parties"]
            .as_array()
            .and_then(|rows| rows.iter().find(|p| p["supplier"] == true))
            .map(|p| crate::data::s(&p["id"]))
            .filter(|id| !id.is_empty());
        if let Some(supplier) = supplier {
            click(&mut rt, Page::Purchases.key());
            click(&mut rt, "btn-New purchase order");
            rt.state.purchases.supplier_id = supplier;
            rt.state.purchases.receive_and_bill = true;
            rt.state.purchases.lines[0].item_id = "ITEM-A".into();
            rt.state.purchases.lines[0].qty = "1".into();
            rt.state.purchases.lines[0].price = "80000".into();
            rt.state.forms.set("buy-qty-0", "1");
            rt.state.forms.set("buy-price-0", "80000");
            click(&mut rt, "btn-Save");
            assert!(!rt.state.purchases.modal_open, "{}", blow(&rt, "purchase"));
            let purchases = rt.state.books.list("purchases");
            let bill = purchases.iter().find(|r| r["command"].get("Bill").is_some()).map(|r| crate::data::s(&r["id"]));
            let receipt = purchases.iter().find(|r| r["command"].get("Receive").is_some()).map(|r| crate::data::s(&r["id"]));
            let (return_id, tab) = bill
                .filter(|id| !id.is_empty())
                .map(|id| (id, 2))
                .or_else(|| receipt.filter(|id| !id.is_empty()).map(|id| (id, 1)))
                .unwrap_or_else(|| panic!("{}", blow(&rt, "receipt")));
            click(&mut rt, &format!("segment{tab}"));
            for _ in 0..8 {
                rt.frame(1.0 / 60.0);
            }
            click(&mut rt, &format!("purchase-{return_id}"));
            click(&mut rt, "btn-Return");
            let sent_back = rt.state.books.list("purchases").into_iter().any(|r| r["command"].get("Return").is_some());
            assert!(sent_back, "{}", blow(&rt, "purchase return"));
        }

        click(&mut rt, Page::Accounting.key());
        let accounts = crate::ui::pickers::account_options(&rt.state);
        assert!(accounts.len() >= 2, "chart of accounts");
        click(&mut rt, "btn-New journal");
        rt.state.accounting.debit_account = accounts[0].0.clone();
        rt.state.accounting.credit_account = accounts[1].0.clone();
        rt.state.forms.set("journal-amount", "25000");
        rt.state.forms.set("journal-reason", "Owner adjustment");
        click(&mut rt, "btn-Save");
        assert!(!rt.state.accounting.modal_open, "{}", blow(&rt, "journal"));
        let journal_id = rt
            .state
            .books
            .list("journals")
            .into_iter()
            .find(|j| crate::data::s(&j["reason"]) == "Owner adjustment")
            .map(|j| crate::data::s(&j["id"]))
            .unwrap_or_else(|| panic!("{}", blow(&rt, "journal id")));
        click(&mut rt, &format!("journal-{journal_id}"));
        click(&mut rt, "btn-Reverse");
        let reversed = rt.state.books.list("journals").into_iter().any(|j| {
            j["kind"].get("Reversal").is_some() || crate::data::s(&j["event"]) == "reversal"
        });
        assert!(reversed, "{}", blow(&rt, "reversal"));

        click(&mut rt, "segment1");
        for _ in 0..8 {
            rt.frame(1.0 / 60.0);
        }
        let first_account = crate::data::s(&rt.state.books.workspace["accounts"][0]["id"]);
        click(&mut rt, &format!("acc-{first_account}"));
        click(&mut rt, "btn-Ledger");
        assert!(rt.state.accounting.ledger_open, "{}", blow(&rt, "ledger"));
        assert!(rt.find("ledger-dialog").is_some());
        click(&mut rt, "btn-Close");

        click(&mut rt, "segment2");
        for _ in 0..8 {
            rt.frame(1.0 / 60.0);
        }
        let period = rt.state.books.period_id.clone();
        click(&mut rt, &format!("period-{period}"));
        click(&mut rt, "btn-Lock fiscal period");
        let locked = rt
            .state
            .books
            .workspace["periods"]
            .as_array()
            .and_then(|rows| rows.iter().find(|p| crate::data::s(&p["id"]) == period))
            .and_then(|p| p["locked"].as_bool())
            .unwrap_or(false);
        if locked {
            click(&mut rt, "btn-Lock fiscal period");
            let open_again = rt
                .state
                .books
                .workspace["periods"]
                .as_array()
                .and_then(|rows| rows.iter().find(|p| crate::data::s(&p["id"]) == period))
                .and_then(|p| p["locked"].as_bool())
                .unwrap_or(true);
            assert!(!open_again, "{}", blow(&rt, "reopen period"));
        } else {
            let note = blow(&rt, "period stayed open");
            assert!(note.contains("Integrity") || note.contains("یکپارچ"), "{note}");
        }

        click(&mut rt, Page::Settings.key());
        click(&mut rt, "btn-Your business");
        assert!(rt.find("business-profile-dialog").is_some(), "{}", blow(&rt, "business dialog"));
        rt.state.forms.set("biz-prefix", "SARV");
        rt.state.forms.set("biz-phone", "02100000000");
        click(&mut rt, "btn-Save");
        assert!(!rt.state.settings.business_open, "{}", blow(&rt, "business save"));
        assert_eq!(crate::data::s(&rt.state.books.workspace["business_profile"]["invoice_prefix"]), "SARV");

        click(&mut rt, "btn-Backup");
        click(&mut rt, "btn-Restore backup");
        let filename = rt
            .state
            .settings
            .backups["backups"]
            .as_array()
            .and_then(|rows| rows.first())
            .map(|row| crate::data::s(&row["filename"]))
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| panic!("{}", blow(&rt, "backup list")));
        click(&mut rt, &format!("btn-backup-{filename}"));
        assert!(!rt.state.settings.restore_note.is_empty(), "{}", blow(&rt, "restore preview"));
        click(&mut rt, "btn-restore-confirm");
        assert!(!rt.state.settings.restore_open, "{}", blow(&rt, "restore"));
        assert!(!rt.state.books.company_name().is_empty(), "books still open after restore");
    }

    #[test]
    fn test_first_run_is_a_welcome_then_the_books() {
        let dir = std::env::temp_dir().join(format!("sarv-fresh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let env = Env { system_dark: true, scale: 2.0 };
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
        assert!(rt.find("sidebar-brand").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sheet_formulas_save_beside_the_books() {
        let mut rt = make_test_runtime();
        let generation = sarv_quote::store::Store::open(&rt.state.engine.dir).unwrap().local_generation();
        click(&mut rt, Page::Sheets.key());
        assert!(rt.find("sheet-card").is_some(), "{}", blow(&rt, "sheet"));
        click(&mut rt, "cell-B1");
        assert_eq!(rt.state.forms.get("sheet-formula"), "=BOOKS(\"cash\")");

        click(&mut rt, "cell-C1");
        rt.state.forms.set("sheet-formula", "10");
        click(&mut rt, "btn-sheet-apply");
        click(&mut rt, "cell-C2");
        rt.state.forms.set("sheet-formula", "25");
        click(&mut rt, "btn-sheet-apply");
        click(&mut rt, "cell-C3");
        rt.state.forms.set("sheet-formula", "=C1+C2");
        click(&mut rt, "btn-sheet-apply");
        click(&mut rt, "btn-Add sheet");
        assert_eq!(rt.state.sheets.book.sheets.len(), 2);
        click(&mut rt, "btn-sheet-tab-0");
        assert!(rt.state.sheets.dirty, "edits leave the workbook dirty before save");
        click(&mut rt, "btn-Save sheet");
        assert!(!rt.state.sheets.dirty, "successful save must clear the dirty flag");

        let loaded = rt.state.engine.call("sheet_load", serde_json::json!({ "company_id": rt.state.books.company_id })).unwrap();
        assert_eq!(loaded["workbook"]["sheets"][0]["cells"]["C3"], "=C1+C2");
        assert_eq!(loaded["workbook"]["sheets"][0]["cells"]["B1"], "=BOOKS(\"cash\")");
        assert_eq!(sarv_quote::store::Store::open(&rt.state.engine.dir).unwrap().local_generation(), generation);
        assert!(rt.state.toasts.iter().any(|t| t.message.contains("برگه") || t.message.contains("Sheet")));
    }

    #[test]
    fn test_pos_charges_the_catalog_and_keeps_the_books() {
        let mut rt = make_test_runtime();
        rt.state.fmt.unit = crate::i18n::Unit::Rial;
        rt.state
            .run(
                "product_profile",
                serde_json::json!({
                    "item_id": "ITEM-A",
                    "profile": {
                        "barcode": "6260001",
                        "category": "",
                        "notes": "",
                        "sale_price": "50000",
                        "minimum_stock": "0"
                    }
                }),
                None,
            )
            .expect("price");
        click(&mut rt, Page::Pos.key());
        assert!(rt.find("pos-screen").is_some());
        assert!(rt.find("pos-ticket").is_some());
        assert!(
            rt.state.forms.view("pos-search").focused,
            "POS opens with catalog search focused for barcode entry"
        );
        assert!(rt.find("field-pos-search").is_some());
        assert!(crate::ui::views::pos::product_matches("کالای الف", "SKU-A", "ITEM-A", "6260001", "sku-a"));
        assert!(crate::ui::views::pos::product_matches("کالای الف", "SKU-A", "ITEM-A", "6260001", "۶۲۶"));
        assert!(!crate::ui::views::pos::product_matches("کالای الف", "SKU-A", "ITEM-A", "6260001", "no-such-sku"));

        click(&mut rt, "pos-item-ITEM-A");
        click(&mut rt, "btn-pos-account");
        assert_eq!(rt.state.pos.lines.len(), 1, "on account waits for a customer");
        click(&mut rt, "pos-item-ITEM-A");
        assert_eq!(rt.state.pos.lines[0].qty, 2);
        click(&mut rt, "pos-minus-ITEM-A");
        assert_eq!(rt.state.pos.lines[0].qty, 1);
        rt.state.forms.set("pos-received", "1000");
        click(&mut rt, "btn-pos-cash");
        assert_eq!(rt.state.pos.lines.len(), 1, "short cash does not post");

        rt.state.forms.set("pos-received", "50000");
        let before = rt.state.books.count("sales_count");
        click(&mut rt, "btn-pos-cash");
        assert!(rt.state.pos.lines.is_empty(), "{}", blow(&rt, "pos cash"));
        assert!(rt.state.books.count("sales_count") > before);
        let dumped = serde_json::to_string(&rt.state.books.list("sales")[0]).unwrap_or_default();
        assert!(dumped.contains("50000"), "catalog price posted once: {dumped}");
        assert!(rt.state.toasts.iter().any(|t| t.tone == Tone::Success));
    }

}
