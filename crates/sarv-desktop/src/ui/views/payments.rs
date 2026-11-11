//! Payments — receivables/payables KPIs and kit Table of settlements.

use std::borrow::Cow;

use rune::{Align, Cx, El, col, row};
use rune_kit::{Cell, Table, TableColumn, TableState};

use serde_json::json;

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;

#[derive(Default)]
pub struct PaymentsState {
    pub tab_idx: usize,
    pub table: TableState,
    pub modal_open: bool,
    pub receive: bool,
    pub party_id: String,
    pub party_combo: rune_kit::ComboboxState,
    pub funds: usize, // 0 cash, 1 bank, 2 cheque
    pub post_key: String,
    pub posting: bool,
}

fn first_nonempty(a: String, b: String) -> String {
    if a.is_empty() { b } else { a }
}

fn payments(s: &State) -> Vec<serde_json::Value> {
    s.books.list("payments")
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    if !s.books.initialized() {
        return ui.page(
            "payments-screen",
            &ui.t("Payments"),
            "",
            vec![],
            ui.kit_card()
                .title(ui.t("Confirm starting balances to settle invoices and bills."), Some(&icons::PAYMENTS))
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
        |s: &State| payments(s).len(),
        |s: &State, i| Cow::Owned(format!("payment-{}", crate::data::s(&payments(s)[i]["id"]))),
    )
    .name(ui.t("Payments"))
    .column(TableColumn::new("ref", ui.t("Reference")).width(140.0).min(96.0))
    .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
    .column(TableColumn::new("date", ui.t("Date")).width(110.0).min(88.0))
    .column(TableColumn::new("amount", ui.t("Amount")).width(160.0).min(100.0).align(Align::End).sortable())
    .column(TableColumn::new("status", ui.t("Status")).width(110.0).min(80.0))
    .cell(|s: &State, i, col| {
        let list = payments(s);
        let Some(record) = list.get(i) else {
            return Cell::text("");
        };
        let id = crate::data::s(&record["id"]);
        let cmd = &record["command"];
        let data = &record["data"];
        let ref_no = first_nonempty(crate::data::s(&data["reference"]), crate::data::s(&cmd["reference"]));
        let party_id = first_nonempty(crate::data::s(&data["party"]["id"]), crate::data::s(&cmd["party_id"]));
        match col {
            "ref" => Cell::text(if ref_no.is_empty() {
                format!("#{}", &id[..id.len().min(8)])
            } else {
                ref_no
            }),
            "name" => Cell::text(s.books.party_name(&party_id)),
            "date" => {
                let date_raw = cmd
                    .get("context")
                    .and_then(|c| c.get("effective_date"))
                    .cloned()
                    .unwrap_or_else(|| record["date"].clone());
                Cell::text(s.fmt.date(&crate::data::s(&date_raw))).muted()
            }
            "amount" => {
                let amount = first_nonempty(crate::data::s(&data["applied"]), crate::data::s(&record["total"]));
                Cell::text(s.fmt.money(&amount)).mono()
            }
            _ => Cell::text(s.fmt.t("Posted")).muted(),
        }
    })
    .zebra(true)
    .build(s, &ui.k)
    .key("payments-table")
    .grow(1.0)
    .min_h(0.0);

    let body = col()
        .w_full()
        .gap(16.0)
        .grow(1.0)
        .min_h(0.0)
        .child(metrics)
        .child(ui.fill_or_empty(payments(s).len(), "No payments recorded yet", "Settle invoices and bills", &icons::PAYMENTS, table))
        .child(cheque_strip(s, ui));

    if s.payments.modal_open {
        mount_payment_dialog(s, ui, layer);
    }

    ui.page(
        "payments-screen",
        &ui.t("Payments"),
        &ui.t("Settle invoices and bills"),
        vec![
            ui.btn(Btn::Primary, "Receive money", Some(&icons::ARROW), |st| {
                st.payments.modal_open = true;
                st.payments.receive = true;
                st.payments.party_id.clear();
                st.payments.party_combo = Default::default();
                st.payments.funds = 0;
                st.forms.set("pay-amount", "");
                st.forms.set("pay-ref", "");
                st.forms.set("pay-reason", "");
                st.payments.post_key = format!("pay-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
                st.payments.posting = false;
            }),
            ui.btn(Btn::Glass, "Pay bill", Some(&icons::PURCHASES), |st| {
                st.payments.modal_open = true;
                st.payments.receive = false;
                st.payments.party_id.clear();
                st.payments.party_combo = Default::default();
                st.payments.funds = 0;
                st.forms.set("pay-amount", "");
                st.forms.set("pay-ref", "");
                st.forms.set("pay-reason", "");
                st.payments.post_key = format!("pay-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
                st.payments.posting = false;
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
    opts = pickers::with_create_option(
        opts,
        &q,
        &if customers { ui.t("New customer") } else { ui.t("New supplier") },
    );
    let party = pickers::entity_combo(
        ui,
        &mut portals,
        "pay-party",
        &s.payments.party_combo,
        |st| &mut st.payments.party_combo,
        &opts,
        if s.payments.party_id.is_empty() { None } else { Some(s.payments.party_id.as_str()) },
        &ui.t("Select contact"),
        pickers::PICKER_WIDTH,
        move |st, id| {
            if let Some(name) = pickers::create_text(id) {
                let kind = if st.payments.receive { ResourceKind::Customer } else { ResourceKind::Vendor };
                match st.create_party(kind, name, "", "", "") {
                    Ok(pid) => st.payments.party_id = pid,
                    Err(e) => st.toast_err(e),
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
            ui.labeled("Amount", ui.field(s, "pay-amount", "۰")),
            ui.labeled("Reference", ui.field(s, "pay-ref", "")),
        ))
        .child(ui.labeled("Description", ui.field(s, "pay-reason", &ui.t("Note (optional)"))));
    if s.payments.funds == 2 {
        form = form.child(ui.form_grid(
            ui.labeled("Cheque", ui.field(s, "pay-sayad", "")),
            ui.labeled("Number", ui.field(s, "pay-cheque-no", "")),
        )).child(ui.form_grid(
            ui.labeled("Bank", ui.field(s, "pay-bank", "")),
            ui.labeled("Due date", ui.field(s, "pay-due", &crate::data::today())),
        ));
    }

    let title = if s.payments.receive { ui.t("Receive money") } else { ui.t("Pay bill") };
    ui.dialog_described(
        layer,
        &mut portals,
        "payment",
        &title,
        &ui.t("Record a cash or bank settlement."),
        true,
        520.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.payments.modal_open = false),
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
                let amount = st.parse_money_input(&st.forms.get("pay-amount"));
                if amount <= 0 {
                    st.toast(Tone::Danger, st.fmt.t("Amount"));
                    st.payments.posting = false;
                    return;
                }
                let side = if st.payments.receive { "receive" } else { "make" };
                let funds_role = match st.payments.funds {
                    0 => "cash",
                    1 => "bank",
                    _ => if st.payments.receive { "cheques_receivable" } else { "cheques_payable" },
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
                    if r.trim().is_empty() { "Payment".into() } else { r }
                };
                let key = if st.payments.post_key.is_empty() {
                    format!("pay-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
                } else {
                    st.payments.post_key.clone()
                };
                let reference = {
                    let r = st.forms.get("pay-ref");
                    if r.trim().is_empty() { key.clone() } else { r }
                };
                let mut payload = json!({
                    "side": side,
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
                    let due = st.forms.get("pay-due");
                    if sayad.trim().is_empty() || number.trim().is_empty() || due.trim().is_empty() {
                        st.toast(Tone::Danger, st.fmt.t("Cheque"));
                        st.payments.posting = false;
                        return;
                    }
                    payload["cheque"] = json!({
                        "sayad_id": sayad.trim(),
                        "number": number.trim(),
                        "bank": bank.trim(),
                        "branch": "",
                        "due_date": due.trim(),
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
                let preview_action = if st.payments.funds == 2 { "payment_preview" } else { "payment_preview" };
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
                    }
                    Err(_) => {
                        // `run` already toasted the engine error.
                        st.payments.posting = false;
                    }
                }
            }),
        ],
        |st| st.payments.modal_open = false,
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

fn first_fund_account(s: &State) -> Option<String> {
    let rows = s.books.banking["accounts"].as_array()?;
    let pick = |want: &str| {
        rows.iter()
            .find(|r| crate::data::s(&r["role"]).eq_ignore_ascii_case(want))
            .map(|r| crate::data::s(&r["account"]["id"]))
            .filter(|id| !id.is_empty())
    };
    pick("Bank")
        .or_else(|| pick("Cash"))
        .or_else(|| {
            rows.first()
                .map(|r| crate::data::s(&r["account"]["id"]))
                .filter(|id| !id.is_empty())
        })
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
        let label = if number.is_empty() { id.clone() } else { number };
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
            let needs_fund = action == "cheque_clear";
            let key = format!("cheque-{action}-{cheque_id}");
            actions = actions.child(ui.button(Btn::Glass, &key, &ui.t(button_label), None, move |st| {
                let req = format!("{action}-{cheque_id}");
                let mut payload = json!({ "cheque_id": cheque_id, "request_key": req, "source_id": req });
                if needs_fund {
                    let Some(fund) = first_fund_account(st) else {
                        st.toast(Tone::Danger, st.fmt.t("Select account"));
                        return;
                    };
                    payload["fund_account_id"] = json!(fund);
                }
                let _ = st.run(action, payload, Some("Posted"));
            }));
        }
        let mut line = col()
            .key(format!("cheque-row-{id}"))
            .w_full()
            .gap(6.0)
            .child(
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
