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

pub const CREATE: &[(char, Shortcut)] = &[
    (
        'n',
        Shortcut {
            id: "sc-n",
            label: "New sales invoice",
            keys: &["Alt", "N"],
            run: |st| {
                sales::open_sale(st);
                st.nav(Page::Sales);
            },
        },
    ),
    (
        'c',
        Shortcut {
            id: "sc-c",
            label: "New customer",
            keys: &["Alt", "C"],
            run: |st| {
                contacts::open_new_contact(st, false);
                st.nav(Page::Contacts);
            },
        },
    ),
    (
        'v',
        Shortcut {
            id: "sc-v",
            label: "New supplier",
            keys: &["Alt", "V"],
            run: |st| {
                contacts::open_new_contact(st, true);
                st.nav(Page::Contacts);
            },
        },
    ),
    (
        'p',
        Shortcut {
            id: "sc-p",
            label: "New product",
            keys: &["Alt", "P"],
            run: |st| {
                inventory::open_new_product(st);
                st.nav(Page::Inventory);
            },
        },
    ),
    (
        'b',
        Shortcut {
            id: "sc-buy",
            label: "New purchase order",
            keys: &["Alt", "B"],
            run: |st| {
                purchases::open_purchase(st);
                st.nav(Page::Purchases);
            },
        },
    ),
    (
        'j',
        Shortcut {
            id: "sc-j",
            label: "New journal",
            keys: &["Alt", "J"],
            run: |st| {
                accounting::open_journal(st);
                st.nav(Page::Accounting);
            },
        },
    ),
    (
        't',
        Shortcut {
            id: "sc-t",
            label: "Transfer stock",
            keys: &["Alt", "T"],
            run: |st| {
                inventory::open_transfer(st);
                st.nav(Page::Inventory);
            },
        },
    ),
    (
        'w',
        Shortcut {
            id: "sc-w",
            label: "New warehouse",
            keys: &["Alt", "W"],
            run: |st| {
                overview::open_named(st, NamedKind::Warehouse);
                st.nav(Page::Overview);
            },
        },
    ),
    (
        'u',
        Shortcut {
            id: "sc-u",
            label: "New unit",
            keys: &["Alt", "U"],
            run: |st| {
                overview::open_named(st, NamedKind::Unit);
                st.nav(Page::Overview);
            },
        },
    ),
    (
        'r',
        Shortcut {
            id: "sc-r",
            label: "Receive money",
            keys: &["Alt", "R"],
            run: |st| {
                payments::open_payment(st, true);
                st.nav(Page::Payments);
            },
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
