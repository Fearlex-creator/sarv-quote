//! Record detail sheet: shows header facts, document lines/allocations, and linked journal lines.

use rune::{El, col, row, text};
use serde_json::{Value, json};

use crate::app::{State, Tone};
use crate::data::s;
use crate::ui::glass::{Btn, DIALOG_LG, Ui};
use crate::ui::icons;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailKind {
    #[default]
    Journal,
    Sale,
    Purchase,
    Payment,
    Draft,
}

#[derive(Default)]
pub struct DetailState {
    pub open: bool,
    pub record: Value,
    pub kind: DetailKind,
    pub journal: Option<Value>,
}

/// Open the detail dialog for any record id (journal, sale, purchase, payment, or draft).
pub fn open(st: &mut State, id: &str) {
    let id = id.trim();
    if id.is_empty() {
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    }
    match st.query("detail", json!({ "id": id })) {
        Ok(record) => {
            let kind = classify(&record);
            let journal = if kind == DetailKind::Journal {
                None
            } else {
                let jid = s(&record["journal_id"]);
                if jid.is_empty() {
                    None
                } else {
                    st.books
                        .list("journals")
                        .into_iter()
                        .find(|j| s(&j["id"]) == jid)
                        .or_else(|| st.query("detail", json!({ "id": jid })).ok())
                }
            };
            st.detail = DetailState {
                open: true,
                record,
                kind,
                journal,
            };
        }
        Err(err) => {
            if let Some(draft) = st.books.list("drafts").into_iter().find(|d| s(&d["id"]) == id) {
                st.detail = DetailState {
                    open: true,
                    record: draft,
                    kind: DetailKind::Draft,
                    journal: None,
                };
            } else {
                st.toast_err(err);
            }
        }
    }
}

fn classify(record: &Value) -> DetailKind {
    if record.get("lines").is_some() && record.get("source_id").is_some() {
        return DetailKind::Journal;
    }
    if record["data"].get("Payment").is_some() || record["data"].get("Reversal").is_some() {
        return DetailKind::Payment;
    }
    let cmd = &record["command"];
    let data = &record["data"];
    if cmd.get("Fulfill").is_some()
        || data.get("Fulfill").is_some()
        || data["Order"].get("customer").is_some()
        || cmd["Order"].get("customer_id").is_some()
        || data["Return"].get("customer_id").is_some()
    {
        return DetailKind::Sale;
    }
    DetailKind::Purchase
}

fn first_nonempty(values: impl IntoIterator<Item = String>) -> String {
    values.into_iter().find(|v| !v.is_empty()).unwrap_or_default()
}

fn active_variant(record: &Value) -> (&'static str, &Value, &Value) {
    for key in ["Fulfill", "Order", "Receive", "Bill", "Return", "Payment", "Reversal"] {
        let d = &record["data"][key];
        let c = &record["command"][key];
        if !d.is_null() || !c.is_null() {
            return (key, d, c);
        }
    }
    ("", &Value::Null, &Value::Null)
}

fn record_facts(st: &State) -> (String, String, String, String, String) {
    let r = &st.detail.record;
    let id = s(&r["id"]);
    match st.detail.kind {
        DetailKind::Journal => {
            let ref_no = first_nonempty([s(&r["source_id"]), format!("#{}", &id[..id.len().min(8)])]);
            let date = s(&r["date"]);
            let desc = first_nonempty([s(&r["reason"]), s(&r["event"])]);
            let total_debit: i128 = r["lines"]
                .as_array()
                .map(|lines| lines.iter().filter_map(|l| s(&l["debit"]).parse::<i128>().ok()).sum())
                .unwrap_or(0);
            (ref_no, date, desc, total_debit.to_string(), "Journal".into())
        }
        DetailKind::Draft => {
            let ref_no = first_nonempty([s(&r["reference"]), format!("#{}", &id[..id.len().min(8)])]);
            let date = s(&r["document_date"]);
            let party = st.books.party_name(&s(&r["customer_id"]));
            let total = s(&r["supplied_total"]);
            (ref_no, date, party, total, "Draft".into())
        }
        DetailKind::Payment => {
            let (_, d, c) = active_variant(r);
            let ref_no = first_nonempty([s(&d["reference"]), s(&c["reference"]), format!("#{}", &id[..id.len().min(8)])]);
            let date = first_nonempty([s(&c["context"]["effective_date"]), s(&c["context"]["date"]), s(&r["date"])]);
            let party_id = first_nonempty([s(&d["party"]["id"]), s(&c["party_id"])]);
            let party = first_nonempty([st.books.party_name(&party_id), s(&d["party"]["name"])]);
            let total = first_nonempty([s(&d["total"]), s(&d["applied"]), s(&r["total"])]);
            (ref_no, date, party, total, "Payment".into())
        }
        DetailKind::Sale | DetailKind::Purchase => {
            let (kind_key, d, c) = active_variant(r);
            let ref_no = first_nonempty([
                s(&c["invoice_reference"]),
                s(&c["reference"]),
                s(&c["return_reference"]),
                format!("#{}", &id[..id.len().min(8)]),
            ]);
            let date = first_nonempty([
                s(&c["context"]["effective_date"]),
                s(&c["context"]["date"]),
                s(&c["document_date"]),
                s(&r["date"]),
            ]);
            let party_id = first_nonempty([
                s(&d["customer"]["id"]),
                s(&d["supplier"]["id"]),
                s(&c["customer_id"]),
                s(&c["supplier_id"]),
                s(&d["customer_id"]),
                s(&d["supplier_id"]),
            ]);
            let party = first_nonempty([st.books.party_name(&party_id), s(&d["customer"]["name"]), s(&d["supplier"]["name"])]);
            let total = first_nonempty([s(&d["total"]), s(&c["supplied_total"]), "0".into()]);
            let label = if st.detail.kind == DetailKind::Sale {
                if kind_key == "Fulfill" { "Invoice" } else { "Sale" }
            } else {
                "Purchase"
            };
            (ref_no, date, party, total, label.into())
        }
    }
}

fn item_name(s_state: &State, item_id: &str) -> String {
    if item_id.is_empty() {
        return String::new();
    }
    if let Some(items) = s_state.books.workspace["resources"]["items"].as_array() {
        if let Some(found) = items.iter().find(|it| s(&it["id"]) == item_id) {
            let name = s(&found["name"]);
            if !name.is_empty() {
                return name;
            }
        }
    }
    item_id.to_string()
}

fn line_rows(s_state: &State, ui: &Ui) -> Option<El> {
    let r = &s_state.detail.record;
    match s_state.detail.kind {
        DetailKind::Journal => None,
        DetailKind::Payment => {
            let (_, d, _) = active_variant(r);
            let allocs = d["allocations"].as_array()?;
            if allocs.is_empty() {
                return None;
            }
            let mut rows = vec![line_header(ui, &[ui.t("Reference"), ui.t("Date"), ui.t("Amount")])];
            for (i, alloc) in allocs.iter().enumerate() {
                let open_id = s(&alloc["open_item_id"]);
                let date = ui.f.date(&s(&alloc["document_date"]));
                let amt = ui.f.money(&s(&alloc["amount"]));
                rows.push(line_row(ui, &format!("detail-alloc-{i}"), &[open_id, date, amt]));
            }
            Some(ui.section("detail-lines", &ui.t("Allocations"), col().w_full().gap(6.0).children(rows)))
        }
        DetailKind::Draft => {
            let lines = r["lines"].as_array()?;
            if lines.is_empty() {
                return None;
            }
            let mut rows = vec![line_header(ui, &[ui.t("Item"), ui.t("Quantity"), ui.t("Unit price")])];
            for (i, line) in lines.iter().enumerate() {
                let name = item_name(s_state, &s(&line["item_id"]));
                let qty = ui.f.qty(&s(&line["quantity_thousandths"]));
                let price = ui.f.money(&s(&line["unit_price"]));
                rows.push(line_row(ui, &format!("detail-line-{i}"), &[name, qty, price]));
            }
            Some(ui.section("detail-lines", &ui.t("Lines"), col().w_full().gap(6.0).children(rows)))
        }
        DetailKind::Sale | DetailKind::Purchase => {
            let (kind_key, d, _) = active_variant(r);
            let lines = d["lines"].as_array()?;
            if lines.is_empty() {
                return None;
            }
            let mut rows = vec![line_header(
                ui,
                &[ui.t("Item"), ui.t("Quantity"), ui.t("Unit price"), ui.t("Line total")],
            )];
            for (i, line) in lines.iter().enumerate() {
                let name = if kind_key == "Order" || kind_key == "Fulfill" {
                    first_nonempty([s(&line["item"]["name"]), item_name(s_state, &s(&line["item_id"]))])
                } else {
                    first_nonempty([item_name(s_state, &s(&line["item_id"])), s(&line["receipt_line_id"])])
                };
                let qty_raw = first_nonempty([
                    s(&line["quantity_thousandths"]),
                    s(&line["base_quantity_thousandths"]),
                    s(&line["input"]["quantity_thousandths"]),
                ]);
                let unit_raw = s(&line["input"]["unit_price"]);
                let total_raw = first_nonempty([s(&line["net"]), s(&line["value"]), s(&line["gross"]), "0".into()]);
                let qty = ui.f.qty(&qty_raw);
                let price = if unit_raw.is_empty() {
                    "—".to_string()
                } else {
                    ui.f.money(&unit_raw)
                };
                let total = ui.f.money(&total_raw);
                rows.push(line_row(ui, &format!("detail-line-{i}"), &[name, qty, price, total]));
            }
            Some(ui.section("detail-lines", &ui.t("Lines"), col().w_full().gap(6.0).children(rows)))
        }
    }
}

fn journal_section(s_state: &State, ui: &Ui) -> Option<El> {
    let journal = if s_state.detail.kind == DetailKind::Journal {
        &s_state.detail.record
    } else {
        s_state.detail.journal.as_ref()?
    };
    let lines = journal["lines"].as_array()?;
    if lines.is_empty() {
        return None;
    }
    let mut rows = vec![line_header(ui, &[ui.t("Account"), ui.t("Debit"), ui.t("Credit")])];
    for (i, line) in lines.iter().enumerate() {
        let code = s(&line["account"]["code"]);
        let name = s(&line["account"]["name"]);
        let account = if code.is_empty() { name } else { format!("{code} · {name}") };
        let debit = ui.f.money(&s(&line["debit"]));
        let credit = ui.f.money(&s(&line["credit"]));
        rows.push(line_row(ui, &format!("detail-jline-{i}"), &[account, debit, credit]));
    }
    Some(ui.section("detail-journal", &ui.t("Journal lines"), col().w_full().gap(6.0).children(rows)))
}

fn line_header(ui: &Ui, cols: &[String]) -> El {
    let mut r = row().w_full().gap(8.0).py(4.0);
    for (idx, col_label) in cols.iter().enumerate() {
        let cell = text(col_label.clone()).font(ui.k.ty.caption()).color(ui.k.c.text_muted);
        r = r.child(if idx == 0 {
            cell.grow(2.0).min_w(0.0)
        } else {
            cell.grow(1.0).min_w(0.0)
        });
    }
    r
}

fn line_row(ui: &Ui, key: &str, cols: &[String]) -> El {
    let mut r = row().key(key.to_string()).w_full().gap(8.0).py(4.0);
    for (idx, val) in cols.iter().enumerate() {
        let cell = text(val.clone()).font(ui.k.ty.ui).color(ui.k.c.text_base);
        r = r.child(if idx == 0 {
            cell.grow(2.0).min_w(0.0)
        } else {
            cell.grow(1.0).min_w(0.0)
        });
    }
    r
}

fn fact_row(ui: &Ui, key: &str, label: &str, value: &str) -> El {
    row()
        .key(key.to_string())
        .w_full()
        .px(12.0)
        .py(8.0)
        .center_y()
        .justify_between()
        .child(ui.muted(label))
        .child(text(value.to_string()).font(ui.k.ty.ui_medium).color(ui.k.c.text_base))
}

pub fn mount(s_state: &State, ui: &Ui, layer: &mut Vec<El>) {
    if !s_state.detail.open {
        return;
    }
    let (ref_no, date, party, total, kind_label) = record_facts(s_state);
    let party_label = if s_state.detail.kind == DetailKind::Journal {
        ui.t("Description")
    } else {
        ui.t("Party")
    };
    let facts = ui.panel_rows(
        "detail-facts",
        vec![
            fact_row(ui, "detail-ref", &ui.t("Reference"), &ref_no),
            fact_row(ui, "detail-date", &ui.t("Date"), &ui.f.date(&date)),
            fact_row(ui, "detail-party", &party_label, if party.is_empty() { "—" } else { &party }),
            fact_row(ui, "detail-total", &ui.t("Total"), &ui.f.money(&total)),
        ],
    );

    let mut body = col().w_full().gap(14.0).child(facts);
    if let Some(lines) = line_rows(s_state, ui) {
        body = body.child(lines);
    }
    if let Some(journal) = journal_section(s_state, ui) {
        body = body.child(journal);
    }

    let mut footer = Vec::new();
    if s_state.detail.kind == DetailKind::Sale
        && (s_state.detail.record["command"].get("Fulfill").is_some() || s_state.detail.record["data"].get("Fulfill").is_some())
    {
        let doc_id = s(&s_state.detail.record["id"]);
        footer.push(ui.btn(
            Btn::Glass,
            "Print / save PDF",
            Some(&icons::DOWNLOAD),
            move |st| match crate::ui::documents::write_sales_pdf(st, &doc_id) {
                Ok(path) => st.toast(Tone::Success, path.display().to_string()),
                Err(e) => st.toast_err(e),
            },
        ));
    } else if s_state.detail.kind == DetailKind::Draft {
        let draft_id = s(&s_state.detail.record["id"]);
        footer.push(ui.btn(
            Btn::Glass,
            "Print / save PDF",
            Some(&icons::DOWNLOAD),
            move |st| match crate::ui::documents::write_quotation_pdf(st, &draft_id) {
                Ok(path) => st.toast(Tone::Success, path.display().to_string()),
                Err(e) => st.toast_err(e),
            },
        ));
    }
    if s_state.detail.kind != DetailKind::Journal
        && let Some(j) = &s_state.detail.journal
    {
        let jid = s(&j["id"]);
        if !jid.is_empty() {
            footer.push(ui.btn(Btn::Glass, "Open journal", None, move |st| open(st, &jid)));
        }
    }
    footer.push(ui.btn(Btn::Ghost, "Close", None, |st| st.detail.open = false));

    let title = format!("{} · {}", ui.t(&kind_label), ref_no);
    let mut portals = Vec::new();
    ui.dialog_described(
        layer,
        &mut portals,
        "detail",
        &title,
        "",
        s_state.detail.open,
        DIALOG_LG,
        body,
        footer,
        |st| st.detail.open = false,
    );
}
