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
    pub modal_open: bool,
    pub receive: bool,
    pub party_id: String,
    pub selected: String,
    pub party_combo: rune_kit::ComboboxState,
    pub funds: usize, // 0 cash, 1 bank, 2 cheque
    pub post_key: String,
    pub posting: bool,
    pub rows: Vec<Row>,
    pub rows_stamp: u64,
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
    let status = s.fmt.t("Posted");
    payments(s)
        .iter()
        .map(|record| {
            let id = crate::data::s(&record["id"]);
            let cmd = &record["command"];
            let data = &record["data"];
            let ref_no = first_nonempty(crate::data::s(&data["reference"]), crate::data::s(&cmd["reference"]));
            let ref_text = if ref_no.is_empty() {
                format!("#{}", &id[..id.len().min(8)])
            } else {
                ref_no
            };
            let party_id = first_nonempty(crate::data::s(&data["party"]["id"]), crate::data::s(&cmd["party_id"]));
            let name = s.books.party_name(&party_id);
            let date_raw = crate::data::s(
                &cmd.get("context")
                    .and_then(|c| c.get("effective_date"))
                    .cloned()
                    .unwrap_or_else(|| record["date"].clone()),
            );
            let amount_raw = first_nonempty(crate::data::s(&data["applied"]), crate::data::s(&record["total"]));
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

pub fn ensure(st: &mut State) {
    if st.payments.rows_stamp == st.stamp {
        return;
    }
    st.payments.rows = build_rows(st);
    st.payments.rows_stamp = st.stamp;
    reapply_sort(&st.payments.table, &mut st.payments.rows, col_index);
}

pub fn apply_sort(st: &mut State, col: &str, dir: SortDir) {
    st.payments.table.set_sort(col, dir);
    if let Some(idx) = col_index(col) {
        sort_rows(&mut st.payments.rows, idx, dir);
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

    let mut body = col().w_full().gap(16.0).grow(1.0).min_h(0.0).child(metrics).child(ui.fill_or_empty(
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
    body = body.child(cheque_strip(s, ui));

    if s.payments.modal_open {
        mount_payment_dialog(s, ui, layer);
    }

    ui.page(
        "payments-screen",
        &ui.t("Payments"),
        &ui.t("Settle invoices and bills"),
        vec![
            ui.btn(Btn::Primary, "Receive money", Some(&icons::ARROW), |st| open_payment(st, true)),
            ui.btn(Btn::Glass, "Pay bill", Some(&icons::PURCHASES), |st| open_payment(st, false)),
            ui.btn_enabled(Btn::Glass, "Open", None, !s.payments.selected.is_empty(), |st| {
                let id = st.payments.selected.clone();
                crate::ui::detail::open(st, &id);
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
    let mut form = col()
        .gap(14.0)
        .child(ui.labeled(if customers { "Customer" } else { "Vendor" }, party))
        .child(ui.section("pay-funds-sec", &ui.t("Cash and bank"), funds_bar))
        .child(ui.form_grid(
            ui.labeled("Amount", ui.money_field(s, "pay-amount")),
            ui.date_field(s, "pay-date", "Date"),
        ))
        .child(ui.form_grid(
            ui.labeled("Reference", ui.field(s, "pay-ref", "")),
            ui.labeled("Description", ui.field(s, "pay-reason", &ui.t("Note (optional)"))),
        ));
    if s.payments.funds == 2 {
        form = form
            .child(ui.form_grid(
                ui.labeled("Cheque", ui.field(s, "pay-sayad", "")),
                ui.labeled("Number", ui.field(s, "pay-cheque-no", "")),
            ))
            .child(ui.form_grid(
                ui.labeled("Bank", ui.field(s, "pay-bank", "")),
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
                let opts = st.query(
                    "payment_options",
                    json!({
                        "side": side,
                        "party_id": st.payments.party_id
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
                let advance = left.to_string();
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
                    let sayad = crate::i18n::latin_digits(&st.forms.get("pay-sayad"));
                    let number = st.forms.get("pay-cheque-no");
                    let bank = st.forms.get("pay-bank");
                    if sayad.trim().is_empty() || number.trim().is_empty() || st.forms.get("pay-due").trim().is_empty() {
                        st.toast(Tone::Danger, st.fmt.t("Enter the cheque's Sayad ID, number and due date."));
                        st.payments.posting = false;
                        return;
                    }
                    let sayad_trimmed = sayad.trim();
                    if sayad_trimmed.len() != 16 || !sayad_trimmed.chars().all(|c| c.is_ascii_digit()) {
                        st.toast(Tone::Danger, st.fmt.t("A Sayad ID has exactly 16 digits."));
                        st.payments.posting = false;
                        return;
                    }
                    let Ok(due) = st.form_date("pay-due") else {
                        st.payments.posting = false;
                        return;
                    };
                    payload["cheque"] = json!({
                        "sayad_id": sayad_trimmed,
                        "number": number.trim(),
                        "bank": bank.trim(),
                        "branch": "",
                        "due_date": due,
                        "drawer_national_id": "",
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
        // Endorse / spend stays a separate flow; do not offer it from this strip.
        "spend" => None,
        _ => None,
    }
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

fn cheque_strip(s: &State, ui: &Ui) -> El {
    let rows = s.books.workspace["cheques"].as_array().cloned().unwrap_or_default();
    if rows.is_empty() {
        return ui.muted(ui.t("No cheques yet"));
    }
    let mut col_el = col().w_full().gap(8.0).child(ui.heading(ui.t("Cheques")));
    for entry in rows.iter().take(8) {
        let id = crate::data::s(&entry["cheque"]["id"]);
        let number = crate::data::s(&entry["cheque"]["details"]["number"]);
        let status = crate::data::s(&entry["status"]);
        let allowed = entry["allowed"].as_array().cloned().unwrap_or_default();
        let label = if number.is_empty() { id.clone() } else { number.clone() };
        let status_label = ui.t(cheque_status_label(&status));
        let mut actions = row().w_full().gap(8.0).flex_wrap();
        let mut has_action = false;
        for act in allowed.iter().filter_map(|v| v.as_str()) {
            let Some(button_label) = cheque_action_label(act) else {
                continue;
            };
            let Some(action) = cheque_dispatch_action(act) else {
                continue;
            };
            has_action = true;
            let cheque_id = id.clone();
            let act_owned = act.to_string();
            let key = format!("cheque-{action}-{cheque_id}");
            actions = actions.child(ui.button(Btn::Glass, &key, &ui.t(button_label), None, move |st| {
                open_cheque_confirm(st, &cheque_id, &act_owned);
            }));
        }
        let mut line = col().key(format!("cheque-row-{id}")).w_full().gap(6.0).child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(ui.body(label).grow(1.0).min_w(0.0))
                .child(ui.muted(status_label)),
        );
        if has_action {
            line = line.child(actions);
        }
        col_el = col_el.child(line);
    }
    col_el
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
