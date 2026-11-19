//! Banking and cash accounts — kit Table + metric cards.

use std::borrow::Cow;

use rune::{Cx, El, col, row};
use rune_kit::{Cell, Checkbox, SortDir, Table, TableColumn, TableState};

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, DIALOG_LG, Ui};
use crate::ui::icons;
use crate::ui::rows::{Row, SortKey, reapply_sort, sort_rows};

#[derive(Default)]
pub struct BankingState {
    pub modal_open: bool,
    pub table: TableState,
    pub selected: String,
    pub from_id: String,
    pub to_id: String,
    /// 0 transfer, 1 deposit (money in), 2 withdrawal (money out).
    pub kind: usize,
    pub post_key: String,
    pub posting: bool,
    pub rows: Vec<Row>,
    pub rows_stamp: u64,
    pub rec_open: bool,
    pub rec_account_id: String,
    pub rec_data: serde_json::Value,
    pub rec_key: String,
}

fn bank_rows(s: &State) -> Vec<serde_json::Value> {
    s.books.banking["accounts"].as_array().cloned().unwrap_or_default()
}

fn option_list(rows: &[serde_json::Value], id_key: &str) -> Vec<(String, String)> {
    rows.iter()
        .map(|row| {
            let acc = if row.get("account").is_some() { &row["account"] } else { row };
            let id = crate::data::s(&acc[id_key]);
            let label = format!("{} {}", crate::data::s(&acc["code"]), crate::data::s(&acc["name"]));
            (id, label.trim().to_string())
        })
        .filter(|(id, _)| !id.is_empty())
        .collect()
}

fn funds_options(s: &State) -> Vec<(String, String)> {
    option_list(&bank_rows(s), "id")
}

fn counterpart_options(s: &State) -> Vec<(String, String)> {
    s.books.banking["counterparts"]
        .as_array()
        .map(|rows| option_list(rows, "id"))
        .unwrap_or_default()
}

fn col_index(col: &str) -> Option<usize> {
    match col {
        "code" => Some(0),
        "name" => Some(1),
        "class" => Some(2),
        "balance" => Some(3),
        _ => None,
    }
}

fn build_rows(s: &State) -> Vec<Row> {
    bank_rows(s)
        .iter()
        .map(|row| {
            let acc = &row["account"];
            let code = crate::data::s(&acc["code"]);
            let name = crate::data::s(&acc["name"]);
            let class = s.fmt.t(&crate::data::s(&row["role"]));
            let bal_raw = crate::data::s(&row["balance"]);
            Row {
                id: format!("bank-acc-{}", crate::data::s(&acc["id"])),
                cells: vec![code.clone(), name.clone(), class.clone(), s.fmt.money(&bal_raw)],
                keys: vec![
                    SortKey::text(&code),
                    SortKey::text(&name),
                    SortKey::text(&class),
                    SortKey::num_str(&bal_raw),
                ],
            }
        })
        .collect()
}

pub fn ensure(st: &mut State) {
    if st.banking.rows_stamp == st.stamp {
        return;
    }
    st.banking.rows = build_rows(st);
    st.banking.rows_stamp = st.stamp;
    reapply_sort(&st.banking.table, &mut st.banking.rows, col_index);
}

pub fn apply_sort(st: &mut State, col: &str, dir: SortDir) {
    st.banking.table.set_sort(col, dir);
    if let Some(idx) = col_index(col) {
        sort_rows(&mut st.banking.rows, idx, dir);
    }
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let table = Table::new(
        |s: &State| &s.banking.table,
        |s: &mut State| &mut s.banking.table,
        |s: &State| s.banking.rows.len(),
        |s: &State, i| Cow::Owned(s.banking.rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
    )
    .name(ui.t("Banking"))
    .column(TableColumn::new("code", ui.t("Code")).width(100.0).min(72.0).sortable())
    .column(TableColumn::new("name", ui.t("Account")).flex(2.0).min(140.0).sortable())
    .column(TableColumn::new("class", ui.t("Classification")).width(140.0).min(100.0).sortable())
    .column(
        TableColumn::new("balance", ui.t("Balance"))
            .width(160.0)
            .min(110.0)
            .align(rune::Align::End)
            .sortable(),
    )
    .cell(|s: &State, i, col| {
        let Some(row) = s.banking.rows.get(i) else {
            return Cell::text("");
        };
        match col {
            "code" => Cell::text(&row.cells[0]),
            "name" => Cell::text(&row.cells[1]),
            "balance" => Cell::text(&row.cells[3]).mono(),
            _ => Cell::text(&row.cells[2]).muted(),
        }
    })
    .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
    .on_select(|st, id, _| st.banking.selected = id.strip_prefix("bank-acc-").unwrap_or(id).to_string())
    .on_open(|st, id, _| {
        crate::ui::views::accounting::open_ledger_for(st, id.strip_prefix("bank-acc-").unwrap_or(id));
    })
    .zebra(true)
    .build(s, &ui.k)
    .key("banking-table")
    .grow(1.0)
    .min_h(0.0);

    let kpis = row()
        .w_full()
        .gap(12.0)
        .child(ui.metric(
            "kpi-bank-bal",
            &icons::WALLET,
            &ui.t("Cash and bank"),
            ui.f.money(&s.books.position("cash")),
            ui.t("Available funds"),
            Tone::Info,
        ))
        .child(ui.metric(
            "kpi-bank-accounts",
            &icons::BANKING,
            &ui.t("Accounts"),
            ui.f.digits(&s.banking.rows.len().to_string()),
            ui.t("Cash and bank"),
            Tone::Success,
        ))
        .child(ui.metric(
            "kpi-bank-rec",
            &icons::SALES,
            &ui.t("Receivables"),
            ui.f.money(&s.books.position("receivables")),
            ui.t("Open customer invoices"),
            Tone::Warning,
        ));

    let body = col().w_full().gap(16.0).grow(1.0).min_h(0.0).child(kpis).child(ui.fill_or_empty(
        s.banking.rows.len(),
        "No accounts",
        "Cash and bank",
        &icons::WALLET,
        table,
    ));

    if s.banking.modal_open {
        mount_transfer_dialog(s, ui, layer);
    }
    if s.banking.rec_open {
        mount_reconcile_dialog(s, ui, layer);
    }

    let mut actions = vec![
        ui.btn(Btn::Primary, "Transfer", Some(&icons::ARROW), |st| open_move(st, 0)),
        ui.btn(Btn::Glass, "Money in", None, |st| open_move(st, 1)),
        ui.btn(Btn::Glass, "Money out", None, |st| open_move(st, 2)),
    ];
    if !s.banking.selected.is_empty() {
        actions.push(ui.btn(Btn::Glass, "Reconcile", Some(&icons::CHECK), |st| {
            let aid = st.banking.selected.clone();
            open_reconcile(st, &aid);
        }));
    }

    ui.page("banking-screen", &ui.t("Banking"), &ui.t("Banking and payments"), actions, body)
}

fn mount_transfer_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    use crate::ui::pickers;
    use serde_json::json;

    let mut portals = Vec::new();
    let funds = funds_options(s);
    let other = if s.banking.kind == 0 {
        funds.clone()
    } else {
        counterpart_options(s)
    };
    let accounts = funds;
    let targets = other;
    let from_open = s.selects.get("bank-from").copied().unwrap_or(false);
    let to_open = s.selects.get("bank-to").copied().unwrap_or(false);
    let from = pickers::select_field(
        ui,
        &mut portals,
        "bank-from",
        &accounts,
        if s.banking.from_id.is_empty() {
            None
        } else {
            Some(s.banking.from_id.as_str())
        },
        &ui.t("Select account"),
        pickers::ACCOUNT_PICKER_WIDTH,
        from_open,
        |st, id| st.banking.from_id = id.to_string(),
    );
    let to = pickers::select_field(
        ui,
        &mut portals,
        "bank-to",
        &targets,
        if s.banking.to_id.is_empty() {
            None
        } else {
            Some(s.banking.to_id.as_str())
        },
        &ui.t("Select account"),
        pickers::ACCOUNT_PICKER_WIDTH,
        to_open,
        |st, id| st.banking.to_id = id.to_string(),
    );
    let form = col()
        .gap(14.0)
        .child(ui.form_grid(ui.labeled("From", from), ui.labeled("To", to)))
        .child(ui.form_grid(
            ui.labeled("Amount", ui.money_field(s, "bank-amount")),
            ui.date_field(s, "bank-date", "Date"),
        ))
        .child(ui.labeled("Description", ui.field(s, "bank-reason", &ui.t("Note (optional)"))));

    let title = move_dialog_title(s.banking.kind);

    ui.form_dialog(
        layer,
        &mut portals,
        "banking-move",
        &ui.t(title),
        &ui.t("Move money between cash and bank accounts."),
        true,
        DIALOG_LG,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close_or_confirm),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                if st.banking.posting || !st.banking.modal_open {
                    return;
                }
                st.banking.posting = true;
                if st.banking.from_id.is_empty() || st.banking.to_id.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Select account"));
                    st.banking.posting = false;
                    return;
                }
                let amt = match st.parse_positive_money(&st.forms.get("bank-amount")) {
                    Ok(v) => v,
                    Err(key) => {
                        st.toast(Tone::Danger, st.fmt.t(key));
                        st.banking.posting = false;
                        return;
                    }
                };
                let Ok(date) = st.form_date("bank-date") else {
                    st.banking.posting = false;
                    return;
                };
                let reason = {
                    let r = st.forms.get("bank-reason");
                    if r.trim().is_empty() { st.fmt.t("Transfer") } else { r }
                };
                let kind = match st.banking.kind {
                    1 => "deposit",
                    2 => "withdrawal",
                    _ => "transfer",
                };
                if st.banking.post_key.is_empty() {
                    st.banking.post_key = st.mint_key("bank");
                }
                let key = st.banking.post_key.clone();
                let mut payload = json!({
                    "kind": kind,
                    "date": date,
                    "account_id": st.banking.from_id,
                    "counterpart_id": st.banking.to_id,
                    "amount": amt.to_string(),
                    "reference": reason,
                    "reason": reason,
                    "source_id": key,
                    "request_key": key
                });
                match st.query("banking_preview", payload.clone()) {
                    Ok(pr) => payload["expected_generation"] = pr["_generation"].clone(),
                    Err(e) => {
                        st.toast_err(e);
                        st.banking.posting = false;
                        return;
                    }
                }
                if st.run("banking_post", payload, Some("Posted")).is_ok() {
                    st.banking.modal_open = false;
                    st.banking.post_key.clear();
                }
                st.banking.posting = false;
            }),
        ],
        close_or_confirm,
    );
}

fn is_dirty(st: &State) -> bool {
    st.forms.any_with_prefix("bank-") || !st.banking.from_id.is_empty() || !st.banking.to_id.is_empty()
}

pub fn close_or_confirm(st: &mut State) {
    if !is_dirty(st) {
        st.banking.modal_open = false;
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.banking.modal_open = false);
}

fn open_move(st: &mut State, kind: usize) {
    st.banking.modal_open = true;
    st.banking.kind = kind;
    st.banking.from_id.clear();
    st.banking.to_id.clear();
    st.banking.post_key = st.mint_key("bank");
    st.banking.posting = false;
    st.forms.clear_prefix("bank-");
}

pub fn move_dialog_title(kind: usize) -> &'static str {
    match kind {
        1 => "Money in",
        2 => "Money out",
        _ => "Transfer",
    }
}

fn reload_reconcile(st: &mut State) {
    let aid = st.banking.rec_account_id.clone();
    if aid.is_empty() {
        return;
    }
    match st.query(
        "reconciliation",
        serde_json::json!({ "account_id": aid, "as_of": crate::data::today() }),
    ) {
        Ok(v) => st.banking.rec_data = v,
        Err(e) => st.toast_err(e),
    }
}

pub fn open_reconcile(st: &mut State, account_id: &str) {
    st.banking.rec_account_id = account_id.to_string();
    st.banking.rec_key = st.mint_key("reconcile");
    st.forms.clear_prefix("rec-");
    st.forms.set("rec-date", &st.fmt.date_input(&crate::data::today()));
    st.banking.rec_open = true;
    reload_reconcile(st);
}

fn start_reconcile_session(st: &mut State) {
    let date_raw = st.forms.get("rec-date");
    let date = match st.fmt.parse_date(&date_raw) {
        Ok(d) => d,
        Err(e) => {
            st.toast(Tone::Warning, e);
            return;
        }
    };
    let bal_raw = st.forms.get("rec-balance");
    let trimmed = bal_raw.trim();
    let (neg, abs_raw) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest.trim()),
        None => (false, trimmed),
    };
    let bal_rials = match crate::i18n::parse_money(abs_raw, st.fmt.unit) {
        Ok(v) => {
            if neg {
                -v
            } else {
                v
            }
        }
        Err(_) => {
            st.toast(Tone::Warning, st.fmt.t("Enter an amount"));
            return;
        }
    };
    let generation = crate::data::s(&st.banking.rec_data["_generation"]);
    let key = st.mint_key("rec-start");
    let mut payload = serde_json::json!({
        "account_id": st.banking.rec_account_id,
        "statement_date": date,
        "statement_balance": bal_rials.to_string(),
        "request_key": key,
    });
    if !generation.is_empty() {
        payload["expected_generation"] = serde_json::json!(generation);
    }
    if st.run("reconciliation_start", payload, None).is_ok() {
        reload_reconcile(st);
    }
}

fn parse_line_index(v: &serde_json::Value) -> usize {
    v.as_u64().or_else(|| crate::data::s(v).parse::<u64>().ok()).unwrap_or(0) as usize
}

fn toggle_reconcile_line(st: &mut State, journal_id: &str, line_index: usize) {
    let rec_id = crate::data::s(&st.banking.rec_data["open_session"]["id"]);
    if rec_id.is_empty() {
        return;
    }
    let mut matched = Vec::new();
    for l in st.banking.rec_data["lines"].as_array().into_iter().flatten() {
        let jid = crate::data::s(&l["journal_id"]);
        let idx = parse_line_index(&l["line_index"]);
        let is_m = l["matched"].as_bool().unwrap_or(false);
        if jid == journal_id && idx == line_index {
            if !is_m {
                matched.push(serde_json::json!({ "journal_id": jid, "line_index": idx }));
            }
        } else if is_m {
            matched.push(serde_json::json!({ "journal_id": jid, "line_index": idx }));
        }
    }
    let generation = crate::data::s(&st.banking.rec_data["_generation"]);
    let key = st.mint_key("rec-match");
    let mut payload = serde_json::json!({
        "reconciliation_id": rec_id,
        "matched_lines": matched,
        "request_key": key,
    });
    if !generation.is_empty() {
        payload["expected_generation"] = serde_json::json!(generation);
    }
    if st.run("reconciliation_set_matched", payload, None).is_ok() {
        reload_reconcile(st);
    }
}

fn complete_reconcile_session(st: &mut State) {
    let rec_id = crate::data::s(&st.banking.rec_data["open_session"]["id"]);
    if rec_id.is_empty() {
        return;
    }
    let generation = crate::data::s(&st.banking.rec_data["_generation"]);
    let key = st.mint_key("rec-complete");
    let mut payload = serde_json::json!({
        "reconciliation_id": rec_id,
        "request_key": key,
    });
    if !generation.is_empty() {
        payload["expected_generation"] = serde_json::json!(generation);
    }
    if st
        .run("reconciliation_complete", payload, Some("Bank reconciliation completed"))
        .is_ok()
    {
        reload_reconcile(st);
        st.banking.rec_open = false;
    }
}

fn mount_reconcile_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let data = &s.banking.rec_data;
    let has_open = !data["open_session"].is_null();
    let mut body = col().w_full().gap(14.0);

    if !has_open {
        let prev = crate::data::s(&data["previous_reconciled_balance"]);
        let book = crate::data::s(&data["book_balance"]);
        body = body
            .child(
                row()
                    .w_full()
                    .gap(12.0)
                    .child(ui.metric(
                        "rec-kpi-prev",
                        &icons::CHECK,
                        &ui.t("Last reconciled"),
                        ui.f.money(&prev),
                        ui.t("Bank statement"),
                        Tone::Success,
                    ))
                    .child(ui.metric(
                        "rec-kpi-book",
                        &icons::WALLET,
                        &ui.t("Book balance"),
                        ui.f.money(&book),
                        ui.t("Cash and bank"),
                        Tone::Info,
                    )),
            )
            .child(ui.form_grid(
                ui.date_field(s, "rec-date", "Statement date"),
                ui.labeled("Statement balance", ui.money_field(s, "rec-balance")),
            ));
    } else {
        let stmt_bal = crate::data::s(&data["open_session"]["statement_balance"]);
        let cleared = crate::data::s(&data["cleared_balance"]);
        let diff = crate::data::s(&data["difference"]);
        let diff_zero = diff == "0" || diff.is_empty();
        body = body.child(
            row()
                .w_full()
                .gap(12.0)
                .child(ui.metric(
                    "rec-stmt",
                    &icons::BANKING,
                    &ui.t("Statement balance"),
                    ui.f.money(&stmt_bal),
                    ui.f.date(&crate::data::s(&data["open_session"]["statement_date"])),
                    Tone::Info,
                ))
                .child(ui.metric(
                    "rec-cleared",
                    &icons::CHECK,
                    &ui.t("Cleared balance"),
                    ui.f.money(&cleared),
                    ui.t("Matched entries"),
                    Tone::Success,
                ))
                .child(ui.metric(
                    "rec-diff",
                    &icons::WARNING,
                    &ui.t("Difference"),
                    ui.f.money(&diff),
                    ui.t(if diff_zero { "Balanced" } else { "Unreconciled" }),
                    if diff_zero { Tone::Success } else { Tone::Warning },
                )),
        );
        let mut list = col().w_full().gap(6.0);
        for l in data["lines"].as_array().into_iter().flatten() {
            let jid = crate::data::s(&l["journal_id"]);
            let idx = parse_line_index(&l["line_index"]);
            let matched = l["matched"].as_bool().unwrap_or(false);
            let date = ui.f.date(&crate::data::s(&l["date"]));
            let desc = crate::data::s(&l["description"]);
            let debit = crate::data::s(&l["debit"]);
            let credit = crate::data::s(&l["credit"]);
            let amt = if debit != "0" && !debit.is_empty() {
                format!("+{}", ui.f.money(&debit))
            } else {
                format!("−{}", ui.f.money(&credit))
            };
            let jid_cb = jid.clone();
            let cb = Checkbox::new(matched)
                .label(format!("{date} · {desc}"))
                .on_toggle(move |st: &mut State| toggle_reconcile_line(st, &jid_cb, idx))
                .build(&ui.k)
                .key(format!("rec-line-{jid}-{idx}"));
            list = list.child(
                row()
                    .w_full()
                    .center_y()
                    .justify_between()
                    .gap(12.0)
                    .child(cb.grow(1.0).min_w(0.0))
                    .child(ui.body(amt)),
            );
        }
        body = body.child(list);
    }

    let mut actions = vec![ui.btn(Btn::Ghost, "Close", None, |st| st.banking.rec_open = false)];
    if !has_open {
        actions.push(ui.btn(Btn::Primary, "Start reconciliation", Some(&icons::CHECK), start_reconcile_session));
    } else {
        let diff = crate::data::s(&data["difference"]);
        let diff_zero = diff == "0" || diff.is_empty();
        actions.push(ui.btn_enabled(
            Btn::Primary,
            "Complete reconciliation",
            Some(&icons::CHECK),
            diff_zero,
            complete_reconcile_session,
        ));
    }

    let mut portals = Vec::new();
    ui.form_dialog(
        layer,
        &mut portals,
        "bank-reconcile",
        &ui.t("Bank reconciliation"),
        &ui.t("Match ledger entries against your bank statement"),
        true,
        DIALOG_LG,
        body,
        actions,
        |st| st.banking.rec_open = false,
    );
}
