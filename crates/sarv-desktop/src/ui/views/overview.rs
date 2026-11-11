//! Overview: the books at a glance, on a frosted sheet, with the setup guide the business still owes.

use std::borrow::Cow;

use rune::{Align, Cx, El, ScrollSpec, TextStyle, col, el, row, text};
use rune_kit::{Cell, Switch, Table, TableColumn, TableState};
use serde_json::json;

use crate::app::{Page, State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;
use crate::ui::resources::ResourceKind;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum GuideMode {
    #[default]
    Auto,
    Open,
    Closed,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum NamedKind {
    #[default]
    Warehouse,
    Unit,
}

#[derive(Default)]
pub struct OverviewState {
    pub table: TableState,
    pub guide: GuideMode,
    pub starter_open: bool,
    pub starter_busy: bool,
    pub pick: [bool; 7],
    pub named_open: bool,
    pub named_kind: NamedKind,
}

const WAREHOUSES: [&str; 2] = ["Main warehouse", "Storefront / Shop"];
const UNITS: [&str; 5] = ["Piece", "Box", "Kilogram", "Meter", "Pack"];

struct Counts {
    warehouses: usize,
    units: usize,
    items: usize,
    parties: usize,
    transactions: usize,
    drafts: usize,
    journals: usize,
}

fn resource_len(s: &State, key: &str) -> usize {
    s.books.workspace["resources"][key].as_array().map(|a| a.len()).unwrap_or(0)
}

fn counts(s: &State) -> Counts {
    Counts {
        warehouses: resource_len(s, "warehouses"),
        units: resource_len(s, "units"),
        items: resource_len(s, "items"),
        parties: resource_len(s, "parties"),
        transactions: s.books.count("sales_count") + s.books.count("purchase_count") + s.books.count("payment_count"),
        drafts: s.books.list("drafts").len(),
        journals: s.books.count("journal_count"),
    }
}

fn has_named(s: &State, bucket: &str, name: &str) -> bool {
    let want = name.to_lowercase();
    s.books.workspace["resources"][bucket]
        .as_array()
        .is_some_and(|rows| rows.iter().any(|r| crate::data::s(&r["name"]).to_lowercase() == want))
}

fn profile_ready(s: &State) -> bool {
    let p = &s.books.workspace["business_profile"];
    let details = ["phone", "registration_id", "tax_id", "address"]
        .iter()
        .any(|k| !crate::data::s(&p[k]).trim().is_empty());
    s.books.has_company() && (details || s.books.initialized())
}

fn guide_expanded(s: &State, done: usize) -> bool {
    match s.overview.guide {
        GuideMode::Open => true,
        GuideMode::Closed => false,
        GuideMode::Auto => done < 5,
    }
}

/// Auto unmounts when all five steps are done. Open still shows the card; Closed keeps the
/// collapsed header so the user can reopen (steps stay hidden via `guide_expanded`).
fn guide_mounted(s: &State, done: usize) -> bool {
    match s.overview.guide {
        GuideMode::Open => true,
        GuideMode::Closed => true,
        GuideMode::Auto => done < 5,
    }
}

fn recent_journals(s: &State) -> Vec<serde_json::Value> {
    s.books.list("journals").into_iter().take(6).collect()
}

fn figure(ui: &Ui, key: &str, icon: &'static rune::Icon, label: &str, value: String, detail: String, tone: Tone) -> El {
    let style = TextStyle { size: 22.0, weight: 560.0, line_height: 28.0, ..ui.k.ty.body_medium };
    col()
        .key(key)
        .grow(1.0)
        .min_w(168.0)
        .gap(2.0)
        .child(
            row()
                .gap(6.0)
                .center_y()
                .child(ui.ic(icon, 14.0, ui.tone_fg(tone)))
                .child(ui.muted(label)),
        )
        .child(text(value).font(style).color(ui.k.c.text_base))
        .child(ui.muted(detail))
}

fn mark(ui: &Ui, n: usize, done: bool) -> El {
    let fill = if done { ui.k.c.success.fg.alpha(0.16) } else { ui.k.c.layer_02 };
    let inner = if done {
        ui.ic(&icons::CHECK, 12.0, ui.k.c.success.fg)
    } else {
        text(ui.f.digits(&format!("{}", n))).font(ui.k.ty.caption()).color(ui.k.c.text_muted)
    };
    row().w(22.0).h(22.0).center().radius(11.0).fill(fill).child(inner)
}

fn progress(ui: &Ui, done: usize) -> El {
    let filled = done as f32;
    let rest = (5.0 - filled).max(0.0);
    let track = row().key("setup-progress").w_full().h(4.0).radius(2.0).fill(ui.k.c.layer_02);
    if done == 0 {
        track
    } else if done >= 5 {
        track.child(el().grow(1.0).h_full().radius(2.0).fill(ui.k.c.success.fg))
    } else {
        track
            .child(el().grow(filled).h_full().radius(2.0).fill(ui.k.c.success.fg))
            .child(el().grow(rest))
    }
}

fn step(ui: &Ui, key: &str, n: usize, title: &str, detail: &str, done: bool, badge: String, actions: Vec<El>) -> El {
    let mut body = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(4.0)
        .child(
            row()
                .w_full()
                .center_y()
                .gap(8.0)
                .child(ui.title(title).grow(1.0))
                .child(ui.badge(if done { Tone::Success } else { Tone::Warning }, badge)),
        )
        .child(ui.muted(detail));
    if !actions.is_empty() {
        let actions = actions.into_iter().fold(row().w_full().gap(8.0).flex_wrap(), |r, a| r.child(a));
        body = body.child(actions);
    }
    row()
        .key(key)
        .w_full()
        .gap(12.0)
        .child(mark(ui, n, done))
        .child(body)
}

fn open_sale(st: &mut State) {
    st.sales.start_new();
    st.forms.set("sale-note", "");
    st.forms.set("sale-received", "");
    st.nav(Page::Sales);
}

fn open_purchase(st: &mut State) {
    st.purchases.start_new();
    st.nav(Page::Purchases);
}

fn open_customer(st: &mut State, supplier: bool) {
    st.contacts.modal_open = true;
    st.contacts.is_supplier = supplier;
    st.forms.set("contact-name", "");
    st.forms.set("contact-phone", "");
    st.forms.set("contact-email", "");
    st.forms.set("contact-notes", "");
    st.nav(Page::Contacts);
}

fn open_product(st: &mut State) {
    st.inventory.modal_open = true;
    st.inventory.unit_id = crate::ui::pickers::first_unit(st);
    st.inventory.supplier_id.clear();
    st.inventory.is_service = false;
    st.forms.set("item-name", "");
    st.forms.set("item-sku", "");
    st.forms.set("item-price", "");
    st.forms.set("item-cost", "");
    st.forms.set("item-barcode", "");
    st.forms.set("item-notes", "");
    st.nav(Page::Inventory);
}

fn open_payment(st: &mut State) {
    st.payments.modal_open = true;
    st.payments.receive = true;
    st.payments.party_id.clear();
    st.payments.party_combo = Default::default();
    st.payments.funds = 0;
    st.forms.set("pay-amount", "");
    st.forms.set("pay-ref", "");
    st.forms.set("pay-reason", "");
    st.nav(Page::Payments);
}

fn open_named(st: &mut State, kind: NamedKind) {
    st.overview.starter_open = false;
    st.overview.named_kind = kind;
    st.overview.named_open = true;
    st.forms.set("overview-resource-name", "");
}

fn arm_starter(st: &mut State) {
    for (i, name) in WAREHOUSES.iter().enumerate() {
        let exists = has_named(st, "warehouses", &st.fmt.t(name));
        st.overview.pick[i] = i == 0 && !exists;
    }
    for (i, name) in UNITS.iter().enumerate() {
        let exists = has_named(st, "units", &st.fmt.t(name));
        st.overview.pick[i + 2] = i < 3 && !exists;
    }
    st.overview.named_open = false;
    st.overview.starter_busy = false;
    st.overview.starter_open = true;
}

fn save_named(st: &mut State) {
    let name = st.forms.get("overview-resource-name").trim().to_string();
    if name.is_empty() {
        st.toast(Tone::Warning, st.fmt.t("Name"));
        return;
    }
    let kind = match st.overview.named_kind {
        NamedKind::Warehouse => ResourceKind::Warehouse,
        NamedKind::Unit => ResourceKind::Unit,
    };
    if st.create_named(kind, &name).is_ok() {
        st.overview.named_open = false;
    }
}

fn save_starter(st: &mut State) {
    if st.overview.starter_busy {
        return;
    }
    st.overview.starter_busy = true;
    let mut made = 0usize;
    let mut n = 0u32;
    let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    for (i, name) in WAREHOUSES.iter().enumerate() {
        let localized = st.fmt.t(name);
        if st.overview.pick[i] && !has_named(st, "warehouses", &localized) {
            n += 1;
            let id = format!("ui-{stamp}-w{n}");
            if st
                .run(
                    "resource",
                    json!({
                        "kind": "warehouse",
                        "id": id,
                        "name": localized,
                        "reason": "Register starter warehouse"
                    }),
                    None,
                )
                .is_err()
            {
                st.overview.starter_busy = false;
                return;
            }
            made += 1;
        }
    }
    for (i, name) in UNITS.iter().enumerate() {
        let localized = st.fmt.t(name);
        if st.overview.pick[i + 2] && !has_named(st, "units", &localized) {
            n += 1;
            let id = format!("ui-{stamp}-u{n}");
            if st
                .run(
                    "resource",
                    json!({
                        "kind": "unit",
                        "id": id,
                        "name": localized,
                        "reason": "Register starter unit"
                    }),
                    None,
                )
                .is_err()
            {
                st.overview.starter_busy = false;
                return;
            }
            made += 1;
        }
    }
    st.overview.starter_busy = false;
    if made > 0 {
        st.toast(Tone::Success, st.fmt.t("Essentials added to your business"));
        st.overview.starter_open = false;
    }
}

fn starter_switch(s: &State, ui: &Ui, index: usize, bucket: &str, name: &'static str) -> El {
    let localized = ui.t(name);
    let exists = has_named(s, bucket, &localized);
    let checked = exists || s.overview.pick[index];
    let i = index;
    row()
        .w_full()
        .center_y()
        .gap(8.0)
        .child(
            Switch::new(checked)
                .label(localized)
                .disabled(exists || s.overview.starter_busy)
                .on_toggle(move |st: &mut State| {
                    if !st.overview.starter_busy {
                        st.overview.pick[i] = !st.overview.pick[i];
                    }
                })
                .build(&ui.k)
                .grow(1.0)
                .key(format!("starter-{index}")),
        )
        .child(if exists { ui.badge(Tone::Info, ui.t("Already exists")) } else { el() })
}

fn mount_starter(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let mut warehouses = col().w_full().gap(8.0).child(ui.heading(ui.t("Default warehouses")));
    for (i, name) in WAREHOUSES.iter().enumerate() {
        warehouses = warehouses.child(starter_switch(s, ui, i, "warehouses", name));
    }
    let mut units = col().w_full().gap(8.0).child(ui.heading(ui.t("Standard measurement units")));
    for (i, name) in UNITS.iter().enumerate() {
        units = units.child(starter_switch(s, ui, i + 2, "units", name));
    }
    let body = col().w_full().gap(16.0).child(warehouses).child(units);
    let label = if s.overview.starter_busy {
        ui.t("Saving…")
    } else {
        ui.t("Create selected essentials")
    };
    ui.dialog_described(
        layer,
        &mut Vec::new(),
        "overview-starter",
        &ui.t("Starter kit: Warehouses & units"),
        &ui.t("Select standard units and warehouses to add to this business in one click."),
        true,
        520.0,
        body,
        vec![
            ui.button(Btn::Ghost, "overview-cancel-starter", &ui.t("Cancel"), None, |st| {
                if !st.overview.starter_busy {
                    st.overview.starter_open = false;
                }
            }),
            ui.button(Btn::Primary, "Create selected essentials", &label, None, save_starter),
        ],
        |st| {
            if !st.overview.starter_busy {
                st.overview.starter_open = false;
            }
        },
    );
}

fn mount_named(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let title = match s.overview.named_kind {
        NamedKind::Warehouse => ui.t("New warehouse"),
        NamedKind::Unit => ui.t("New unit"),
    };
    let body = ui.labeled("Name", ui.field(s, "overview-resource-name", ""));
    ui.dialog_described(
        layer,
        &mut Vec::new(),
        "overview-named",
        &title,
        "",
        true,
        420.0,
        body,
        vec![
            ui.button(Btn::Ghost, "overview-cancel-named", &ui.t("Cancel"), None, |st| st.overview.named_open = false),
            ui.button(Btn::Primary, "overview-save-named", &ui.t("Save"), Some(&icons::CHECK), save_named),
        ],
        |st| st.overview.named_open = false,
    );
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let c = counts(s);
    let profile_done = profile_ready(s);
    let opening_done = s.books.initialized();
    let stock_done = c.warehouses > 0 && c.units > 0;
    let catalog_done = c.items > 0;
    let contacts_done = c.parties > 0 && c.transactions > 0;
    let done = [profile_done, opening_done, stock_done, catalog_done, contacts_done]
        .iter()
        .filter(|d| **d)
        .count();
    let expanded = guide_expanded(s, done);

    let (_, to) = s.books.range();
    let unit = if s.fmt.unit == crate::i18n::Unit::Rial { "IRR".to_string() } else { ui.t("toman") };
    let as_of = row()
        .w_full()
        .center_y()
        .gap(8.0)
        .child(ui.muted(ui.t("As of")))
        .child(ui.title(ui.f.date(&to)))
        .child(ui.muted(unit));

    let quick = row()
        .key("quick-actions")
        .w_full()
        .gap(8.0)
        .flex_wrap()
        .child(ui.wrap_btn(Btn::Primary, "New sales invoice", Some(&icons::SALES), open_sale))
        .child(ui.wrap_btn(Btn::Glass, "New purchase order", Some(&icons::PURCHASES), open_purchase))
        .child(ui.wrap_btn(Btn::Glass, "New customer", Some(&icons::CONTACTS), |st| open_customer(st, false)))
        .child(ui.wrap_btn(Btn::Glass, "New product", Some(&icons::INVENTORY), open_product))
        .child(ui.wrap_btn(Btn::Glass, "Record a payment", Some(&icons::PAYMENTS), open_payment));

    let profit = if !s.books.report["profit_loss"]["profit"].is_null() {
        ui.f.money(&crate::data::s(&s.books.report["profit_loss"]["profit"]))
    } else {
        ui.f.money("0")
    };
    let metrics = row()
        .key("overview-metrics")
        .w_full()
        .gap(16.0)
        .flex_wrap()
        .child(figure(ui, "metric-cash", &icons::WALLET, &ui.t("Cash and bank"), ui.f.money(&s.books.position("cash")), ui.t("Available funds"), Tone::Info))
        .child(figure(ui, "metric-rec", &icons::SALES, &ui.t("Receivables"), ui.f.money(&s.books.position("receivables")), ui.t("Open customer invoices"), Tone::Success))
        .child(figure(ui, "metric-pay", &icons::PURCHASES, &ui.t("Payables"), ui.f.money(&s.books.position("payables")), ui.t("Due supplier bills"), Tone::Warning))
        .child(figure(ui, "metric-profit", &icons::REPORTS, &ui.t("Profit this period"), profit, ui.t("Net profit"), Tone::Info));

    let rec = &s.books.report["reconciliation"];
    let controls = if rec.is_null() {
        "—".to_string()
    } else if rec["reconciled"].as_bool() == Some(true) {
        ui.t("Balanced")
    } else {
        ui.t("Difference")
    };
    let summary = row()
        .key("overview-summary")
        .w_full()
        .gap(8.0)
        .flex_wrap()
        .child(
            ui.text_btn("overview-drafts", &format!("{}  {}", ui.t("Drafts"), ui.f.digits(&c.drafts.to_string())), |st| {
                st.sales.tab_idx = 1;
                st.nav(Page::Sales);
            })
            .grow(1.0)
            .min_w(140.0),
        )
        .child(
            ui.text_btn("overview-journals", &format!("{}  {}", ui.t("Journals"), ui.f.digits(&c.journals.to_string())), |st| {
                st.nav(Page::Accounting);
            })
            .grow(1.0)
            .min_w(140.0),
        )
        .child(
            ui.text_btn("overview-controls", &format!("{}  {}", ui.t("Controls"), controls), |st| st.nav(Page::Reports))
                .grow(1.0)
                .min_w(140.0),
        );

    let sheet = ui.frost(
        "overview-sheet",
        col().w_full().gap(14.0).child(as_of).child(quick).child(metrics).child(el().w_full().h(0.5).fill(ui.k.c.border_base)).child(summary),
    );

    let recent_body = if recent_journals(s).is_empty() {
        ui.empty(&icons::REPORTS, "No recent entries yet.", "Post a sale or journal to see activity here.").key("recent-empty")
    } else {
        Table::new(
            |s: &State| &s.overview.table,
            |s: &mut State| &mut s.overview.table,
            |s: &State| recent_journals(s).len(),
            |s: &State, i| Cow::Owned(format!("rec-{}", crate::data::s(&recent_journals(s)[i]["id"]))),
        )
        .name(ui.t("Recent entries"))
        .column(TableColumn::new("date", ui.t("Date")).width(110.0).min(88.0))
        .column(TableColumn::new("ref", ui.t("Reference")).width(130.0).min(96.0))
        .column(TableColumn::new("desc", ui.t("Description")).flex(2.0).min(140.0))
        .column(TableColumn::new("debit", ui.t("Debit")).width(140.0).min(96.0).align(Align::End))
        .column(TableColumn::new("credit", ui.t("Credit")).width(140.0).min(96.0).align(Align::End))
        .cell(|s: &State, i, col| {
            let list = recent_journals(s);
            let Some(j) = list.get(i) else {
                return Cell::text("");
            };
            let id = crate::data::s(&j["id"]);
            let short = if id.len() > 8 { &id[..8] } else { id.as_str() };
            match col {
                "date" => Cell::text(s.fmt.date(&crate::data::s(&j["date"]))).muted(),
                "ref" => Cell::text(format!("#{short}")),
                "desc" => Cell::text(crate::data::s(&j["reason"])),
                "debit" => Cell::text(s.fmt.money(&crate::data::s(&j["total_debit"]))).mono(),
                _ => Cell::text(s.fmt.money(&crate::data::s(&j["total_credit"]))).mono(),
            }
        })
        .zebra(true)
        .build(s, &ui.k)
        .key("recent-table")
        .grow(1.0)
        .min_h(0.0)
    };

    let mut recent = col()
        .key("recent-entries-panel")
        .w_full()
        .gap(10.0)
        .child(
            row()
                .w_full()
                .center_y()
                .child(ui.heading(ui.t("Recent entries")).grow(1.0))
                .child(ui.btn(Btn::Ghost, "View all", Some(&icons::ARROW), |st| st.nav(Page::Accounting))),
        )
        .child(recent_body);
    // An open guide is the long column, so it scrolls on its own and the ledger stays on screen.
    // A finished guide gets out of the way and the ledger fills the window.
    if expanded {
        recent = recent.h(200.0);
    } else {
        recent = recent.grow(1.0).min_h(160.0);
    }

    let failed = s.books.workspace["integrity"]["Failed"]["message"].as_str().unwrap_or("").to_string();
    let mut upper = col().key("overview-upper").w_full().gap(14.0).child(sheet);
    if guide_mounted(s, done) {
        let prefix = crate::data::s(&s.books.workspace["business_profile"]["invoice_prefix"]);
        let profile_badge = if !prefix.trim().is_empty() {
            prefix
        } else {
            let name = s.books.company_name();
            if name.is_empty() { ui.t("Ready") } else { name }
        };
        let steps = if expanded {
            col()
                .w_full()
                .gap(14.0)
                .child(ui.muted(ui.t(
                    "Complete these steps to get your accounting workspace, catalog, and invoicing ready.",
                )))
                .child(step(
                    ui,
                    "setup-step-profile",
                    1,
                    &ui.t("Business identity & invoice profile"),
                    &ui.t("Phone, registration/tax IDs, address, and default invoice prefix for printed PDFs."),
                    profile_done,
                    profile_badge,
                    vec![ui.compact_btn(Btn::Glass, "Edit business profile", |st| {
                        crate::ui::views::settings::open_business(st);
                        st.nav(Page::Settings);
                    })],
                ))
                .child(step(
                    ui,
                    "setup-step-opening",
                    2,
                    &ui.t("Opening balances"),
                    &ui.t("Confirm zero starting balances or validate an opening CSV import to unlock posting."),
                    opening_done,
                    ui.t(if opening_done { "Balanced" } else { "Action needed" }),
                    Vec::new(),
                ))
                .child(step(
                    ui,
                    "setup-step-warehouses",
                    3,
                    &ui.t("Warehouses & measurement units"),
                    &ui.t("Define where stock is held and how items are counted (e.g., Piece, Box, Kg)."),
                    stock_done,
                    format!("{} · {}", ui.f.digits(&c.warehouses.to_string()), ui.f.digits(&c.units.to_string())),
                    vec![
                        ui.compact_btn(
                            if c.warehouses == 0 && c.units == 0 { Btn::Primary } else { Btn::Glass },
                            "Quick-add essentials",
                            arm_starter,
                        ),
                        ui.compact_btn(Btn::Glass, "Add warehouse", |st| open_named(st, NamedKind::Warehouse)),
                        ui.compact_btn(Btn::Glass, "Add unit", |st| open_named(st, NamedKind::Unit)),
                    ],
                ))
                .child(step(
                    ui,
                    "setup-step-products",
                    4,
                    &ui.t("Products & price catalog"),
                    &ui.t("Register your goods with default purchase/selling prices and low-stock alerts."),
                    catalog_done,
                    format!("{} {}", ui.f.digits(&c.items.to_string()), ui.t("Products")),
                    vec![ui.compact_btn(
                        if stock_done && c.items == 0 { Btn::Primary } else { Btn::Glass },
                        "Add product",
                        open_product,
                    )],
                ))
                .child(step(
                    ui,
                    "setup-step-contacts",
                    5,
                    &ui.t("Contacts & first transaction"),
                    &ui.t("Add your first customer or supplier and issue a quotation, sales invoice, or purchase order."),
                    contacts_done,
                    format!("{} {}", ui.f.digits(&c.parties.to_string()), ui.t("Contacts")),
                    vec![
                        ui.compact_btn(
                            if catalog_done && c.parties == 0 { Btn::Primary } else { Btn::Glass },
                            "Add customer",
                            |st| open_customer(st, false),
                        ),
                        ui.compact_btn(Btn::Glass, "Add supplier", |st| open_customer(st, true)),
                        ui.compact_btn(
                            if c.parties > 0 && c.transactions == 0 { Btn::Primary } else { Btn::Glass },
                            "Create sales invoice",
                            open_sale,
                        ),
                        ui.compact_btn(Btn::Glass, "Create purchase order", open_purchase),
                    ],
                ))
        } else {
            col()
        };
        let toggle_key = if expanded { "Hide guide" } else { "Setup guide" };
        let guide = ui.frost(
            "setup-guide-card",
            col()
                .w_full()
                .gap(12.0)
                .child(
                    row()
                        .w_full()
                        .center_y()
                        .gap(12.0)
                        .child(ui.ic(&icons::SPARKLE, 16.0, ui.k.c.info.icon))
                        .child(
                            col()
                                .grow(1.0)
                                .min_w(0.0)
                                .gap(2.0)
                                .child(ui.heading(ui.t("Business setup guide")))
                                .child(ui.muted(ui.f.tf(
                                    "{done} of {total} completed",
                                    &[("done", &ui.f.digits(&done.to_string())), ("total", &ui.f.digits("5"))],
                                ))),
                        )
                        .child(ui.compact_btn(Btn::Ghost, toggle_key, |st| {
                            let open_now = guide_expanded(st, {
                                let c = counts(st);
                                [
                                    profile_ready(st),
                                    st.books.initialized(),
                                    c.warehouses > 0 && c.units > 0,
                                    c.items > 0,
                                    c.parties > 0 && c.transactions > 0,
                                ]
                                .iter()
                                .filter(|d| **d)
                                .count()
                            });
                            st.overview.guide = if open_now { GuideMode::Closed } else { GuideMode::Open };
                        })),
                )
                .child(progress(ui, done))
                .child(steps),
        );
        upper = upper.child(guide);
    }
    if expanded {
        upper = upper.scroll(ScrollSpec { stick_bottom: false }).grow(1.0).min_h(0.0);
    }
    let mut body = col().w_full().gap(14.0).grow(1.0).min_h(0.0);
    if !failed.is_empty() {
        body = body.child(ui.kit_card().title(failed, Some(&icons::INFO)).build(&ui.k).key("integrity-notice"));
    }
    body = body.child(upper).child(recent);

    if s.overview.starter_open {
        mount_starter(s, ui, layer);
    }
    if s.overview.named_open {
        mount_named(s, ui, layer);
    }

    ui.page(
        "overview-screen",
        &ui.t("Overview"),
        &s.books.company_name(),
        vec![ui.btn(Btn::Primary, "New journal", Some(&icons::PLUS), |st| {
            st.accounting.modal_open = true;
            st.accounting.debit_account.clear();
            st.accounting.credit_account.clear();
            st.forms.set("journal-amount", "");
            st.forms.set("journal-reason", "");
            st.nav(Page::Accounting);
        })],
        body,
    )
}
