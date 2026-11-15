//! Sales — list + Quick Sale facture (creatable customer/product Combobox, lines, pay).

use std::borrow::Cow;

use rune::{Align, Cx, El, col, row, text};
use rune_kit::{Cell, ComboboxState, SortDir, Table, TableColumn, TableState};
use serde_json::json;

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, DIALOG_MD, DIALOG_SM, Ui};
use crate::ui::icons;
use crate::ui::pickers;
use crate::ui::resources::ResourceKind;
use crate::ui::rows::{Row, SortKey, reapply_sort, sort_rows};

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
    pub order_ref: String,
    pub invoice_ref: String,
    pub posted_record_id: Option<String>,
    pub fulfilled: bool,
    pub invoice_id: Option<String>,
    pub editing_revision: Option<String>,
    pub posting: bool,
    pub selected: String,
    pub rows: Vec<Row>,
    pub draft_rows: Vec<Row>,
    pub rows_stamp: u64,
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
        self.order_ref.clear();
        self.invoice_ref.clear();
        self.posted_record_id = None;
        self.fulfilled = false;
        self.invoice_id = None;
        self.editing_revision = None;
        self.posting = false;
    }

    pub fn start_edit(&mut self, draft: &serde_json::Value) {
        self.modal_open = true;
        self.customer_id = crate::data::s(&draft["customer_id"]);
        self.customer_combo = ComboboxState::default();
        let mut lines: Vec<SaleLine> = draft["lines"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|l| SaleLine {
                id: crate::data::s(&l["id"]),
                item_id: crate::data::s(&l["item_id"]),
                qty: crate::data::s(&l["quantity"]),
                price: crate::data::s(&l["unit_price"]),
                discount: crate::data::s(&l["line_discount"]),
            })
            .collect();
        if lines.is_empty() || lines.last().is_some_and(|l| !l.item_id.is_empty()) {
            lines.push(SaleLine::blank());
        }
        self.lines = lines;
        self.line_combos = vec![ComboboxState::default(); self.lines.len()];
        self.pay_method = 2;
        self.creating_item_for_line = None;
        self.new_item_name.clear();
        self.draft_id = crate::data::s(&draft["id"]);
        self.order_ref.clear();
        self.invoice_ref.clear();
        self.posted_record_id = None;
        self.fulfilled = false;
        self.invoice_id = None;
        self.editing_revision = Some(crate::data::s(&draft["revision"]));
        self.posting = false;
    }

    fn ensure_line_combos(&mut self) {
        while self.line_combos.len() < self.lines.len() {
            self.line_combos.push(ComboboxState::default());
        }
        self.line_combos.truncate(self.lines.len());
    }
}

pub fn open_sale(st: &mut State) {
    st.sales.start_new();
    st.forms.clear_prefix("sale-");
    st.sales.post_key = st.mint_key("sale");
    st.sales.draft_id = st.mint_key("draft");
    st.sales.order_ref = next_order_ref(st, &st.sales.post_key);
    st.sales.invoice_ref = next_invoice_ref(st, &st.sales.post_key);
}

fn is_dirty(st: &State) -> bool {
    st.forms.any_with_prefix("sale-") || !st.sales.customer_id.is_empty() || st.sales.lines.iter().any(|l| !l.item_id.is_empty())
}

pub fn close_or_confirm(st: &mut State) {
    if !is_dirty(st) {
        st.sales.modal_open = false;
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.sales.modal_open = false);
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
    date: &str,
) -> serde_json::Value {
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

pub(crate) fn next_order_ref(s: &State, post_key: &str) -> String {
    format!("ORD-{}-{}", s.books.count("sales_count") + 1, ref_tail(post_key))
}

pub(crate) fn next_invoice_ref(s: &State, post_key: &str) -> String {
    let prefix = s.books.workspace["business_profile"]["invoice_prefix"].as_str().unwrap_or("INV");
    format!("{prefix}-{}-{}", s.books.count("sales_count") + 1, ref_tail(post_key))
}

/// Step-level progress and stable references for a multi-step counter sale.
#[derive(Default, Clone)]
pub struct SaleProgress {
    pub order_ref: String,
    pub invoice_ref: String,
    pub posted_record_id: Option<String>,
    pub fulfilled: bool,
    pub invoice_id: Option<String>,
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
    pub date: String,
}

pub fn post_ticket(st: &mut State, ticket: &Ticket, progress: &mut SaleProgress) -> bool {
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
        match st.parse_money_input(&l.price) {
            Ok(v) if v >= 0 => {}
            _ => {
                st.toast(Tone::Danger, st.fmt.t("Enter a valid amount."));
                return false;
            }
        }
        if !l.discount.trim().is_empty() && st.parse_money_input(&l.discount).is_err() {
            st.toast(Tone::Danger, st.fmt.t("Enter a valid amount."));
            return false;
        }
    }
    if ticket.pay_method < 2 && !ticket.received.trim().is_empty() && st.parse_money_input(&ticket.received).is_err() {
        st.toast(Tone::Danger, st.fmt.t("Enter a valid amount."));
        return false;
    }
    let customer = if ticket.customer_id.is_empty() {
        match st.ensure_walk_in_customer() {
            Ok(id) => id,
            Err(_) => return false,
        }
    } else {
        ticket.customer_id.clone()
    };
    if ticket.post_key.is_empty() || ticket.draft_id.is_empty() {
        st.toast(Tone::Danger, st.fmt.t("Reopen this form and try again."));
        return false;
    }
    let post_key = ticket.post_key.clone();
    let draft_id = ticket.draft_id.clone();
    if progress.order_ref.is_empty() {
        progress.order_ref = next_order_ref(st, &post_key);
    }
    if progress.invoice_ref.is_empty() {
        progress.invoice_ref = next_invoice_ref(st, &post_key);
    }
    let reference = progress.order_ref.clone();
    let invoice = progress.invoice_ref.clone();
    let mut note = ticket.note.trim().to_string();
    if note.is_empty() {
        note = st.fmt.t("Goods sales order");
    }
    let record_id = if let Some(ref existing) = progress.posted_record_id {
        existing.clone()
    } else {
        let mut draft = build_draft_json(st, &draft_id, &customer, &reference, &filled, &note, &ticket.date);
        let preview = match st.query("draft_preview", json!({ "draft": draft.clone() })) {
            Ok(v) => v,
            Err(e) => {
                st.toast_err(e);
                return false;
            }
        };
        draft["supplied_total"] = preview["supplied_total"].clone();
        if st
            .run("draft_save", json!({ "draft": draft.clone(), "expected_revision": null }), None)
            .is_err()
        {
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
        let revision = saved["revision"]
            .as_u64()
            .or_else(|| saved["revision"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(1);
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
        progress.posted_record_id = Some(record_id.clone());
        record_id
    };
    let fulfill_key = format!("{post_key}-fulfill");
    let invoice_id = if progress.fulfilled {
        progress.invoice_id.clone().unwrap_or_else(|| {
            // Plan 026 fallback when resuming without a recorded invoice_id.
            format!("{fulfill_key}-invoice")
        })
    } else {
        let fulfill_note = st.fmt.t("Deliver goods and issue invoice");
        let fulfill_res =
            match crate::ui::ops::post_operation(st, &record_id, "fulfill", &invoice, &fulfill_note, &fulfill_key, &ticket.date) {
                Ok(v) => v,
                Err(_) => return false,
            };
        let returned = crate::data::s(&fulfill_res["invoice_id"]);
        let invoice_id = if !returned.is_empty() {
            returned
        } else {
            // Plan 026 fallback: older engine responses without `invoice_id`.
            format!("{fulfill_key}-invoice")
        };
        progress.fulfilled = true;
        progress.invoice_id = Some(invoice_id.clone());
        invoice_id
    };
    if ticket.pay_method < 2 && settle_invoice(st, ticket, &customer, &invoice, &invoice_id, &format!("{post_key}-pay")).is_err() {
        return false;
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
    let Ok(date) = st.form_date("sale-date") else {
        finish_posting(st);
        return;
    };
    let ticket = Ticket {
        draft_id: st.sales.draft_id.clone(),
        post_key: st.sales.post_key.clone(),
        customer_id: st.sales.customer_id.clone(),
        lines: st.sales.lines.clone(),
        pay_method: st.sales.pay_method,
        note: {
            let note = st.forms.get("sale-note").trim().to_string();
            if note.is_empty() { st.fmt.t("Goods sales order") } else { note }
        },
        received: st.forms.get("sale-received"),
        date,
    };
    let mut progress = SaleProgress {
        order_ref: st.sales.order_ref.clone(),
        invoice_ref: st.sales.invoice_ref.clone(),
        posted_record_id: st.sales.posted_record_id.clone(),
        fulfilled: st.sales.fulfilled,
        invoice_id: st.sales.invoice_id.clone(),
    };
    if post_ticket(st, &ticket, &mut progress) {
        st.sales.modal_open = false;
        st.sales.post_key.clear();
        st.sales.draft_id.clear();
        st.sales.order_ref.clear();
        st.sales.invoice_ref.clear();
        st.sales.posted_record_id = None;
        st.sales.fulfilled = false;
        st.sales.invoice_id = None;
        st.sales.editing_revision = None;
        st.sales.tab_idx = 0;
        st.toast(Tone::Success, st.fmt.t("Posted"));
    } else {
        st.sales.order_ref = progress.order_ref;
        st.sales.invoice_ref = progress.invoice_ref;
        st.sales.posted_record_id = progress.posted_record_id;
        st.sales.fulfilled = progress.fulfilled;
        st.sales.invoice_id = progress.invoice_id;
    }
    finish_posting(st);
}

fn settle_invoice(
    st: &mut State,
    ticket: &Ticket,
    customer: &str,
    reference: &str,
    invoice_id: &str,
    request_key: &str,
) -> Result<(), String> {
    let open = match st.query("payment_options", json!({ "side": "receive", "party_id": customer })) {
        Ok(v) => v,
        Err(e) => {
            st.toast_err(e.clone());
            return Err(e);
        }
    };
    let owed: i64 = open["items"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|i| crate::data::s(&i["id"]) == invoice_id)
        .map(|i| crate::data::s(&i["remaining"]).parse().unwrap_or(0))
        .unwrap_or(0);
    if owed <= 0 {
        let msg = st.fmt.t("No open customer invoices");
        st.toast_err(msg.clone());
        return Err(msg);
    }
    let received = match st.parse_money_input(&ticket.received) {
        Ok(v) if v > 0 => v,
        Ok(_) | Err(crate::i18n::MoneyError::Empty) => owed,
        Err(_) => {
            let msg = st.fmt.t("Enter a valid amount.");
            st.toast_err(msg.clone());
            return Err(msg);
        }
    };
    let paid = received.min(owed);
    if paid <= 0 {
        let msg = st.fmt.t("Amount");
        st.toast_err(msg.clone());
        return Err(msg);
    }
    let funds = if ticket.pay_method == 0 { "cash" } else { "bank" };
    let mut pay = json!({
        "side": "receive",
        "date": &ticket.date,
        "party_id": customer,
        "funds_role": funds,
        "reference": reference,
        "reason": "Invoice settlement",
        "source_id": request_key,
        "request_key": request_key,
        "advance_amount": "0",
        "allocations": [{ "open_item_id": invoice_id, "amount": paid.to_string() }]
    });
    let pr = match st.query("payment_preview", pay.clone()) {
        Ok(v) => v,
        Err(e) => {
            st.toast_err(e.clone());
            return Err(e);
        }
    };
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
            Err(_) => {
                finish_posting(st);
                return;
            }
        }
    } else {
        st.sales.customer_id.clone()
    };
    if st.sales.draft_id.is_empty() {
        st.toast(Tone::Danger, st.fmt.t("Reopen this form and try again."));
        finish_posting(st);
        return;
    }
    let Ok(date) = st.form_date("sale-date") else {
        finish_posting(st);
        return;
    };
    let draft_id = st.sales.draft_id.clone();
    let existing = st
        .sales
        .editing_revision
        .as_ref()
        .and_then(|_| st.books.list("drafts").into_iter().find(|d| crate::data::s(&d["id"]) == draft_id));
    let reference = existing
        .as_ref()
        .map(|d| crate::data::s(&d["reference"]))
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| next_order_ref(st, &st.sales.post_key));
    let note = {
        let note = st.forms.get("sale-note").trim().to_string();
        if note.is_empty() { "Quotation".into() } else { note }
    };
    let mut draft = build_draft_json(st, &draft_id, &customer, &reference, &filled, &note, &date);
    if let Some(rev) = &st.sales.editing_revision {
        draft["revision"] = json!(rev);
    }
    let preview = match st.query("draft_preview", json!({ "draft": draft.clone() })) {
        Ok(v) => v,
        Err(e) => {
            st.toast_err(e);
            finish_posting(st);
            return;
        }
    };
    draft["supplied_total"] = preview["supplied_total"].clone();
    let expected_revision = st.sales.editing_revision.clone();
    let already_issued = existing.is_some_and(|d| !d["quote_issue"].is_null());
    let save_payload = if already_issued {
        json!({ "draft": draft, "expected_revision": expected_revision })
    } else {
        json!({ "draft": draft, "expected_revision": expected_revision, "issue_quotation": true })
    };
    match st.run("draft_save", save_payload, Some("Saved")) {
        Ok(_) => {
            st.sales.modal_open = false;
            st.sales.post_key.clear();
            st.sales.draft_id.clear();
            st.sales.editing_revision = None;
            st.sales.tab_idx = 1;
            finish_posting(st);
        }
        Err(_) => {
            finish_posting(st);
        }
    }
}

fn col_index(col: &str) -> Option<usize> {
    match col {
        "ref" => Some(0),
        "customer" => Some(1),
        "date" => Some(2),
        "amount" => Some(3),
        "status" => Some(4),
        _ => None,
    }
}

fn build_invoice_rows(s: &State) -> Vec<Row> {
    let status = s.fmt.t("Posted");
    sales_records(s)
        .iter()
        .map(|record| {
            let ref_text = sales_ref(record);
            let customer = s.books.party_name(&sales_party(record));
            let date_raw = sales_date(record);
            let amount_raw = sales_amount(record);
            Row {
                id: format!("sale-{}", crate::data::s(&record["id"])),
                cells: vec![
                    ref_text.clone(),
                    customer.clone(),
                    s.fmt.date(&date_raw),
                    s.fmt.money(&amount_raw),
                    status.clone(),
                ],
                keys: vec![
                    SortKey::text(&ref_text),
                    SortKey::text(&customer),
                    SortKey::text(&date_raw),
                    SortKey::num_str(&amount_raw),
                    SortKey::text(&status),
                ],
            }
        })
        .collect()
}

fn build_draft_rows(s: &State) -> Vec<Row> {
    let fallback_status = s.fmt.t("Draft");
    drafts(s)
        .iter()
        .map(|draft| {
            let ref_text = crate::data::s(&draft["reference"]);
            let customer = s.books.party_name(&crate::data::s(&draft["customer_id"]));
            let date_raw = crate::data::s(&draft["document_date"]);
            let amount_raw = crate::data::s(&draft["supplied_total"]);
            let note = crate::data::s(&draft["reason"]);
            let status = if note.is_empty() { fallback_status.clone() } else { note };
            Row {
                id: format!("draft-{}", crate::data::s(&draft["id"])),
                cells: vec![
                    ref_text.clone(),
                    customer.clone(),
                    s.fmt.date(&date_raw),
                    s.fmt.money(&amount_raw),
                    status.clone(),
                ],
                keys: vec![
                    SortKey::text(&ref_text),
                    SortKey::text(&customer),
                    SortKey::text(&date_raw),
                    SortKey::num_str(&amount_raw),
                    SortKey::text(&status),
                ],
            }
        })
        .collect()
}

pub fn ensure(st: &mut State) {
    if st.sales.rows_stamp == st.stamp {
        return;
    }
    st.sales.rows = build_invoice_rows(st);
    st.sales.draft_rows = build_draft_rows(st);
    st.sales.rows_stamp = st.stamp;
    if st.sales.tab_idx == 0 {
        reapply_sort(&st.sales.table, &mut st.sales.rows, col_index);
    } else {
        reapply_sort(&st.sales.table, &mut st.sales.draft_rows, col_index);
    }
}

pub fn apply_sort(st: &mut State, col: &str, dir: SortDir) {
    st.sales.table.set_sort(col, dir);
    if let Some(idx) = col_index(col) {
        if st.sales.tab_idx == 0 {
            sort_rows(&mut st.sales.rows, idx, dir);
        } else {
            sort_rows(&mut st.sales.draft_rows, idx, dir);
        }
    }
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let t_inv = ui.t("Invoices");
    let t_draft = ui.t("Drafts");
    let tabs = [(t_inv.as_str(), s.sales.tab_idx == 0), (t_draft.as_str(), s.sales.tab_idx == 1)];
    let tab_bar = ui.tabs("sales-tabs", &tabs, |st, idx| {
        st.sales.tab_idx = idx;
        st.sales.selected.clear();
        st.sales.table = TableState::new();
        st.sales.rows_stamp = 0;
        ensure(st);
    });

    let table = if s.sales.tab_idx == 0 {
        Table::new(
            |s: &State| &s.sales.table,
            |s: &mut State| &mut s.sales.table,
            |s: &State| s.sales.rows.len(),
            |s: &State, i| Cow::Owned(s.sales.rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
        )
        .name(ui.t("Invoices"))
        .column(TableColumn::new("ref", ui.t("Reference")).width(140.0).min(96.0).sortable())
        .column(TableColumn::new("customer", ui.t("Customer")).flex(2.0).min(140.0).sortable())
        .column(TableColumn::new("date", ui.t("Date")).width(110.0).min(88.0).sortable())
        .column(
            TableColumn::new("amount", ui.t("Amount"))
                .width(160.0)
                .min(100.0)
                .align(Align::End)
                .sortable(),
        )
        .column(TableColumn::new("status", ui.t("Status")).width(110.0).min(80.0).sortable())
        .cell(|s: &State, i, col| {
            let Some(row) = s.sales.rows.get(i) else {
                return Cell::text("");
            };
            match col {
                "ref" => Cell::text(&row.cells[0]),
                "customer" => Cell::text(&row.cells[1]),
                "date" => Cell::text(&row.cells[2]).muted(),
                "amount" => Cell::text(&row.cells[3]).mono(),
                _ => Cell::text(&row.cells[4]).muted(),
            }
        })
        .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
        .on_select(|st, id, _| st.sales.selected = id.strip_prefix("sale-").unwrap_or(id).to_string())
        .on_open(|st, id, _| crate::ui::detail::open(st, id.strip_prefix("sale-").unwrap_or(id)))
        .zebra(true)
        .build(s, &ui.k)
        .key("sales-table")
        .grow(1.0)
        .min_h(0.0)
    } else {
        Table::new(
            |s: &State| &s.sales.table,
            |s: &mut State| &mut s.sales.table,
            |s: &State| s.sales.draft_rows.len(),
            |s: &State, i| Cow::Owned(s.sales.draft_rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
        )
        .name(ui.t("Drafts"))
        .column(TableColumn::new("ref", ui.t("Reference")).width(140.0).min(96.0).sortable())
        .column(TableColumn::new("customer", ui.t("Customer")).flex(2.0).min(140.0).sortable())
        .column(TableColumn::new("date", ui.t("Date")).width(110.0).min(88.0).sortable())
        .column(
            TableColumn::new("amount", ui.t("Amount"))
                .width(160.0)
                .min(100.0)
                .align(Align::End)
                .sortable(),
        )
        .column(TableColumn::new("status", ui.t("Status")).width(110.0).min(80.0).sortable())
        .cell(|s: &State, i, col| {
            let Some(row) = s.sales.draft_rows.get(i) else {
                return Cell::text("");
            };
            match col {
                "ref" => Cell::text(&row.cells[0]),
                "customer" => Cell::text(&row.cells[1]),
                "date" => Cell::text(&row.cells[2]).muted(),
                "amount" => Cell::text(&row.cells[3]).mono(),
                _ => Cell::text(&row.cells[4]).muted(),
            }
        })
        .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
        .on_select(|st, id, _| st.sales.selected = id.strip_prefix("draft-").unwrap_or(id).to_string())
        .on_open(|st, id, _| crate::ui::detail::open(st, id.strip_prefix("draft-").unwrap_or(id)))
        .zebra(true)
        .build(s, &ui.k)
        .key("sales-table")
        .grow(1.0)
        .min_h(0.0)
    };

    let add_btn = ui.btn(Btn::Primary, "New sales invoice", Some(&icons::PLUS), open_sale);

    if s.sales.modal_open {
        mount_sale_dialog(s, ui, layer);
    }

    let mut body = col().w_full().gap(14.0).grow(1.0).min_h(0.0).child(tab_bar).child(ui.fill_or_empty(
        if s.sales.tab_idx == 0 {
            s.sales.rows.len()
        } else {
            s.sales.draft_rows.len()
        },
        if s.sales.tab_idx == 0 { "No invoices" } else { "No drafts" },
        "Sales that are delivered and billed.",
        &icons::SALES,
        table,
    ));
    let footer = if s.sales.tab_idx == 0 {
        let shown = s.books.list("sales").len();
        let total = s.books.count("sales_count");
        ui.load_more_footer("sales-load-more", shown, total, load_more_sales)
    } else {
        let shown = s.books.list("drafts").len();
        let count = s.books.count("drafts_count");
        let total = if count > 0 {
            count
        } else if shown >= 100 {
            shown + 1
        } else {
            shown
        };
        ui.load_more_footer("drafts-load-more", shown, total, load_more_drafts)
    };
    if let Some(f) = footer {
        body = body.child(f);
    }

    let mut actions = vec![
        add_btn,
        ui.btn_enabled(Btn::Glass, "Open", None, !s.sales.selected.is_empty(), |st| {
            let id = st.sales.selected.clone();
            crate::ui::detail::open(st, &id);
        }),
        ui.btn_enabled(
            Btn::Glass,
            "Print / save PDF",
            Some(&icons::DOWNLOAD),
            !s.sales.selected.is_empty(),
            print_selected,
        ),
    ];
    if s.sales.tab_idx == 0 {
        actions.push(ui.btn_enabled(
            Btn::Glass,
            "Deliver & invoice",
            None,
            !s.sales.selected.is_empty(),
            fulfill_selected,
        ));
        actions.push(ui.btn_enabled(Btn::Glass, "Return", None, !s.sales.selected.is_empty(), return_selected));
    } else {
        actions.push(ui.btn_enabled(Btn::Glass, "Edit", None, !s.sales.selected.is_empty(), edit_selected_draft));
        actions.push(ui.btn_enabled(
            Btn::Glass,
            "Convert to sale",
            None,
            !s.sales.selected.is_empty(),
            convert_selected_draft,
        ));
        actions.push(ui.btn_enabled(Btn::Danger, "Delete", None, !s.sales.selected.is_empty(), delete_selected_draft));
    }

    ui.page(
        "sales-screen",
        &ui.t("Sales"),
        &ui.t("Sales that are delivered and billed."),
        actions,
        body,
    )
}

fn load_more_sales(st: &mut State) {
    if let Err(e) = st.books.load_more(&st.engine, "sales") {
        st.toast_err(e);
        return;
    }
    st.stamp += 1;
    ensure(st);
}

fn load_more_drafts(st: &mut State) {
    if let Err(e) = st.books.load_more(&st.engine, "drafts") {
        st.toast_err(e);
        return;
    }
    st.stamp += 1;
    ensure(st);
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
            let price = s.parse_money_input(&price_raw).unwrap_or(0);
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
        if s.sales.customer_id.is_empty() {
            None
        } else {
            Some(s.sales.customer_id.as_str())
        },
        &ui.t("Select customer"),
        pickers::PICKER_WIDTH,
        |st, id| {
            if let Some(name) = pickers::create_text(id) {
                if let Ok(cid) = st.create_party(ResourceKind::Customer, name, "", "", "") {
                    st.sales.customer_id = cid;
                }
            } else {
                st.sales.customer_id = id.to_string();
            }
        },
    );
    let add_customer = pickers::add_new_btn(ui, "New customer", |st| {
        let name = st.sales.customer_combo.search().text.trim().to_string();
        let name = if name.is_empty() { st.fmt.t("Walk-in customer") } else { name };
        if let Ok(cid) = st.create_party(ResourceKind::Customer, &name, "", "", "") {
            st.sales.customer_id = cid;
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
            if line.item_id.is_empty() {
                None
            } else {
                Some(line.item_id.as_str())
            },
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
                if let Some(l) = st.sales.lines.iter_mut().find(|l| l.id == line_id) {
                    l.item_id = id.to_string();
                    if l.price.is_empty() {
                        let rials: i64 = price.parse().unwrap_or(0);
                        l.price = st.fmt.money_input(rials);
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
        let row_el = row()
            .key(format!("sale-line-{i}"))
            .w_full()
            .gap(8.0)
            .center_y()
            .child(item_picker.grow(1.0).min_w(0.0))
            .child(ui.field(s, &format!("sale-qty-{i}"), qty_ph).w(90.0))
            .child(ui.money_field(s, &format!("sale-price-{i}")).w(160.0));
        lines_col = lines_col.child(row_el);
    }

    let total = estimate_total(s);
    let total_row = row()
        .w_full()
        .px(4.0)
        .center_y()
        .justify_between()
        .child(ui.muted(ui.t("Amount")))
        .child(
            text(ui.f.money(&total.to_string()))
                .font(ui.k.ty.body_medium)
                .color(ui.k.c.text_base),
        );

    let mut total_col = col().w_full().gap(4.0).child(total_row);
    if s.lang() == crate::i18n::Lang::Fa {
        let words = sarv_quote::iran::words::amount_in_words(i128::from(total), s.fmt.unit == crate::i18n::Unit::Toman);
        total_col = total_col.child(row().key("sale-total-words").w_full().px(4.0).child(ui.muted(words)));
    }

    let form = col()
        .gap(16.0)
        .child(ui.labeled("Customer", pickers::picker_row(customer_picker, add_customer)))
        .child(ui.section("sale-lines-sec", &ui.t("Products"), lines_col))
        .child(total_col)
        .child(
            ui.form_grid(
                col()
                    .gap(12.0)
                    .child(ui.date_field(s, "sale-date", "Date"))
                    .child(ui.labeled("Description", ui.field(s, "sale-note", &ui.t("Note (optional)")))),
                ui.section("sale-pay", &ui.t("Payment method"), {
                    let mut pay = col().gap(10.0).child(pay_bar);
                    if s.sales.pay_method < 2 {
                        pay = pay.child(ui.labeled("Amount received", ui.money_field(s, "sale-received")));
                    }
                    pay
                }),
            ),
        );

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

    ui.form_dialog(
        layer,
        &mut portals,
        "sale",
        &ui.t("New sales invoice"),
        &ui.t("Pick a customer and products, or type to add new ones."),
        true,
        DIALOG_MD,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close_or_confirm),
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
        close_or_confirm,
    );

    if s.sales.creating_item_for_line.is_some() {
        let quick = col()
            .gap(12.0)
            .child(ui.labeled("Name", ui.field(s, "quick-item-name", &ui.t("Enter a product name."))))
            .child(ui.form_grid(
                ui.labeled("Your cost", ui.money_field(s, "quick-item-cost")),
                ui.labeled("Unit price", ui.money_field(s, "quick-item-price")),
            ));
        let add = ui.btn(Btn::Primary, "Add product", Some(&icons::CHECK), |st| {
            let Some(idx) = st.sales.creating_item_for_line else { return };
            let name = st.forms.get("quick-item-name").trim().to_string();
            if name.is_empty() {
                st.toast(Tone::Danger, st.fmt.t("Enter a product name."));
                return;
            }
            let sale = match st.parse_money_input(&st.forms.get("quick-item-price")) {
                Ok(v) => v.to_string(),
                Err(crate::i18n::MoneyError::Empty) => "0".into(),
                Err(_) => {
                    st.toast(Tone::Danger, st.fmt.t("Enter a valid amount."));
                    return;
                }
            };
            let cost_raw = st.forms.get("quick-item-cost");
            let cost = if cost_raw.trim().is_empty() {
                None
            } else {
                match st.parse_money_input(&cost_raw) {
                    Ok(v) => Some(v.to_string()),
                    Err(_) => {
                        st.toast(Tone::Danger, st.fmt.t("Enter a valid amount."));
                        return;
                    }
                }
            };
            if let Ok(id) = st.create_item(&name, "", "", &sale, cost.as_deref(), "", "", "", "", "", "0", false) {
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
        });
        ui.dialog_described(
            layer,
            &mut Vec::new(),
            "quick-product",
            &ui.t("New product"),
            &ui.t("Add it to your inventory and this sale at the same time."),
            true,
            DIALOG_SM,
            quick,
            vec![ui.btn(Btn::Ghost, "Cancel", None, |st| st.sales.creating_item_for_line = None), add],
            |st| st.sales.creating_item_for_line = None,
        );
    }
}

fn variant<'a>(record: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    let data = &record["data"][key];
    if !data.is_null() { data } else { &record["command"][key] }
}

fn sales_ref(record: &serde_json::Value) -> String {
    for (kind, field) in [
        ("Order", "reference"),
        ("Fulfill", "invoice_reference"),
        ("Return", "return_reference"),
    ] {
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
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    }
    let result = if st.sales.tab_idx == 0 {
        crate::ui::documents::write_sales_pdf(st, &st.sales.selected)
    } else {
        crate::ui::documents::write_quotation_pdf(st, &st.sales.selected)
    };
    match result {
        Ok(path) => st.toast_file(path),
        Err(e) => st.toast_err(e),
    }
}

fn fulfill_selected(st: &mut State) {
    if st.sales.selected.is_empty() {
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    }
    let id = st.sales.selected.clone();
    let key = st.mint_key("fulfill");
    let reference = next_invoice_ref(st, &key);
    crate::ui::operation_dialog::open(
        st,
        &id,
        "fulfill",
        "Deliver & invoice",
        reference,
        "Deliver goods and issue invoice",
    );
}

fn return_selected(st: &mut State) {
    let Some(record) = st
        .books
        .list("sales")
        .into_iter()
        .find(|r| crate::data::s(&r["id"]) == st.sales.selected)
    else {
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    };
    if !is_invoice(&record) {
        st.toast(Tone::Info, st.fmt.t("No invoices"));
        return;
    }
    let id = st.sales.selected.clone();
    let ref_ = sales_ref(&record);
    crate::ui::operation_dialog::open(
        st,
        &id,
        "sale_return",
        "Return goods",
        format!("RET-{ref_}"),
        "Restock customer return and issue credit",
    );
}

fn edit_selected_draft(st: &mut State) {
    let Some(draft) = st
        .books
        .list("drafts")
        .into_iter()
        .find(|d| crate::data::s(&d["id"]) == st.sales.selected)
    else {
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    };
    st.forms.clear_prefix("sale-");
    st.sales.start_edit(&draft);
    st.sales.post_key = st.mint_key("sale");
    st.sales.order_ref = next_order_ref(st, &st.sales.post_key);
    st.sales.invoice_ref = next_invoice_ref(st, &st.sales.post_key);
    for (i, line) in st.sales.lines.iter().enumerate() {
        if !line.item_id.is_empty() {
            st.forms.set(&format!("sale-qty-{i}"), &line.qty);
            st.forms.set(&format!("sale-price-{i}"), &line.price);
        }
    }
    st.forms.set("sale-note", &crate::data::s(&draft["reason"]));
    let doc_date = crate::data::s(&draft["document_date"]);
    if !doc_date.is_empty() {
        st.forms.set("sale-date", &st.fmt.date_input(&doc_date));
    }
}

fn convert_selected_draft(st: &mut State) {
    let Some(draft) = st
        .books
        .list("drafts")
        .into_iter()
        .find(|d| crate::data::s(&d["id"]) == st.sales.selected)
    else {
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    };
    let id = crate::data::s(&draft["id"]);
    let rev = crate::data::s(&draft["revision"]);
    let key = st.mint_key("convert");
    let Ok(posted) = st.run("draft_post", json!({ "id": id, "revision": rev, "request_key": key }), None) else {
        return;
    };
    let record_id = crate::data::s(&posted["record_id"]);
    if record_id.is_empty() {
        st.toast(Tone::Danger, st.fmt.t("Draft could not be saved."));
        return;
    }
    let reference = next_invoice_ref(st, &key);
    st.sales.tab_idx = 0;
    st.sales.selected = record_id.clone();
    crate::ui::operation_dialog::open(
        st,
        &record_id,
        "fulfill",
        "Deliver & invoice",
        reference,
        "Deliver goods and issue invoice",
    );
}

fn delete_selected_draft(st: &mut State) {
    let Some(draft) = st
        .books
        .list("drafts")
        .into_iter()
        .find(|d| crate::data::s(&d["id"]) == st.sales.selected)
    else {
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    };
    let id = crate::data::s(&draft["id"]);
    let rev = crate::data::s(&draft["revision"]);
    st.confirm = Some(crate::ui::confirm::Confirm::one(
        st.fmt.t("Delete this quotation?"),
        String::new(),
        Vec::new(),
        "Confirm delete",
        st.fmt.t("Delete"),
        true,
        move |st| {
            if st
                .run("draft_delete", json!({ "id": id, "revision": rev }), Some("Deleted"))
                .is_ok()
            {
                st.sales.selected.clear();
            }
        },
    ));
}
