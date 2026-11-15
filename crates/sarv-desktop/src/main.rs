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
        if s.request_close_top_dialog() {
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
                s.show_pos = true;
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
                ui::views::sales::open_sale(s);
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
mod tests;
