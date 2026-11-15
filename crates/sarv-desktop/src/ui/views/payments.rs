//! Payments — receivables/payables KPIs and kit Table of settlements.

use std::borrow::Cow;

use rune::{Align, Cx, El, col, row};
use rune_kit::{Cell, SortDir, Table, TableColumn, TableState};

use serde_json::json;

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, DIALOG_MD, Ui};
use crate::ui::icons;
use crate::ui::rows::{Row, SortKey, reapply_sort, sort_rows};

#[derive(Default)]
pub struct PaymentsState {
    pub tab_idx: usize,
    pub table: TableState,
    pub cheques_table: TableState,
    pub modal_open: bool,
    pub receive: bool,
    pub party_id: String,
    pub selected: String,
    pub selected_cheque: String,
    pub party_combo: rune_kit::ComboboxState,
    pub funds: usize, // 0 cash, 1 bank, 2 cheque
    pub post_key: String,
    pub posting: bool,
    pub rows: Vec<Row>,
    pub rows_stamp: u64,
    pub cheques_rows: Vec<Row>,
    pub cheques_rows_stamp: u64,
    pub spend_open: bool,
    pub spend_cheque_id: String,
    pub spend_cheque_amount: String,
    pub spend_cheque_number: String,
    pub spend_party_id: String,
    pub spend_party_combo: rune_kit::ComboboxState,
    pub spend_key: String,
    pub spend_posting: bool,
}

fn load_more_payments(st: &mut State) {
    if let Err(e) = st.books.load_more(&st.engine, "payments") {
        st.toast_err(e);
        return;
    }
    st.stamp += 1;
    ensure(st);
}

pub fn open_payment(st: &mut State, receive: bool) {
    st.forms.clear_prefix("pay-");
    st.payments.receive = receive;
    st.payments.party_id.clear();
    st.payments.party_combo = Default::default();
    st.payments.funds = 0;
    st.payments.post_key = st.mint_key("pay");
    st.payments.posting = false;
    st.payments.modal_open = true;
}

fn is_dirty(st: &State) -> bool {
    st.forms.any_with_prefix("pay-") || !st.payments.party_id.is_empty()
}

pub fn close_or_confirm(st: &mut State) {
    if !is_dirty(st) {
        st.payments.modal_open = false;
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.payments.modal_open = false);
}

fn first_nonempty(a: String, b: String) -> String {
    if a.is_empty() { b } else { a }
}

fn payments(s: &State) -> Vec<serde_json::Value> {
    s.books.list("payments")
}

fn col_index(col: &str) -> Option<usize> {
    match col {
        "ref" => Some(0),
        "name" => Some(1),
        "date" => Some(2),
        "amount" => Some(3),
        "status" => Some(4),
        _ => None,
    }
}

fn build_rows(s: &State) -> Vec<Row> {
    let list = payments(s);
    let reversed_ids: std::collections::HashSet<String> = list
        .iter()
        .filter_map(|record| {
            let pid = crate::data::s(&record["data"]["Reversal"]["payment_id"]);
            if pid.is_empty() { None } else { Some(pid) }
        })
        .collect();
    list.iter()
        .map(|record| {
            let id = crate::data::s(&record["id"]);
            let is_reversal = record["data"].get("Reversal").is_some();
            let is_reversed = is_reversal || reversed_ids.contains(&id);
            let status = if is_reversed { s.fmt.t("Reversed") } else { s.fmt.t("Posted") };
            let cmd = record["command"]
                .get("Payment")
                .or_else(|| record["command"].get("Reversal"))
                .unwrap_or(&record["command"]);
            let data = record["data"]
                .get("Payment")
                .or_else(|| record["data"].get("Reversal"))
                .unwrap_or(&record["data"]);
            let ref_no = first_nonempty(crate::data::s(&data["reference"]), crate::data::s(&cmd["reference"]));
            let ref_text = if ref_no.is_empty() {
                format!("#{}", &id[..id.len().min(8)])
            } else {
                ref_no
            };
            let party_id = first_nonempty(crate::data::s(&data["party"]["id"]), crate::data::s(&cmd["party_id"]));
            let name = first_nonempty(s.books.party_name(&party_id), crate::data::s(&data["party"]["name"]));
            let date_raw = first_nonempty(
                crate::data::s(&cmd["context"]["effective_date"]),
                first_nonempty(crate::data::s(&cmd["context"]["date"]), crate::data::s(&record["date"])),
            );
            let amount_raw = first_nonempty(
                crate::data::s(&data["total"]),
                first_nonempty(crate::data::s(&data["applied"]), crate::data::s(&record["total"])),
            );
            Row {
                id: format!("payment-{id}"),
                cells: vec![
                    ref_text.clone(),
                    name.clone(),
                    s.fmt.date(&date_raw),
                    s.fmt.money(&amount_raw),
                    status.clone(),
                ],
                keys: vec![
                    SortKey::text(&ref_text),
                    SortKey::text(&name),
                    SortKey::text(&date_raw),
                    SortKey::num_str(&amount_raw),
                    SortKey::text(&status),
                ],
            }
        })
        .collect()
}

fn cheque_col_index(col: &str) -> Option<usize> {
    match col {
        "number" => Some(0),
        "sayad" => Some(1),
        "party" => Some(2),
        "bank" => Some(3),
        "amount" => Some(4),
        "due_date" => Some(5),
        "direction" => Some(6),
        "status" => Some(7),
        _ => None,
    }
}

fn is_cheque_closed(status: &str) -> bool {
    matches!(status, "cleared" | "bounced" | "spent" | "returned" | "voided")
}

fn build_cheque_rows(s: &State) -> Vec<Row> {
    let mut entries = s.books.workspace["cheques"].as_array().cloned().unwrap_or_default();
    entries.sort_by(|a, b| {
        let sa = crate::data::s(&a["status"]);
        let sb = crate::data::s(&b["status"]);
        let da = crate::data::s(&a["cheque"]["details"]["due_date"]);
        let db = crate::data::s(&b["cheque"]["details"]["due_date"]);
        let na = crate::data::s(&a["cheque"]["details"]["number"]);
        let nb = crate::data::s(&b["cheque"]["details"]["number"]);
        (is_cheque_closed(&sa), da, na).cmp(&(is_cheque_closed(&sb), db, nb))
    });
    entries
        .into_iter()
        .map(|entry| {
            let id = crate::data::s(&entry["cheque"]["id"]);
            let details = &entry["cheque"]["details"];
            let number = first_nonempty(crate::data::s(&details["number"]), format!("#{}", &id[..id.len().min(8)]));
            let sayad_raw = crate::data::s(&details["sayad_id"]);
            let sayad_cell = if sayad_raw.is_empty() {
                String::new()
            } else {
                crate::i18n::isolate(&sayad_raw)
            };
            let party_id = crate::data::s(&entry["cheque"]["party_id"]);
            let party = s.books.party_name(&party_id);
            let bank = crate::data::s(&details["bank"]);
            let amount_raw = crate::data::s(&entry["cheque"]["amount"]);
            let due_raw = crate::data::s(&details["due_date"]);
            let dir_raw = crate::data::s(&entry["cheque"]["direction"]);
            let dir_label = if dir_raw.eq_ignore_ascii_case("Issued") {
                s.fmt.t("Issued")
            } else {
                s.fmt.t("Received")
            };
            let status_raw = crate::data::s(&entry["status"]);
            let status_label = s.fmt.t(cheque_status_label(&status_raw));
            Row {
                id: format!("cheque-{id}"),
                cells: vec![
                    number.clone(),
                    sayad_cell,
                    party.clone(),
                    bank.clone(),
                    s.fmt.money(&amount_raw),
                    s.fmt.date(&due_raw),
                    dir_label.clone(),
                    status_label.clone(),
                ],
                keys: vec![
                    SortKey::text(&number),
                    SortKey::text(&sayad_raw),
                    SortKey::text(&party),
                    SortKey::text(&bank),
                    SortKey::num_str(&amount_raw),
                    SortKey::text(&due_raw),
                    SortKey::text(&dir_label),
                    SortKey::text(&status_label),
                ],
            }
        })
        .collect()
}

pub fn ensure(st: &mut State) {
    if st.payments.rows_stamp != st.stamp {
        st.payments.rows = build_rows(st);
        st.payments.rows_stamp = st.stamp;
        reapply_sort(&st.payments.table, &mut st.payments.rows, col_index);
    }
    if st.payments.cheques_rows_stamp != st.stamp {
        st.payments.cheques_rows = build_cheque_rows(st);
        st.payments.cheques_rows_stamp = st.stamp;
        reapply_sort(&st.payments.cheques_table, &mut st.payments.cheques_rows, cheque_col_index);
    }
}

pub fn apply_sort(st: &mut State, col: &str, dir: SortDir) {
    st.payments.table.set_sort(col, dir);
    if let Some(idx) = col_index(col) {
        sort_rows(&mut st.payments.rows, idx, dir);
    }
}

pub fn apply_cheque_sort(st: &mut State, col: &str, dir: SortDir) {
    st.payments.cheques_table.set_sort(col, dir);
    if let Some(idx) = cheque_col_index(col) {
        sort_rows(&mut st.payments.cheques_rows, idx, dir);
    }
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    if !s.books.initialized() {
        return ui.page(
            "payments-screen",
            &ui.t("Payments"),
            "",
            vec![],
            ui.kit_card()
                .title(
                    ui.t("Confirm starting balances to settle invoices and bills."),
                    Some(&icons::PAYMENTS),
                )
                .action(ui.btn(Btn::Primary, "Record opening balances", Some(&icons::CHECK), |st| {
                    st.nav(crate::app::Page::Settings);
                }))
                .build(&ui.k),
        );
    }

    let receivables = s.books.position("receivables");
    let payables = s.books.position("payables");

    let metrics = row()
        .w_full()
        .gap(12.0)
        .child(ui.metric(
            "pay-ar",
            &icons::SALES,
            &ui.t("Receivables outstanding"),
            ui.f.money(&receivables),
            ui.t("Open customer invoices"),
            Tone::Success,
        ))
        .child(ui.metric(
            "pay-ap",
            &icons::PURCHASES,
            &ui.t("Payables outstanding"),
            ui.f.money(&payables),
            ui.t("Due supplier bills"),
            Tone::Warning,
        ));

    let table = Table::new(
        |s: &State| &s.payments.table,
        |s: &mut State| &mut s.payments.table,
        |s: &State| s.payments.rows.len(),
        |s: &State, i| Cow::Owned(s.payments.rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
    )
    .name(ui.t("Payments"))
    .column(TableColumn::new("ref", ui.t("Reference")).width(140.0).min(96.0).sortable())
    .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
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
        let Some(row) = s.payments.rows.get(i) else {
            return Cell::text("");
        };
        match col {
            "ref" => Cell::text(&row.cells[0]),
            "name" => Cell::text(&row.cells[1]),
            "date" => Cell::text(&row.cells[2]).muted(),
            "amount" => Cell::text(&row.cells[3]).mono(),
            _ => Cell::text(&row.cells[4]).muted(),
        }
    })
    .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
    .on_select(|st, id, _| st.payments.selected = id.strip_prefix("payment-").unwrap_or(id).to_string())
    .on_open(|st, id, _| crate::ui::detail::open(st, id.strip_prefix("payment-").unwrap_or(id)))
    .zebra(true)
    .build(s, &ui.k)
    .key("payments-table")
    .grow(1.0)
    .min_h(0.0);

    let payments_lbl = ui.t("Payments");
    let cheques_lbl = ui.t("Cheques");
    let tabs = ui.tabs(
        "payments-tabs",
        &[
            (payments_lbl.as_str(), s.payments.tab_idx == 0),
            (cheques_lbl.as_str(), s.payments.tab_idx == 1),
        ],
        |st, i| st.payments.tab_idx = i,
    );

    let mut body = col().w_full().gap(16.0).grow(1.0).min_h(0.0).child(metrics).child(tabs);
    if s.payments.tab_idx == 0 {
        body = body.child(ui.fill_or_empty(
            s.payments.rows.len(),
            "No payments recorded yet",
            "Settle invoices and bills",
            &icons::PAYMENTS,
            table,
        ));
        if let Some(footer) = ui.load_more_footer(
            "payments-load-more",
            s.books.list("payments").len(),
            s.books.count("payment_count"),
            load_more_payments,
        ) {
            body = body.child(footer);
        }
    } else {
        let cheques_table = Table::new(
            |s: &State| &s.payments.cheques_table,
            |s: &mut State| &mut s.payments.cheques_table,
            |s: &State| s.payments.cheques_rows.len(),
            |s: &State, i| Cow::Owned(s.payments.cheques_rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
        )
        .name(ui.t("Cheques"))
        .column(TableColumn::new("number", ui.t("Cheque number")).width(110.0).min(80.0).sortable())
        .column(TableColumn::new("sayad", ui.t("Sayad ID")).width(150.0).min(110.0).sortable())
        .column(TableColumn::new("party", ui.t("Name")).flex(1.5).min(120.0).sortable())
        .column(TableColumn::new("bank", ui.t("Bank")).width(110.0).min(80.0).sortable())
        .column(
            TableColumn::new("amount", ui.t("Amount"))
                .width(140.0)
                .min(100.0)
                .align(Align::End)
                .sortable(),
        )
        .column(TableColumn::new("due_date", ui.t("Due date")).width(110.0).min(88.0).sortable())
        .column(TableColumn::new("direction", ui.t("Direction")).width(100.0).min(80.0).sortable())
        .column(TableColumn::new("status", ui.t("Status")).width(110.0).min(80.0).sortable())
        .cell(|s: &State, i, col| {
            let Some(row) = s.payments.cheques_rows.get(i) else {
                return Cell::text("");
            };
            match col {
                "number" => Cell::text(&row.cells[0]),
                "sayad" => Cell::text(&row.cells[1]).mono(),
                "party" => Cell::text(&row.cells[2]),
                "bank" => Cell::text(&row.cells[3]).muted(),
                "amount" => Cell::text(&row.cells[4]).mono(),
                "due_date" => Cell::text(&row.cells[5]).muted(),
                "direction" => Cell::text(&row.cells[6]).muted(),
                _ => Cell::text(&row.cells[7]).muted(),
            }
        })
        .on_sort(|st, col, dir, _tx| apply_cheque_sort(st, col, dir))
        .on_select(|st, id, _| {
            st.payments.selected_cheque = id.strip_prefix("cheque-").unwrap_or(id).to_string();
        })
        .zebra(true)
        .build(s, &ui.k)
        .key("cheques-table")
        .grow(1.0)
        .min_h(0.0);

        body = body.child(ui.fill_or_empty(
            s.payments.cheques_rows.len(),
            "No cheques yet",
            "Settle invoices and bills",
            &icons::PAYMENTS,
            cheques_table,
        ));
        if let Some(detail) = cheque_detail_panel(s, ui) {
            body = body.child(detail);
        }
    }

    if s.payments.modal_open {
        mount_payment_dialog(s, ui, layer);
    }
    if s.payments.spend_open {
        mount_spend_dialog(s, ui, layer);
    }

    let pay_selected = s.payments.tab_idx == 0 && !s.payments.selected.is_empty();
    ui.page(
        "payments-screen",
        &ui.t("Payments"),
        &ui.t("Settle invoices and bills"),
        vec![
            ui.btn(Btn::Primary, "Receive money", Some(&icons::ARROW), |st| open_payment(st, true)),
            ui.btn(Btn::Glass, "Pay bill", Some(&icons::PURCHASES), |st| open_payment(st, false)),
            ui.btn_enabled(Btn::Glass, "Open", None, pay_selected, |st| {
                let id = st.payments.selected.clone();
                crate::ui::detail::open(st, &id);
            }),
            ui.btn_enabled(Btn::Glass, "Reverse payment", None, pay_selected, |st| {
                let id = st.payments.selected.clone();
                open_payment_reverse_confirm(st, &id);
            }),
        ],
        body,
    )
}

fn mount_payment_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    use crate::ui::pickers;
    use crate::ui::resources::ResourceKind;
    let mut portals = Vec::new();
    let customers = s.payments.receive;
    let mut opts = pickers::party_options(s, customers, !customers);
    let q = s.payments.party_combo.search().text.clone();
    opts = pickers::with_create_option(opts, &q, &if customers { ui.t("New customer") } else { ui.t("New supplier") });
    let party = pickers::entity_combo(
        ui,
        &mut portals,
        "pay-party",
        &s.payments.party_combo,
        |st| &mut st.payments.party_combo,
        &opts,
        if s.payments.party_id.is_empty() {
            None
        } else {
            Some(s.payments.party_id.as_str())
        },
        &ui.t("Select contact"),
        pickers::PICKER_WIDTH,
        move |st, id| {
            if let Some(name) = pickers::create_text(id) {
                let kind = if st.payments.receive {
                    ResourceKind::Customer
                } else {
                    ResourceKind::Vendor
                };
                if let Ok(pid) = st.create_party(kind, name, "", "", "") {
                    st.payments.party_id = pid;
                }
            } else {
                st.payments.party_id = id.to_string();
            }
        },
    );
    let cash = ui.t("Cash");
    let bank = ui.t("Bank");
    let cheque = ui.t("Cheque");
    let funds = [
        (cash.as_str(), s.payments.funds == 0),
        (bank.as_str(), s.payments.funds == 1),
        (cheque.as_str(), s.payments.funds == 2),
    ];
    let funds_bar = ui.tabs("pay-funds", &funds, |st, i| st.payments.funds = i);
    let amount_field = {
        let f = ui.labeled("Amount", ui.money_field(s, "pay-amount"));
        let raw = s.forms.get("pay-amount");
        if s.lang() == crate::i18n::Lang::Fa && !raw.trim().is_empty() {
            if let Ok(rials) = s.parse_money_input(&raw) {
                let words = sarv_quote::iran::words::amount_in_words(i128::from(rials), s.fmt.unit == crate::i18n::Unit::Toman);
                col().gap(4.0).child(f).child(ui.muted(words).key("pay-amount-words"))
            } else {
                f
            }
        } else {
            f
        }
    };
    let mut form = col()
        .gap(14.0)
        .child(ui.labeled(if customers { "Customer" } else { "Vendor" }, party))
        .child(ui.section("pay-funds-sec", &ui.t("Cash and bank"), funds_bar))
        .child(ui.form_grid(amount_field, ui.date_field(s, "pay-date", "Date")))
        .child(ui.form_grid(
            ui.labeled("Reference", ui.field(s, "pay-ref", "")),
            ui.labeled("Description", ui.field(s, "pay-reason", &ui.t("Note (optional)"))),
        ));
    if s.payments.funds == 2 {
        let sayad_raw = s.forms.get("pay-sayad");
        let sayad_norm = sarv_quote::iran::ids::normalize_digits(sayad_raw.trim());
        let sayad_field = if !sayad_norm.is_empty() && sarv_quote::iran::ids::sayad_id(&sayad_norm).is_err() {
            col()
                .gap(4.0)
                .child(ui.labeled("Sayad ID", ui.field(s, "pay-sayad", "")))
                .child(ui.muted(ui.t("A Sayad ID has exactly 16 digits.")))
        } else {
            ui.labeled("Sayad ID", ui.field(s, "pay-sayad", ""))
        };
        form = form
            .child(ui.form_grid(sayad_field, ui.labeled("Number", ui.field(s, "pay-cheque-no", ""))))
            .child(ui.form_grid(
                ui.labeled("Bank", ui.field(s, "pay-bank", "")),
                ui.labeled("Branch", ui.field(s, "pay-branch", "")),
            ))
            .child(ui.form_grid(
                ui.labeled("Drawer national ID", ui.field(s, "pay-drawer-nid", "")),
                ui.date_field(s, "pay-due", "Due date"),
            ));
    }

    let title = if s.payments.receive {
        ui.t("Receive money")
    } else {
        ui.t("Pay bill")
    };
    ui.form_dialog(
        layer,
        &mut portals,
        "payment",
        &title,
        &ui.t("Record a cash or bank settlement."),
        true,
        DIALOG_MD,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close_or_confirm),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                if st.payments.posting || !st.payments.modal_open {
                    return;
                }
                st.payments.posting = true;
                if st.payments.party_id.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Select contact"));
                    st.payments.posting = false;
                    return;
                }
                let amount = match st.parse_positive_money(&st.forms.get("pay-amount")) {
                    Ok(v) => v,
                    Err(key) => {
                        st.toast(Tone::Danger, st.fmt.t(key));
                        st.payments.posting = false;
                        return;
                    }
                };
                let Ok(date) = st.form_date("pay-date") else {
                    st.payments.posting = false;
                    return;
                };
                let side = if st.payments.receive { "receive" } else { "make" };
                let funds_role = match st.payments.funds {
                    0 => "cash",
                    1 => "bank",
                    _ => {
                        if st.payments.receive {
                            "cheques_receivable"
                        } else {
                            "cheques_payable"
                        }
                    }
                };
                let (allocations, advance) = build_allocations(st, side, &st.payments.party_id, amount);
                let reason = {
                    let r = st.forms.get("pay-reason");
                    if r.trim().is_empty() { st.fmt.t("Payment") } else { r }
                };
                if st.payments.post_key.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Reopen this form and try again."));
                    st.payments.posting = false;
                    return;
                }
                let key = st.payments.post_key.clone();
                let reference = {
                    let r = st.forms.get("pay-ref");
                    if r.trim().is_empty() { key.clone() } else { r }
                };
                let mut payload = json!({
                    "side": side,
                    "date": date,
                    "party_id": st.payments.party_id,
                    "funds_role": funds_role,
                    "reference": reference,
                    "reason": reason,
                    "source_id": key,
                    "request_key": key,
                    "advance_amount": advance,
                    "allocations": allocations
                });
                if st.payments.funds == 2 {
                    let sayad_raw = st.forms.get("pay-sayad");
                    let sayad = sarv_quote::iran::ids::normalize_digits(sayad_raw.trim());
                    let number = st.forms.get("pay-cheque-no");
                    let bank = st.forms.get("pay-bank");
                    let branch = st.forms.get("pay-branch");
                    let drawer_nid = sarv_quote::iran::ids::normalize_digits(st.forms.get("pay-drawer-nid").trim());
                    if sayad.is_empty() || number.trim().is_empty() || st.forms.get("pay-due").trim().is_empty() {
                        st.toast(Tone::Danger, st.fmt.t("Enter the cheque's Sayad ID, number and due date."));
                        st.payments.posting = false;
                        return;
                    }
                    if sarv_quote::iran::ids::sayad_id(&sayad).is_err() {
                        st.toast(Tone::Danger, st.fmt.t("A Sayad ID has exactly 16 digits."));
                        st.payments.posting = false;
                        return;
                    }
                    let Ok(due) = st.form_date("pay-due") else {
                        st.payments.posting = false;
                        return;
                    };
                    payload["cheque"] = json!({
                        "sayad_id": sayad,
                        "number": number.trim(),
                        "bank": bank.trim(),
                        "branch": branch.trim(),
                        "due_date": due,
                        "drawer_national_id": drawer_nid,
                        "purpose": "",
                        "notes": ""
                    });
                }
                let action = if st.payments.funds == 2 {
                    if st.payments.receive { "cheque_receive" } else { "cheque_issue" }
                } else {
                    "payment_post"
                };
                let preview_action = "payment_preview";
                match st.query(preview_action, payload.clone()) {
                    Ok(pr) => payload["expected_generation"] = pr["_generation"].clone(),
                    Err(e) => {
                        st.toast_err(e);
                        st.payments.posting = false;
                        return;
                    }
                }
                match st.run(action, payload, Some("Payment recorded")) {
                    Ok(_) => {
                        st.payments.modal_open = false;
                        st.payments.posting = false;
                        st.payments.post_key.clear();
                    }
                    Err(_) => {
                        // `run` already toasted the engine error.
                        st.payments.posting = false;
                    }
                }
            }),
        ],
        close_or_confirm,
    );
}

/// Engine status key → English message key (FA via `fa_messages`).
pub fn cheque_status_label(status: &str) -> &str {
    match status {
        "in_hand" => "In hand",
        "in_collection" => "In collection",
        "cleared" => "Cleared",
        "bounced" => "Bounced",
        "spent" => "Spent",
        "returned" => "Returned to customer",
        "issued" => "Issued",
        "voided" => "Voided",
        other => other,
    }
}

/// Engine allowed-action key → English button label.
pub fn cheque_action_label(act: &str) -> Option<&'static str> {
    match act {
        "deposit" => Some("Deposit for collection"),
        "clear" => Some("Mark as cleared"),
        "bounce" => Some("Mark as bounced"),
        "return" => Some("Return to customer"),
        "void" => Some("Void cheque"),
        "spend" => Some("Endorse to supplier"),
        _ => None,
    }
}

fn build_allocations(st: &State, side: &str, party_id: &str, amount: i64) -> (Vec<serde_json::Value>, String) {
    let opts = st.query(
        "payment_options",
        json!({
            "side": side,
            "party_id": party_id
        }),
    );
    let mut allocations = Vec::new();
    let mut left = amount;
    if let Ok(o) = opts {
        if let Some(items) = o["items"].as_array() {
            for item in items {
                if left <= 0 {
                    break;
                }
                let rem: i64 = crate::data::s(&item["remaining"]).parse().unwrap_or(0);
                let open_id = crate::data::s(&item["id"]);
                if rem <= 0 || open_id.is_empty() {
                    continue;
                }
                let applied = left.min(rem);
                allocations.push(json!({ "open_item_id": open_id, "amount": applied.to_string() }));
                left -= applied;
            }
        }
    }
    (allocations, left.to_string())
}

fn account_name(s: &State, account_id: &str) -> String {
    s.books.workspace["accounts"]
        .as_array()
        .and_then(|rows| rows.iter().find(|a| crate::data::s(&a["id"]) == account_id))
        .map(|a| crate::data::s(&a["name"]))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| account_id.to_string())
}

pub fn open_payment_reverse_confirm(st: &mut State, payment_id: &str) {
    if payment_id.is_empty() {
        return;
    }
    let preview = match st.query("payment_reverse_preview", json!({ "payment_id": payment_id })) {
        Ok(pr) => pr,
        Err(e) => {
            st.toast_err(e);
            return;
        }
    };
    let rev = &preview["data"]["Reversal"];
    let party_id = crate::data::s(&rev["party"]["id"]);
    let party = first_nonempty(st.books.party_name(&party_id), crate::data::s(&rev["party"]["name"]));
    let amount_raw = first_nonempty(crate::data::s(&preview["total"]), crate::data::s(&rev["total"]));
    let mut lines = vec![(st.fmt.t("Name"), party), (st.fmt.t("Amount"), st.fmt.money(&amount_raw))];
    for jl in preview["journal_lines"].as_array().into_iter().flatten() {
        let acc_id = crate::data::s(&jl["account_id"]);
        let acc_label = account_name(st, &acc_id);
        let debit = crate::data::s(&jl["debit"]);
        let credit = crate::data::s(&jl["credit"]);
        if !debit.is_empty() && debit != "0" {
            lines.push((acc_label, format!("{} {}", st.fmt.t("Debit"), st.fmt.money(&debit))));
        } else if !credit.is_empty() && credit != "0" {
            lines.push((acc_label, format!("{} {}", st.fmt.t("Credit"), st.fmt.money(&credit))));
        }
    }
    let key = st.mint_key("payrev");
    let expected_generation = preview["_generation"].clone();
    let pid = payment_id.to_string();
    st.confirm = Some(crate::ui::confirm::Confirm::one(
        st.fmt.t("Reverse payment"),
        st.fmt.t("Creates an offsetting payment and restores open items."),
        lines,
        "Confirm reversal",
        st.fmt.t("Reverse payment"),
        true,
        move |st| {
            let mut payload = json!({
                "payment_id": pid,
                "request_key": key,
                "source_id": key,
                "reason": st.fmt.t("Payment reversal"),
                "reference": key,
            });
            if !expected_generation.is_null() {
                payload["expected_generation"] = expected_generation.clone();
            }
            let _ = st.run("payment_post", payload, Some("Payment reversed"));
        },
    ));
}

pub fn open_cheque_spend(st: &mut State, cheque_id: &str) {
    let Some(entry) = st.books.workspace["cheques"]
        .as_array()
        .and_then(|rows| rows.iter().find(|e| crate::data::s(&e["cheque"]["id"]) == cheque_id))
        .cloned()
    else {
        return;
    };
    st.forms.clear_prefix("spend-");
    st.payments.spend_cheque_id = cheque_id.to_string();
    st.payments.spend_cheque_amount = crate::data::s(&entry["cheque"]["amount"]);
    st.payments.spend_cheque_number = crate::data::s(&entry["cheque"]["details"]["number"]);
    st.payments.spend_party_id.clear();
    st.payments.spend_party_combo = Default::default();
    st.payments.spend_key = st.mint_key("spend");
    st.payments.spend_posting = false;
    st.payments.spend_open = true;
}

pub fn close_spend_or_confirm(st: &mut State) {
    let dirty = st.forms.any_with_prefix("spend-") || !st.payments.spend_party_id.is_empty();
    if !dirty {
        st.payments.spend_open = false;
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.payments.spend_open = false);
}

fn mount_spend_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    use crate::ui::pickers;
    use crate::ui::resources::ResourceKind;
    let mut portals = Vec::new();
    let mut opts = pickers::party_options(s, false, true);
    let q = s.payments.spend_party_combo.search().text.clone();
    opts = pickers::with_create_option(opts, &q, &ui.t("New supplier"));
    let party = pickers::entity_combo(
        ui,
        &mut portals,
        "spend-party",
        &s.payments.spend_party_combo,
        |st| &mut st.payments.spend_party_combo,
        &opts,
        if s.payments.spend_party_id.is_empty() {
            None
        } else {
            Some(s.payments.spend_party_id.as_str())
        },
        &ui.t("Select contact"),
        pickers::PICKER_WIDTH,
        move |st, id| {
            if let Some(name) = pickers::create_text(id) {
                if let Ok(pid) = st.create_party(ResourceKind::Vendor, name, "", "", "") {
                    st.payments.spend_party_id = pid;
                }
            } else {
                st.payments.spend_party_id = id.to_string();
            }
        },
    );
    let form = col()
        .gap(14.0)
        .child(ui.labeled("Vendor", party))
        .child(ui.form_grid(
            ui.labeled("Amount", ui.body(ui.f.money(&s.payments.spend_cheque_amount))),
            ui.date_field(s, "spend-date", "Date"),
        ))
        .child(ui.form_grid(
            ui.labeled("Reference", ui.field(s, "spend-ref", "")),
            ui.labeled("Description", ui.field(s, "spend-reason", &ui.t("Note (optional)"))),
        ));
    ui.form_dialog(
        layer,
        &mut portals,
        "cheque-spend-dialog",
        &ui.t("Endorse to supplier"),
        &ui.t("Endorse a cheque I hold"),
        true,
        DIALOG_MD,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close_spend_or_confirm),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                if st.payments.spend_posting || !st.payments.spend_open {
                    return;
                }
                st.payments.spend_posting = true;
                if st.payments.spend_party_id.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Select contact"));
                    st.payments.spend_posting = false;
                    return;
                }
                let amount: i64 = st.payments.spend_cheque_amount.parse().unwrap_or(0);
                if amount <= 0 {
                    st.toast(Tone::Danger, st.fmt.t("Enter an amount above zero."));
                    st.payments.spend_posting = false;
                    return;
                }
                let Ok(date) = st.form_date("spend-date") else {
                    st.payments.spend_posting = false;
                    return;
                };
                if st.payments.spend_key.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Reopen this form and try again."));
                    st.payments.spend_posting = false;
                    return;
                }
                let key = st.payments.spend_key.clone();
                let (allocations, advance) = build_allocations(st, "make", &st.payments.spend_party_id, amount);
                let reference = {
                    let r = st.forms.get("spend-ref");
                    if r.trim().is_empty() { key.clone() } else { r }
                };
                let reason = {
                    let r = st.forms.get("spend-reason");
                    if r.trim().is_empty() { st.fmt.t("Endorse to supplier") } else { r }
                };
                let mut payload = json!({
                    "cheque_id": st.payments.spend_cheque_id,
                    "side": "make",
                    "date": date,
                    "party_id": st.payments.spend_party_id,
                    "funds_role": "cheques_receivable",
                    "reference": reference,
                    "reason": reason,
                    "source_id": key,
                    "request_key": key,
                    "advance_amount": advance,
                    "allocations": allocations,
                });
                match st.query("payment_preview", payload.clone()) {
                    Ok(pr) => {
                        if !pr["_generation"].is_null() {
                            payload["expected_generation"] = pr["_generation"].clone();
                        }
                    }
                    Err(e) => {
                        st.toast_err(e);
                        st.payments.spend_posting = false;
                        return;
                    }
                }
                match st.run("cheque_spend", payload, Some("Cheque endorsed")) {
                    Ok(_) => {
                        st.payments.spend_open = false;
                        st.payments.spend_posting = false;
                        st.payments.spend_key.clear();
                    }
                    Err(_) => {
                        st.payments.spend_posting = false;
                    }
                }
            }),
        ],
        close_spend_or_confirm,
    );
}

fn cheque_dispatch_action(act: &str) -> Option<&'static str> {
    match act {
        "deposit" => Some("cheque_deposit"),
        "clear" => Some("cheque_clear"),
        "bounce" => Some("cheque_bounce"),
        "return" => Some("cheque_return"),
        "void" => Some("cheque_void"),
        _ => None,
    }
}

fn fund_accounts(s: &State) -> Vec<(String, String)> {
    s.books.banking["accounts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let acc = if r.get("account").is_some() { &r["account"] } else { r };
            let id = crate::data::s(&acc["id"]);
            if id.is_empty() {
                return None;
            }
            let name = crate::data::s(&acc["name"]);
            Some((id.clone(), if name.is_empty() { id } else { name }))
        })
        .collect()
}

fn cheque_detail_panel(s: &State, ui: &Ui) -> Option<El> {
    if s.payments.selected_cheque.is_empty() {
        return None;
    }
    let entry = s.books.workspace["cheques"].as_array().and_then(|rows| {
        rows.iter()
            .find(|e| crate::data::s(&e["cheque"]["id"]) == s.payments.selected_cheque)
    })?;
    let id = crate::data::s(&entry["cheque"]["id"]);
    let details = &entry["cheque"]["details"];
    let number = first_nonempty(crate::data::s(&details["number"]), id.clone());
    let status = crate::data::s(&entry["status"]);
    let status_label = ui.t(cheque_status_label(&status));
    let bank = crate::data::s(&details["bank"]);
    let branch = crate::data::s(&details["branch"]);
    let bank_text = if branch.is_empty() {
        bank
    } else if bank.is_empty() {
        branch
    } else {
        format!("{bank} · {branch}")
    };
    let due_raw = crate::data::s(&details["due_date"]);
    let amount_raw = crate::data::s(&entry["cheque"]["amount"]);

    let mut actions = row().key(format!("cheque-actions-{id}-{status}")).w_full().gap(8.0).flex_wrap();
    let mut has_action = false;
    for act in entry["allowed"].as_array().into_iter().flatten().filter_map(|v| v.as_str()) {
        let Some(button_label) = cheque_action_label(act) else {
            continue;
        };
        has_action = true;
        let cheque_id = id.clone();
        if act == "spend" {
            actions = actions.child(ui.btn(Btn::Glass, button_label, None, move |st| {
                open_cheque_spend(st, &cheque_id);
            }));
        } else {
            let act_owned = act.to_string();
            let key = format!("cheque-act-{act}");
            actions = actions.child(ui.button(Btn::Glass, &key, &ui.t(button_label), None, move |st| {
                open_cheque_confirm(st, &cheque_id, &act_owned);
            }));
        }
    }

    let mut summary = row()
        .w_full()
        .gap(12.0)
        .center_y()
        .child(ui.title(number).grow(1.0).min_w(0.0))
        .child(ui.muted(ui.f.money(&amount_raw)))
        .child(ui.muted(ui.f.date(&due_raw)));
    if !bank_text.is_empty() {
        summary = summary.child(ui.muted(bank_text));
    }
    summary = summary.child(ui.muted(status_label));

    let mut panel = col().key("cheque-detail").w_full().gap(8.0).child(summary);
    if has_action {
        panel = panel.child(actions);
    }
    Some(panel)
}

pub fn open_cheque_confirm(st: &mut State, cheque_id: &str, act: &str) {
    let Some(button_label) = cheque_action_label(act) else {
        return;
    };
    let Some(action) = cheque_dispatch_action(act) else {
        return;
    };
    let entry = st.books.workspace["cheques"]
        .as_array()
        .and_then(|rows| rows.iter().find(|e| crate::data::s(&e["cheque"]["id"]) == cheque_id))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let number = crate::data::s(&entry["cheque"]["details"]["number"]);
    let cheque_number = if number.is_empty() { cheque_id.to_string() } else { number };
    let amount_raw = crate::data::s(&entry["cheque"]["amount"]);
    let due_date_raw = crate::data::s(&entry["cheque"]["details"]["due_date"]);

    let req = st.mint_key(action);
    let mut payload = json!({ "cheque_id": cheque_id, "request_key": req, "source_id": req });
    let generation = crate::data::s(&st.books.workspace["_generation"]);
    if !generation.is_empty() {
        payload["expected_generation"] = json!(generation);
    }
    let lines = vec![
        (st.fmt.t("Cheque number"), cheque_number),
        (st.fmt.t("Amount"), st.fmt.money(&amount_raw)),
        (st.fmt.t("Cheque due date"), st.fmt.date(&due_date_raw)),
        (st.fmt.t("Action"), st.fmt.t(button_label)),
    ];
    let danger = matches!(action, "cheque_bounce" | "cheque_return" | "cheque_void");
    if action == "cheque_clear" {
        let accounts = fund_accounts(st);
        if accounts.is_empty() {
            st.toast(Tone::Danger, st.fmt.t("Select account"));
            return;
        }
        let choices = if accounts.len() == 1 {
            let fund = accounts[0].0.clone();
            let mut p = payload.clone();
            p["fund_account_id"] = json!(fund);
            vec![crate::ui::confirm::ConfirmChoice {
                key: button_label.to_string(),
                label: st.fmt.t(button_label),
                danger,
                action: std::rc::Rc::new(move |st| {
                    let _ = st.run(action, p.clone(), Some("Posted"));
                }),
            }]
        } else {
            accounts
                .into_iter()
                .map(|(acc_id, acc_name)| {
                    let mut p = payload.clone();
                    p["fund_account_id"] = json!(acc_id);
                    crate::ui::confirm::ConfirmChoice {
                        key: acc_name.clone(),
                        label: acc_name,
                        danger,
                        action: std::rc::Rc::new(move |st| {
                            let _ = st.run(action, p.clone(), Some("Posted"));
                        }),
                    }
                })
                .collect()
        };
        st.confirm = Some(crate::ui::confirm::Confirm {
            title: st.fmt.t(button_label),
            message: st.fmt.t("Confirm this cheque action."),
            lines,
            choices,
        });
    } else {
        st.confirm = Some(crate::ui::confirm::Confirm::one(
            st.fmt.t(button_label),
            st.fmt.t("Confirm this cheque action."),
            lines,
            button_label,
            st.fmt.t(button_label),
            danger,
            move |st| {
                let _ = st.run(action, payload.clone(), Some("Posted"));
            },
        ));
    }
}
