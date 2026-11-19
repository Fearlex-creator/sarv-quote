//! Global creation and navigation shortcuts.

use crate::app::{Page, State};
use crate::ui::views::{accounting, contacts, inventory, overview, payments, purchases, sales};
use overview::NamedKind;

pub struct Shortcut {
    pub id: &'static str,
    pub label: &'static str,
    pub keys: &'static [&'static str],
    pub run: fn(&mut State),
}

/// Creation and quick-navigation chords invoked with `Cmd/Ctrl + Alt + <key>`.
/// Note: `keys` lists only the suffix (`["Alt", "<key>"]`); Settings prepends `mod_key()`
/// (`⌘` on macOS, `Ctrl` elsewhere) when rendering the shortcut table.
pub const CREATE: &[(char, Shortcut)] = &[
    (
        'n',
        Shortcut {
            id: "sc-n",
            label: "New sales invoice",
            keys: &["Alt", "N"],
            run: |st| st.nav_then(Page::Sales, sales::open_sale),
        },
    ),
    (
        'c',
        Shortcut {
            id: "sc-c",
            label: "New customer",
            keys: &["Alt", "C"],
            run: |st| st.nav_then(Page::Contacts, |st| contacts::open_new_contact(st, false)),
        },
    ),
    (
        'v',
        Shortcut {
            id: "sc-v",
            label: "New supplier",
            keys: &["Alt", "V"],
            run: |st| st.nav_then(Page::Contacts, |st| contacts::open_new_contact(st, true)),
        },
    ),
    (
        'p',
        Shortcut {
            id: "sc-p",
            label: "New product",
            keys: &["Alt", "P"],
            run: |st| st.nav_then(Page::Inventory, inventory::open_new_product),
        },
    ),
    (
        'b',
        Shortcut {
            id: "sc-buy",
            label: "New purchase order",
            keys: &["Alt", "B"],
            run: |st| st.nav_then(Page::Purchases, purchases::open_purchase),
        },
    ),
    (
        'j',
        Shortcut {
            id: "sc-j",
            label: "New journal",
            keys: &["Alt", "J"],
            run: |st| st.nav_then(Page::Accounting, accounting::open_journal),
        },
    ),
    (
        't',
        Shortcut {
            id: "sc-t",
            label: "Transfer stock",
            keys: &["Alt", "T"],
            run: |st| st.nav_then(Page::Inventory, inventory::open_transfer),
        },
    ),
    (
        'w',
        Shortcut {
            id: "sc-w",
            label: "New warehouse",
            keys: &["Alt", "W"],
            run: |st| st.nav_then(Page::Overview, |st| overview::open_named(st, NamedKind::Warehouse)),
        },
    ),
    (
        'u',
        Shortcut {
            id: "sc-u",
            label: "New unit",
            keys: &["Alt", "U"],
            run: |st| st.nav_then(Page::Overview, |st| overview::open_named(st, NamedKind::Unit)),
        },
    ),
    (
        'r',
        Shortcut {
            id: "sc-r",
            label: "Receive money",
            keys: &["Alt", "R"],
            run: |st| st.nav_then(Page::Payments, |st| payments::open_payment(st, true)),
        },
    ),
    (
        's',
        Shortcut {
            id: "sc-s",
            label: "Settings",
            keys: &["Alt", "S"],
            run: |st| st.nav(Page::Settings),
        },
    ),
];
