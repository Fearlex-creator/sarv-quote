//! The counter. Tap a product, read one total, take the money.
//! The sale is a real invoice: the same draft, delivery, and payment the books already know.

use rune::{Cx, El, TextStyle, col, row, text};
use rune_kit::ComboboxState;

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::pickers;
use crate::ui::views::sales::{SaleLine, Ticket, post_ticket};

pub struct PosLine {
    pub item_id: String,
    pub name: String,
    pub qty: i64,
    /// Catalog price in rials, locked when the line is added.
    pub price_rials: i64,
}

pub struct PosState {
    pub lines: Vec<PosLine>,
    pub customer_id: String,
    pub customer_combo: ComboboxState,
    pub posting: bool,
    pub post_key: String,
    pub draft_id: String,
}

impl Default for PosState {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            customer_id: String::new(),
            customer_combo: ComboboxState::default(),
            posting: false,
            post_key: String::new(),
            draft_id: String::new(),
        }
    }
}

fn items(s: &State) -> Vec<serde_json::Value> {
    s.books.workspace["resources"]["items"].as_array().cloned().unwrap_or_default()
}

fn barcode(s: &State, id: &str) -> String {
    crate::data::s(&s.books.workspace["product_profiles"][id]["barcode"])
}

pub fn product_matches(name: &str, sku: &str, id: &str, barcode: &str, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }
    let latin = crate::i18n::latin_digits(&query);
    [name, sku, id, barcode].iter().any(|field| {
        let field = field.to_lowercase();
        field.contains(&query) || (!latin.is_empty() && field.contains(&latin))
    })
}

fn matches(s: &State, item: &serde_json::Value, query: &str) -> bool {
    let id = crate::data::s(&item["id"]);
    product_matches(
        &crate::data::s(&item["name"]),
        &crate::data::s(&item["sku"]),
        &id,
        &barcode(s, &id),
        query,
    )
}

fn display_digits(s: &State, rials: i64) -> String {
    if s.fmt.unit == crate::i18n::Unit::Toman { (rials / 10).to_string() } else { rials.to_string() }
}

fn total_rials(s: &State) -> i64 {
    s.pos.lines.iter().fold(0i64, |sum, line| sum.saturating_add(line.price_rials.saturating_mul(line.qty)))
}

fn ensure_keys(st: &mut State) {
    if !st.pos.post_key.is_empty() {
        return;
    }
    let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    st.pos.post_key = format!("pos-{stamp}");
    st.pos.draft_id = format!("draft-{stamp}");
}

fn add_item(st: &mut State, item_id: String) {
    if st.pos.posting {
        return;
    }
    let price = pickers::item_sale_price(st, &item_id);
    let Ok(rials) = price.parse::<i64>() else {
        st.toast(Tone::Danger, st.fmt.t("This product has no sale price."));
        return;
    };
    ensure_keys(st);
    if let Some(line) = st.pos.lines.iter_mut().find(|l| l.item_id == item_id) {
        line.qty = line.qty.saturating_add(1);
        return;
    }
    let name = items(st)
        .into_iter()
        .find(|i| crate::data::s(&i["id"]) == item_id)
        .map(|i| crate::data::s(&i["name"]))
        .unwrap_or_else(|| item_id.clone());
    st.pos.lines.push(PosLine { item_id, name, qty: 1, price_rials: rials });
}

fn step(st: &mut State, item_id: &str, delta: i64) {
    if st.pos.posting {
        return;
    }
    let Some(idx) = st.pos.lines.iter().position(|l| l.item_id == item_id) else {
        return;
    };
    let next = st.pos.lines[idx].qty + delta;
    if next <= 0 {
        st.pos.lines.remove(idx);
    } else {
        st.pos.lines[idx].qty = next;
    }
    if st.pos.lines.is_empty() {
        st.pos.post_key.clear();
        st.pos.draft_id.clear();
    }
}

fn clear_ticket(st: &mut State) {
    st.pos.lines.clear();
    st.pos.customer_id.clear();
    st.pos.customer_combo = ComboboxState::default();
    st.pos.post_key.clear();
    st.pos.draft_id.clear();
    st.pos.posting = false;
    st.forms.set("pos-received", "");
}

fn charge(st: &mut State, method: usize) {
    if st.pos.posting {
        return;
    }
    if st.pos.lines.is_empty() {
        st.toast(Tone::Danger, st.fmt.t("Add at least one product."));
        return;
    }
    if method == 2 && st.pos.customer_id.is_empty() {
        st.toast(Tone::Danger, st.fmt.t("Choose a customer for a sale on account."));
        return;
    }
    if method == 0 {
        let tendered = st.parse_money_input(&st.forms.get("pos-received"));
        if tendered > 0 && tendered < total_rials(st) {
            st.toast(Tone::Danger, st.fmt.t("The cash received is less than the total."));
            return;
        }
    }
    ensure_keys(st);
    st.pos.posting = true;
    let lines = st
        .pos
        .lines
        .iter()
        .map(|l| SaleLine {
            id: format!("pos-{}", l.item_id),
            item_id: l.item_id.clone(),
            qty: l.qty.to_string(),
            price: display_digits(st, l.price_rials),
            discount: "0".into(),
        })
        .collect();
    let ticket = Ticket {
        draft_id: st.pos.draft_id.clone(),
        post_key: st.pos.post_key.clone(),
        customer_id: st.pos.customer_id.clone(),
        lines,
        pay_method: method,
        note: "Goods sales order".into(),
        received: String::new(),
    };
    if post_ticket(st, &ticket) {
        clear_ticket(st);
        st.toast(Tone::Success, st.fmt.t("Posted"));
    }
    st.pos.posting = false;
}

fn step_btn(ui: &Ui, id: String, label: &str, on_press: impl Fn(&mut State) + 'static) -> El {
    row()
        .key(id)
        .role("button")
        .label(label.to_string())
        .w(36.0)
        .h(36.0)
        .radius(18.0)
        .center()
        .fill(ui.k.c.layer_02)
        .on_press(on_press)
        .child(text(label.to_string()).font(ui.k.ty.ui_medium).color(ui.k.c.text_base))
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let query = s.forms.get("pos-search").trim().to_lowercase();
    let catalog_items: Vec<_> = items(s).into_iter().filter(|item| matches(s, item, &query)).take(48).collect();

    let mut tiles = row().key("pos-products").w_full().gap(10.0).flex_wrap();
    if catalog_items.is_empty() {
        tiles = tiles.child(ui.muted(ui.t("Nothing to show")));
    }
    for item in &catalog_items {
        let id = crate::data::s(&item["id"]);
        let name = crate::data::s(&item["name"]);
        let price = pickers::item_sale_price(s, &id);
        let price_label = if price.is_empty() { "—".into() } else { ui.f.money(&price) };
        let count = s.pos.lines.iter().find(|l| l.item_id == id).map(|l| l.qty).unwrap_or(0);
        let pressed = id.clone();
        let mut tile = col()
            .key(format!("pos-item-{id}"))
            .role("button")
            .label(name.clone())
            .w(168.0)
            .h(88.0)
            .radius(14.0)
            .px(14.0)
            .py(12.0)
            .gap(6.0)
            .fill(if count > 0 { ui.k.c.layer_03 } else { ui.k.c.layer_02 })
            .on_press(move |st: &mut State| add_item(st, pressed.clone()));
        tile = tile.child(text(name).font(ui.k.ty.ui_medium).color(ui.k.c.text_base).truncate());
        let mut foot = row().w_full().center_y().child(ui.muted(price_label).grow(1.0).min_w(0.0));
        if count > 0 {
            foot = foot.child(text(ui.f.digits(&count.to_string())).font(ui.k.ty.ui_medium).color(ui.k.c.text_base));
        }
        tiles = tiles.child(tile.child(foot));
    }

    let catalog = col()
        .key("pos-catalog")
        .grow(1.0)
        .min_w(280.0)
        .min_h(0.0)
        .gap(12.0)
        .child(ui.field(s, "pos-search", &ui.t("Search")))
        .child(tiles.scroll(rune::ScrollSpec { stick_bottom: false }).grow(1.0).min_h(0.0));

    let mut lines = col().key("pos-lines").w_full().gap(2.0);
    if s.pos.lines.is_empty() {
        lines = lines.child(
            col()
                .w_full()
                .py(28.0)
                .center()
                .child(ui.muted(ui.t("Tap a product to start a sale."))),
        );
    }
    for line in &s.pos.lines {
        let id = line.item_id.clone();
        let minus_id = id.clone();
        let plus_id = id.clone();
        let amount = line.price_rials.saturating_mul(line.qty);
        lines = lines.child(
            row()
                .key(format!("pos-line-{id}"))
                .w_full()
                .gap(8.0)
                .py(8.0)
                .center_y()
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .gap(2.0)
                        .child(text(line.name.clone()).font(ui.k.ty.ui).color(ui.k.c.text_base).truncate())
                        .child(ui.muted(ui.f.money(&line.price_rials.to_string()))),
                )
                .child(
                    row()
                        .gap(6.0)
                        .center_y()
                        .child(step_btn(ui, format!("pos-minus-{id}"), "−", move |st| step(st, &minus_id, -1)))
                        .child(
                            row()
                                .w(28.0)
                                .center()
                                .child(text(ui.f.digits(&line.qty.to_string())).font(ui.k.ty.ui_medium).color(ui.k.c.text_base)),
                        )
                        .child(step_btn(ui, format!("pos-plus-{plus_id}"), "+", move |st| step(st, &plus_id, 1))),
                )
                .child(
                    row()
                        .w(96.0)
                        .child(text(ui.f.money(&amount.to_string())).font(ui.k.ty.ui).color(ui.k.c.text_base).truncate()),
                ),
        );
    }

    let total = total_rials(s);
    let tendered = s.parse_money_input(&s.forms.get("pos-received"));
    let change = tendered.saturating_sub(total);
    let total_style = TextStyle { size: 34.0, weight: 600.0, line_height: 40.0, ..ui.k.ty.body_medium };
    let mut summary = col().w_full().gap(4.0).child(
        row()
            .w_full()
            .center_y()
            .child(ui.muted(ui.t("Total")).grow(1.0))
            .child(text(ui.f.money(&total.to_string())).font(total_style).color(ui.k.c.text_base)),
    );
    if tendered > total && total > 0 {
        summary = summary.child(
            row()
                .key("pos-change")
                .w_full()
                .center_y()
                .child(ui.muted(ui.t("Change")).grow(1.0))
                .child(ui.heading(ui.f.money(&change.to_string()))),
        );
    }

    let customers = pickers::party_options(s, true, false);
    let current = if s.pos.customer_id.is_empty() { None } else { Some(s.pos.customer_id.as_str()) };
    let customer = pickers::entity_combo(
        ui,
        layer,
        "pos-customer",
        &s.pos.customer_combo,
        |st| &mut st.pos.customer_combo,
        &customers,
        current,
        &ui.t("Walk-in customer"),
        pickers::PICKER_WIDTH,
        |st, id| st.pos.customer_id = id.to_string(),
    );

    let mut ticket = col()
        .w_full()
        .gap(12.0)
        .grow(1.0)
        .min_h(0.0)
        .child(customer)
        .child(lines.scroll(rune::ScrollSpec { stick_bottom: false }).grow(1.0).min_h(0.0))
        .child(summary);
    if !s.pos.lines.is_empty() {
        ticket = ticket.child(ui.labeled("Amount received", ui.field(s, "pos-received", &ui.f.money(&total.to_string()))));
    }
    ticket = ticket
        .child(
            row()
                .w_full()
                .gap(8.0)
                .child(ui.button(Btn::Primary, "pos-cash", &ui.t("Cash"), None, |st| charge(st, 0)).grow(1.0))
                .child(ui.button(Btn::Glass, "pos-card", &ui.t("Card"), None, |st| charge(st, 1)).grow(1.0)),
        )
        .child(ui.button(Btn::Ghost, "pos-account", &ui.t("On account"), None, |st| charge(st, 2)));
    if !s.pos.lines.is_empty() {
        ticket = ticket.child(ui.text_btn("pos-clear", &ui.t("Clear"), clear_ticket));
    }

    let body = row()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .gap(16.0)
        .flex_wrap()
        .child(catalog)
        .child(ui.frost("pos-ticket", ticket).w(360.0).grow(1.0).min_w(300.0).min_h(0.0));

    ui.page("pos-screen", &ui.t("POS"), &s.books.company_name(), Vec::new(), body)
}
