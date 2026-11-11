//! Sales — list + Quick Sale facture (creatable customer/product Combobox, lines, pay).

use std::borrow::Cow;

use rune::{Align, Cx, El, col, row, text};
use rune_kit::{Cell, ComboboxState, Table, TableColumn, TableState};
use serde_json::json;

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;
use crate::ui::pickers;
use crate::ui::resources::ResourceKind;

#[derive(Clone, Debug)]
pub struct SaleLine {
    pub id: String,
    pub item_id: String,
    pub qty: String,
    pub price: String,
    pub discount: String,
}

impl SaleLine {
    fn blank() -> Self {
        Self {
            id: format!("line-{}", chrono::Utc::now().timestamp_millis()),
            item_id: String::new(),
            qty: "1".into(),
            price: String::new(),
            discount: "0".into(),
        }
    }
}

#[derive(Default)]
pub struct SalesState {
    pub modal_open: bool,
    pub tab_idx: usize,
    pub table: TableState,
    pub customer_id: String,
    pub customer_combo: ComboboxState,
    pub line_combos: Vec<ComboboxState>,
    pub lines: Vec<SaleLine>,
    pub pay_method: usize, // 0 cash, 1 bank, 2 later
    pub creating_item_for_line: Option<usize>,
    pub new_item_name: String,
    /// Stable across a double-click of Complete sale. A new dialog mints a new key.
    pub post_key: String,
    pub draft_id: String,
    pub posting: bool,
    pub selected: String,
}

impl SalesState {
    pub fn start_new(&mut self) {
        self.modal_open = true;
        self.customer_id.clear();
        self.customer_combo = ComboboxState::default();
        self.lines = vec![SaleLine::blank()];
        self.line_combos = vec![ComboboxState::default()];
        self.pay_method = 0;
        self.creating_item_for_line = None;
        self.new_item_name.clear();
        let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        self.post_key = format!("sale-{stamp}");
        self.draft_id = format!("draft-{stamp}");
        self.posting = false;
    }

    fn ensure_line_combos(&mut self) {
        while self.line_combos.len() < self.lines.len() {
            self.line_combos.push(ComboboxState::default());
        }
        self.line_combos.truncate(self.lines.len());
    }
}

fn sales_records(s: &State) -> Vec<serde_json::Value> {
    s.books.list("sales")
}

fn drafts(s: &State) -> Vec<serde_json::Value> {
    s.books.list("drafts")
}

fn build_draft_json(
    s: &State,
    draft_id: &str,
    customer: &str,
    reference: &str,
    lines: &[SaleLine],
    note: &str,
) -> serde_json::Value {
    let date = crate::data::today();
    let wh = pickers::first_warehouse(s);
    let display_unit = if s.fmt.unit == crate::i18n::Unit::Toman { "Toman" } else { "Rial" };
    // Machine dates are always ISO; Jalali is a display preference only.
    let date_display = "Iso";
    let draft_lines: Vec<_> = lines
        .iter()
        .filter(|l| !l.item_id.is_empty())
        .map(|l| {
            json!({
                "id": l.id,
                "item_id": l.item_id,
                "warehouse_id": wh,
                "unit_id": pickers::item_unit(s, &l.item_id),
                "quantity": crate::i18n::latin_digits(&l.qty).replace(',', ""),
                "unit_price": crate::ui::ops::typed_display_amount(&l.price),
                "line_discount": crate::ui::ops::typed_display_amount(&l.discount),
                "price_reference": "Manual price",
                "price_date": date,
                "override_reason": ""
            })
        })
        .collect();
    json!({
        "id": if draft_id.is_empty() { s.uid() } else { draft_id.to_string() },
        "revision": "0",
        "company_id": s.books.company_id,
        "period_id": s.books.period_id,
        "setup_id": s.setup_id(),
        "customer_id": customer,
        "reference": reference,
        "document_date": date,
        "financial_date": date,
        "date_display": date_display,
        "display_unit": display_unit,
        "document_discount": "0",
        "supplied_total": "0",
        "reason": note.trim(),
        "quotation": null,
        "lines": draft_lines
    })
}

fn ref_tail(post_key: &str) -> String {
    post_key.chars().rev().take(4).collect::<String>().chars().rev().collect()
}

fn next_order_ref(s: &State, post_key: &str) -> String {
    format!("ORD-{}-{}", s.books.count("sales_count") + 1, ref_tail(post_key))
}

fn next_invoice_ref(s: &State, post_key: &str) -> String {
    let prefix = s.books.workspace["business_profile"]["invoice_prefix"].as_str().unwrap_or("INV");
    format!("{prefix}-{}-{}", s.books.count("sales_count") + 1, ref_tail(post_key))
}

/// One counter sale: preview, save, deliver, invoice, and optionally take the money.
pub struct Ticket {
    pub draft_id: String,
    pub post_key: String,
    pub customer_id: String,
    pub lines: Vec<SaleLine>,
    pub pay_method: usize,
    pub note: String,
    /// Display-unit digits already typed. Empty settles the invoice in full.
    pub received: String,
}

pub fn post_ticket(st: &mut State, ticket: &Ticket) -> bool {
    let filled: Vec<_> = ticket.lines.iter().filter(|l| !l.item_id.is_empty()).cloned().collect();
    if filled.is_empty() {
        st.toast(Tone::Danger, st.fmt.t("Add at least one product."));
        return false;
    }
    for l in &filled {
        if crate::i18n::latin_digits(&l.qty).trim().is_empty() || l.price.trim().is_empty() {
            st.toast(Tone::Danger, st.fmt.t("Check the quantity on every line."));
            return false;
        }
    }
    let customer = if ticket.customer_id.is_empty() {
        match st.ensure_walk_in_customer() {
            Ok(id) => id,
            Err(e) => {
                st.toast_err(e);
                return false;
            }
        }
    } else {
        ticket.customer_id.clone()
    };
    let post_key = if ticket.post_key.is_empty() { st.uid() } else { ticket.post_key.clone() };
    let draft_id = if ticket.draft_id.is_empty() { st.uid() } else { ticket.draft_id.clone() };
    let reference = next_order_ref(st, &post_key);
    let invoice = next_invoice_ref(st, &post_key);
    let mut note = ticket.note.trim().to_string();
    if note.is_empty() {
        note = "Goods sales order".into();
    }
    let mut draft = build_draft_json(st, &draft_id, &customer, &reference, &filled, &note);
    let preview = match st.query("draft_preview", json!({ "draft": draft.clone() })) {
        Ok(v) => v,
        Err(e) => {
            st.toast_err(e);
            return false;
        }
    };
    draft["supplied_total"] = preview["supplied_total"].clone();
    if st.run("draft_save", json!({ "draft": draft.clone(), "expected_revision": null }), None).is_err() {
        return false; // `run` already toasted
    }
    let saved = st
        .books
        .list("drafts")
        .into_iter()
        .find(|d| d["id"] == draft["id"])
        .or_else(|| st.books.list("drafts").into_iter().find(|d| d["reference"] == reference));
    let Some(saved) = saved else {
        st.toast(Tone::Danger, st.fmt.t("Draft could not be saved."));
        return false;
    };
    let revision = saved["revision"].as_u64().or_else(|| saved["revision"].as_str().and_then(|s| s.parse().ok())).unwrap_or(1);
    let posted = match st.run(
        "draft_post",
        json!({
            "id": saved["id"],
            "revision": revision.to_string(),
            "request_key": post_key
        }),
        None,
    ) {
        Ok(v) => v,
        Err(_) => return false, // `run` already toasted
    };
    let record_id = crate::data::s(&posted["record_id"]);
    if record_id.is_empty() {
        st.toast(Tone::Danger, st.fmt.t("Draft could not be saved."));
        return false;
    }
    let fulfill_key = format!("{post_key}-fulfill");
    if let Err(e) = crate::ui::ops::post_operation(
        st,
        &record_id,
        "fulfill",
        &invoice,
        "Deliver goods and issue invoice",
        &fulfill_key,
    ) {
        if st.toasts.last().map(|t| t.message.as_str()) != Some(e.as_str()) {
            st.toast_err(e);
        }
        return false;
    }
    if ticket.pay_method < 2 {
        let invoice_id = format!("{fulfill_key}-invoice");
        if let Err(e) = settle_invoice(st, &customer, &invoice, &invoice_id, &format!("{post_key}-pay"), ticket.pay_method, &ticket.received) {
            if st.toasts.last().map(|t| t.message.as_str()) != Some(e.as_str()) {
                st.toast_err(e);
            }
            return false;
        }
    }
    true
}

fn finish_posting(st: &mut State) {
    st.sales.posting = false;
}

fn complete_sale(st: &mut State) {
    if st.sales.posting || !st.sales.modal_open {
        return;
    }
    st.sales.posting = true;
    let ticket = Ticket {
        draft_id: st.sales.draft_id.clone(),
        post_key: st.sales.post_key.clone(),
        customer_id: st.sales.customer_id.clone(),
        lines: st.sales.lines.clone(),
        pay_method: st.sales.pay_method,
        note: {
            let note = st.forms.get("sale-note").trim().to_string();
            if note.is_empty() { "Goods sales order".into() } else { note }
        },
        received: st.forms.get("sale-received"),
    };
    if post_ticket(st, &ticket) {
        st.sales.modal_open = false;
        st.sales.tab_idx = 0;
        st.toast(Tone::Success, st.fmt.t("Posted"));
    }
    finish_posting(st);
}

fn settle_invoice(
    st: &mut State,
    customer: &str,
    reference: &str,
    invoice_id: &str,
    request_key: &str,
    pay_method: usize,
    received_display: &str,
) -> Result<(), String> {
    let open = st.query("payment_options", json!({ "side": "receive", "party_id": customer }))?;
    let owed: i64 = open["items"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|i| crate::data::s(&i["id"]) == invoice_id)
        .map(|i| crate::data::s(&i["remaining"]).parse().unwrap_or(0))
        .unwrap_or(0);
    if owed <= 0 {
        return Err(st.fmt.t("No open customer invoices"));
    }
    let received = {
        let parsed = st.parse_money_input(received_display);
        if parsed > 0 { parsed } else { owed }
    };
    let paid = received.min(owed);
    if paid <= 0 {
        return Err(st.fmt.t("Amount"));
    }
    let funds = if pay_method == 0 { "cash" } else { "bank" };
    let mut pay = json!({
        "side": "receive",
        "party_id": customer,
        "funds_role": funds,
        "reference": reference,
        "reason": "Invoice settlement",
        "source_id": request_key,
        "request_key": request_key,
        "advance_amount": "0",
        "allocations": [{ "open_item_id": invoice_id, "amount": paid.to_string() }]
    });
    let pr = st.query("payment_preview", pay.clone())?;
    pay["expected_generation"] = pr["_generation"].clone();
    st.run("payment_post", pay, None).map(|_| ())
}

/// Save the facture as a quotation. Does not deliver stock or take payment.
fn issue_quotation(st: &mut State) {
    if st.sales.posting || !st.sales.modal_open {
        return;
    }
    st.sales.posting = true;
    let filled: Vec<_> = st.sales.lines.iter().filter(|l| !l.item_id.is_empty()).cloned().collect();
    if filled.is_empty() {
        st.toast(Tone::Danger, st.fmt.t("Add at least one product."));
        finish_posting(st);
        return;
    }
    let customer = if st.sales.customer_id.is_empty() {
        match st.ensure_walk_in_customer() {
            Ok(id) => id,
            Err(e) => {
                st.toast_err(e);
                finish_posting(st);
                return;
            }
        }
    } else {
        st.sales.customer_id.clone()
    };
    let reference = next_order_ref(st, &st.sales.post_key);
    let note = {
        let note = st.forms.get("sale-note").trim().to_string();
        if note.is_empty() { "Quotation".into() } else { note }
    };
    let draft_id = if st.sales.draft_id.is_empty() { st.uid() } else { st.sales.draft_id.clone() };
    let mut draft = build_draft_json(st, &draft_id, &customer, &reference, &filled, &note);
    let preview = match st.query("draft_preview", json!({ "draft": draft.clone() })) {
        Ok(v) => v,
        Err(e) => {
            st.toast_err(e);
            finish_posting(st);
            return;
        }
    };
    draft["supplied_total"] = preview["supplied_total"].clone();
    match st.run(
        "draft_save",
        json!({ "draft": draft, "expected_revision": null, "issue_quotation": true }),
        Some("Saved"),
    ) {
        Ok(_) => {
            st.sales.modal_open = false;
            st.sales.tab_idx = 1;
            finish_posting(st);
        }
        Err(_) => {
            finish_posting(st);
        }
    }
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let t_inv = ui.t("Invoices");
    let t_draft = ui.t("Drafts");
    let tabs = [(t_inv.as_str(), s.sales.tab_idx == 0), (t_draft.as_str(), s.sales.tab_idx == 1)];
    let tab_bar = ui.tabs("sales-tabs", &tabs, |st, idx| {
        st.sales.tab_idx = idx;
        st.sales.table = TableState::new();
    });

    let table = if s.sales.tab_idx == 0 {
        Table::new(
            |s: &State| &s.sales.table,
            |s: &mut State| &mut s.sales.table,
            |s: &State| sales_records(s).len(),
            |s: &State, i| Cow::Owned(format!("sale-{}", crate::data::s(&sales_records(s)[i]["id"]))),
        )
        .name(ui.t("Invoices"))
        .column(TableColumn::new("ref", ui.t("Reference")).width(140.0).min(96.0))
        .column(TableColumn::new("customer", ui.t("Customer")).flex(2.0).min(140.0).sortable())
        .column(TableColumn::new("date", ui.t("Date")).width(110.0).min(88.0))
        .column(TableColumn::new("amount", ui.t("Amount")).width(160.0).min(100.0).align(Align::End).sortable())
        .column(TableColumn::new("status", ui.t("Status")).width(110.0).min(80.0))
        .cell(|s: &State, i, col| {
            let record = &sales_records(s)[i];
            match col {
                "ref" => Cell::text(sales_ref(record)),
                "customer" => Cell::text(s.books.party_name(&sales_party(record))),
                "date" => Cell::text(s.fmt.date(&sales_date(record))).muted(),
                "amount" => Cell::text(s.fmt.money(&sales_amount(record))).mono(),
                _ => Cell::text(s.fmt.t("Posted")).muted(),
            }
        })
        .on_select(|st, id, _| st.sales.selected = id.strip_prefix("sale-").unwrap_or(&id).to_string())
        .zebra(true)
        .build(s, &ui.k)
        .key("sales-table")
        .grow(1.0)
        .min_h(0.0)
    } else {
        Table::new(
            |s: &State| &s.sales.table,
            |s: &mut State| &mut s.sales.table,
            |s: &State| drafts(s).len(),
            |s: &State, i| Cow::Owned(format!("draft-{}", crate::data::s(&drafts(s)[i]["id"]))),
        )
        .name(ui.t("Drafts"))
        .column(TableColumn::new("ref", ui.t("Reference")).width(140.0).min(96.0))
        .column(TableColumn::new("customer", ui.t("Customer")).flex(2.0).min(140.0))
        .column(TableColumn::new("date", ui.t("Date")).width(110.0).min(88.0))
        .column(TableColumn::new("amount", ui.t("Amount")).width(160.0).min(100.0).align(Align::End))
        .column(TableColumn::new("status", ui.t("Status")).width(110.0).min(80.0))
        .cell(|s: &State, i, col| {
            let draft = &drafts(s)[i];
            let note = crate::data::s(&draft["reason"]);
            match col {
                "ref" => Cell::text(crate::data::s(&draft["reference"])),
                "customer" => Cell::text(s.books.party_name(&crate::data::s(&draft["customer_id"]))),
                "date" => Cell::text(s.fmt.date(&crate::data::s(&draft["document_date"]))).muted(),
                "amount" => Cell::text(s.fmt.money(&crate::data::s(&draft["supplied_total"]))).mono(),
                _ => Cell::text(if note.is_empty() { s.fmt.t("Draft") } else { note }).muted(),
            }
        })
        .on_select(|st, id, _| st.sales.selected = id.strip_prefix("draft-").unwrap_or(&id).to_string())
        .zebra(true)
        .build(s, &ui.k)
        .key("sales-table")
        .grow(1.0)
        .min_h(0.0)
    };

    let add_btn = ui.btn(Btn::Primary, "New sales invoice", Some(&icons::PLUS), |st| {
        st.sales.start_new();
        st.forms.set("sale-note", "");
        st.forms.set("sale-received", "");
    });

    if s.sales.modal_open {
        mount_sale_dialog(s, ui, layer);
    }

    ui.page(
        "sales-screen",
        &ui.t("Sales"),
        &ui.t("Sales that are delivered and billed."),
        vec![
            add_btn,
            ui.btn(Btn::Glass, "Print / save PDF", Some(&icons::DOWNLOAD), print_selected),
            ui.btn(Btn::Glass, "Return", None, return_selected),
        ],
        col().w_full().gap(14.0).grow(1.0).min_h(0.0).child(tab_bar).child(ui.fill_or_empty(
            if s.sales.tab_idx == 0 { sales_records(s).len() } else { drafts(s).len() },
            if s.sales.tab_idx == 0 { "No invoices" } else { "No drafts" },
            "Sales that are delivered and billed.",
            &icons::SALES,
            table,
        )),
    )
}

fn estimate_total(s: &State) -> i64 {
    s.sales
        .lines
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.item_id.is_empty())
        .map(|(i, l)| {
            let qty_raw = {
                let f = s.forms.get(&format!("sale-qty-{i}"));
                if f.trim().is_empty() { l.qty.clone() } else { f }
            };
            let price_raw = {
                let f = s.forms.get(&format!("sale-price-{i}"));
                if f.trim().is_empty() { l.price.clone() } else { f }
            };
            let qty: i64 = crate::i18n::latin_digits(&qty_raw).replace(',', "").parse().unwrap_or(1).max(1);
            let price = s.parse_money_input(&price_raw);
            price * qty
        })
        .sum()
}

fn mount_sale_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let mut portals = Vec::new();
    let mut customer_opts = pickers::party_options(s, true, false);
    let q = s.sales.customer_combo.search().text.clone();
    customer_opts = pickers::with_create_option(customer_opts, &q, &ui.t("New customer"));

    let customer_picker = pickers::entity_combo(
        ui,
        &mut portals,
        "sale-customer",
        &s.sales.customer_combo,
        |st| &mut st.sales.customer_combo,
        &customer_opts,
        if s.sales.customer_id.is_empty() { None } else { Some(s.sales.customer_id.as_str()) },
        &ui.t("Select customer"),
        pickers::PICKER_WIDTH,
        |st, id| {
            if let Some(name) = pickers::create_text(id) {
                match st.create_party(ResourceKind::Customer, name, "", "", "") {
                    Ok(cid) => st.sales.customer_id = cid,
                    Err(e) => st.toast_err(e),
                }
            } else {
                st.sales.customer_id = id.to_string();
            }
        },
    );
    let add_customer = pickers::add_new_btn(ui, "New customer", |st| {
        let name = st.sales.customer_combo.search().text.trim().to_string();
        let name = if name.is_empty() { st.fmt.t("Walk-in customer") } else { name };
        match st.create_party(ResourceKind::Customer, &name, "", "", "") {
            Ok(cid) => st.sales.customer_id = cid,
            Err(e) => st.toast_err(e),
        }
    });

    let pay_cash = ui.t("Cash");
    let pay_bank = ui.t("Bank");
    let pay_later = ui.t("Later");
    let pay_tabs = [
        (pay_cash.as_str(), s.sales.pay_method == 0),
        (pay_bank.as_str(), s.sales.pay_method == 1),
        (pay_later.as_str(), s.sales.pay_method == 2),
    ];
    let pay_bar = ui.tabs("sale-pay-method", &pay_tabs, |st, idx| st.sales.pay_method = idx);

    let header = row()
        .w_full()
        .px(4.0)
        .gap(8.0)
        .child(ui.muted(ui.t("Select product")).grow(1.0))
        .child(ui.muted(ui.t("Qty")).w(90.0))
        .child(ui.muted(ui.t("Unit price")).w(140.0));

    let mut lines_col = col().key("sale-lines").w_full().gap(8.0).child(header);
    for (i, line) in s.sales.lines.iter().enumerate() {
        let mut item_opts = pickers::item_options(s);
        let line_q = s.sales.line_combos.get(i).map(|c| c.search().text.clone()).unwrap_or_default();
        item_opts = pickers::with_create_option(item_opts, &line_q, &ui.t("New product"));
        let line_id = line.id.clone();
        let item_picker = pickers::entity_combo(
            ui,
            &mut portals,
            &format!("sale-item-{i}"),
            s.sales.line_combos.get(i).unwrap_or(&ComboboxState::default()),
            move |st| {
                st.sales.ensure_line_combos();
                &mut st.sales.line_combos[i]
            },
            &item_opts,
            if line.item_id.is_empty() { None } else { Some(line.item_id.as_str()) },
            &ui.t("Select product"),
            pickers::PICKER_WIDTH,
            move |st, id| {
                st.sales.ensure_line_combos();
                if let Some(name) = pickers::create_text(id) {
                    st.sales.creating_item_for_line = Some(i);
                    st.sales.new_item_name = name.to_string();
                    st.forms.set("quick-item-name", name);
                    st.forms.set("quick-item-price", "");
                    st.forms.set("quick-item-cost", "");
                    return;
                }
                let price = pickers::item_sale_price(st, id);
                let toman = st.fmt.unit == crate::i18n::Unit::Toman;
                if let Some(l) = st.sales.lines.iter_mut().find(|l| l.id == line_id) {
                    l.item_id = id.to_string();
                    if l.price.is_empty() {
                        let rials: i64 = price.parse().unwrap_or(0);
                        l.price = if toman { (rials / 10).to_string() } else { rials.to_string() };
                        st.forms.set(&format!("sale-price-{}", i), &l.price);
                    }
                }
                if st.sales.lines.last().map(|l| !l.item_id.is_empty()).unwrap_or(false) {
                    st.sales.lines.push(SaleLine::blank());
                    st.sales.line_combos.push(ComboboxState::default());
                }
            },
        );
        let qty_ph = if line.qty.is_empty() { "1" } else { line.qty.as_str() };
        let price_ph = if line.price.is_empty() { "۰" } else { line.price.as_str() };
        let row_el = row()
            .key(format!("sale-line-{i}"))
            .w_full()
            .gap(8.0)
            .center_y()
            .child(item_picker.grow(1.0).min_w(0.0))
            .child(ui.field(s, &format!("sale-qty-{i}"), qty_ph).w(90.0))
            .child(ui.field(s, &format!("sale-price-{i}"), price_ph).w(140.0));
        lines_col = lines_col.child(row_el);
    }

    let total = estimate_total(s);
    let total_row = row()
        .w_full()
        .px(4.0)
        .center_y()
        .justify_between()
        .child(ui.muted(ui.t("Amount")))
        .child(text(ui.f.money(&total.to_string())).font(ui.k.ty.body_medium).color(ui.k.c.text_base));

    let form = col()
        .gap(16.0)
        .child(ui.labeled("Customer", pickers::picker_row(customer_picker, add_customer)))
        .child(ui.section("sale-lines-sec", &ui.t("Products"), lines_col))
        .child(total_row)
        .child(ui.form_grid(
            ui.labeled("Description", ui.field(s, "sale-note", &ui.t("Note (optional)"))),
            ui.section("sale-pay", &ui.t("Payment method"), {
                let mut pay = col().gap(10.0).child(pay_bar);
                if s.sales.pay_method < 2 {
                    pay = pay.child(ui.labeled("Amount received", ui.field(s, "sale-received", "۰")));
                }
                pay
            }),
        ));

    let save_btn = ui.btn(Btn::Primary, "Complete sale", Some(&icons::CHECK), |st| {
        for (i, line) in st.sales.lines.iter_mut().enumerate() {
            let q = st.forms.get(&format!("sale-qty-{i}"));
            let p = st.forms.get(&format!("sale-price-{i}"));
            if !q.trim().is_empty() {
                line.qty = q;
            }
            if !p.trim().is_empty() {
                line.price = p;
            }
        }
        complete_sale(st);
    });

    ui.dialog_described(
        layer,
        &mut portals,
        "sale",
        &ui.t("New sales invoice"),
        &ui.t("Pick a customer and products, or type to add new ones."),
        true,
        760.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.sales.modal_open = false),
            ui.btn(Btn::Glass, "Issue quotation", None, |st| {
                for (i, line) in st.sales.lines.iter_mut().enumerate() {
                    let q = st.forms.get(&format!("sale-qty-{i}"));
                    let p = st.forms.get(&format!("sale-price-{i}"));
                    if !q.trim().is_empty() {
                        line.qty = q;
                    }
                    if !p.trim().is_empty() {
                        line.price = p;
                    }
                }
                issue_quotation(st);
            }),
            save_btn,
        ],
        |st| st.sales.modal_open = false,
    );

    if s.sales.creating_item_for_line.is_some() {
        let quick = col()
            .gap(12.0)
            .child(ui.labeled("Name", ui.field(s, "quick-item-name", &ui.t("Enter a product name."))))
            .child(ui.form_grid(
                ui.labeled("Your cost", ui.field(s, "quick-item-cost", "۰")),
                ui.labeled("Unit price", ui.field(s, "quick-item-price", "۰")),
            ));
        let add = ui.btn(Btn::Primary, "Add product", Some(&icons::CHECK), |st| {
            let Some(idx) = st.sales.creating_item_for_line else { return };
            let name = st.forms.get("quick-item-name").trim().to_string();
            if name.is_empty() {
                st.toast(Tone::Danger, st.fmt.t("Enter a product name."));
                return;
            }
            let sale = st.parse_money_input(&st.forms.get("quick-item-price")).to_string();
            let cost_raw = st.forms.get("quick-item-cost");
            let cost = if cost_raw.trim().is_empty() {
                None
            } else {
                Some(st.parse_money_input(&cost_raw).to_string())
            };
            match st.create_item(&name, "", "", &sale, cost.as_deref(), "", "", "", "", "", "0", false) {
                Ok(id) => {
                    if let Some(l) = st.sales.lines.get_mut(idx) {
                        l.item_id = id;
                        l.price = st.forms.get("quick-item-price");
                    }
                    st.sales.creating_item_for_line = None;
                    if st.sales.lines.last().map(|l| !l.item_id.is_empty()).unwrap_or(false) {
                        st.sales.lines.push(SaleLine::blank());
                        st.sales.line_combos.push(ComboboxState::default());
                    }
                }
                Err(e) => st.toast_err(e),
            }
        });
        ui.dialog_described(
            layer,
            &mut Vec::new(),
            "quick-product",
            &ui.t("New product"),
            &ui.t("Add it to your inventory and this sale at the same time."),
            true,
            440.0,
            quick,
            vec![
                ui.btn(Btn::Ghost, "Cancel", None, |st| st.sales.creating_item_for_line = None),
                add,
            ],
            |st| st.sales.creating_item_for_line = None,
        );
    }
}

fn variant<'a>(record: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    let data = &record["data"][key];
    if !data.is_null() { data } else { &record["command"][key] }
}

fn sales_ref(record: &serde_json::Value) -> String {
    for (kind, field) in [("Order", "reference"), ("Fulfill", "invoice_reference"), ("Return", "return_reference")] {
        let value = crate::data::s(&record["command"][kind][field]);
        if !value.is_empty() {
            return value;
        }
    }
    let id = crate::data::s(&record["id"]);
    format!("#{}", &id[..id.len().min(8)])
}

fn sales_party(record: &serde_json::Value) -> String {
    for kind in ["Order", "Fulfill", "Return"] {
        let row = variant(record, kind);
        let id = crate::data::s(&row["customer"]["id"]);
        if !id.is_empty() {
            return id;
        }
        let id = crate::data::s(&row["customer_id"]);
        if !id.is_empty() {
            return id;
        }
    }
    String::new()
}

fn sales_date(record: &serde_json::Value) -> String {
    for kind in ["Order", "Fulfill", "Return"] {
        let date = crate::data::s(&record["command"][kind]["context"]["date"]);
        if !date.is_empty() {
            return date;
        }
        let date = crate::data::s(&record["command"][kind]["document_date"]);
        if !date.is_empty() {
            return date;
        }
    }
    String::new()
}

fn sales_amount(record: &serde_json::Value) -> String {
    for kind in ["Order", "Fulfill", "Return"] {
        let amount = crate::data::s(&variant(record, kind)["total"]);
        if !amount.is_empty() {
            return amount;
        }
    }
    "0".into()
}

fn is_invoice(record: &serde_json::Value) -> bool {
    record["command"].get("Fulfill").is_some() || record["data"].get("Fulfill").is_some()
}

fn print_selected(st: &mut State) {
    if st.sales.selected.is_empty() {
        st.toast(Tone::Info, st.fmt.t("Select contact"));
        return;
    }
    let result = if st.sales.tab_idx == 0 {
        crate::ui::documents::write_sales_pdf(st, &st.sales.selected)
    } else {
        crate::ui::documents::write_quotation_pdf(st, &st.sales.selected)
    };
    match result {
        Ok(path) => st.toast(Tone::Success, path.display().to_string()),
        Err(e) => st.toast_err(e),
    }
}

fn return_selected(st: &mut State) {
    let Some(record) = st.books.list("sales").into_iter().find(|r| crate::data::s(&r["id"]) == st.sales.selected) else {
        st.toast(Tone::Info, st.fmt.t("No invoices"));
        return;
    };
    if !is_invoice(&record) {
        st.toast(Tone::Info, st.fmt.t("No invoices"));
        return;
    }
    let id = st.sales.selected.clone();
    let reference = format!("RET-{}", &id[..id.len().min(6)]);
    let key = format!("return-{id}");
    match crate::ui::ops::post_operation(st, &id, "sale_return", &reference, "Restock customer return and issue credit", &key) {
        Ok(_) => st.toast(Tone::Success, st.fmt.t("Goods operation posted")),
        Err(e) => {
            if st.toasts.last().map(|t| t.message.as_str()) != Some(e.as_str()) {
                st.toast_err(e);
            }
        }
    }
}
