//! Accounting screen: journals, chart of accounts and fiscal periods — kit Table + tabs.

use std::borrow::Cow;

use rune::{Align, Cx, El, col, row};
use rune_kit::{Cell, SortDir, Switch, Table, TableColumn, TableState};

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, DIALOG_LG, DIALOG_MD, DIALOG_SM, Ui};
use crate::ui::icons;
use crate::ui::rows::{Row, SortKey, reapply_sort, sort_rows};

#[derive(Default)]
pub struct AccountingState {
    pub tab_idx: usize,
    pub table: TableState,
    pub modal_open: bool,
    pub debit_account: String,
    pub credit_account: String,
    pub selected: String,
    pub ledger_open: bool,
    pub ledger: serde_json::Value,
    pub ledger_account_id: String,
    pub ledger_table: TableState,
    pub ledger_selected: String,
    pub ledger_rows: Vec<serde_json::Value>,
    pub ledger_count: usize,
    pub ledger_opening: String,
    pub post_key: String,
    pub posting: bool,
    pub period_add_open: bool,
    pub period_add_busy: bool,
    pub next_period_start: String,
    pub next_period_end: String,
    pub acct_open: bool,
    pub acct_busy: bool,
    pub acct_class_idx: usize,
    pub acct_parent: String,
    pub acct_generation: String,
    pub acct_archive_open: bool,
    pub acct_archive_busy: bool,
    pub acct_show_archived: bool,
    pub rows: Vec<Row>,
    pub rows_stamp: u64,
}

const ACCOUNT_CLASSES: [(&str, &str); 5] = [
    ("Asset", "Debit"),
    ("Liability", "Credit"),
    ("Equity", "Credit"),
    ("Revenue", "Credit"),
    ("Expense", "Debit"),
];

pub fn open_account(st: &mut State) {
    st.forms.clear_prefix("acct-");
    st.accounting.acct_open = true;
    st.accounting.acct_busy = false;
    st.accounting.acct_class_idx = 0;
    st.accounting.acct_parent.clear();
    st.accounting.acct_generation = crate::data::s(&st.books.workspace["_generation"]);
}

fn acct_is_dirty(st: &State) -> bool {
    st.forms.any_with_prefix("acct-") || !st.accounting.acct_parent.is_empty()
}

pub(crate) fn close_acct_or_confirm(st: &mut State) {
    if !acct_is_dirty(st) {
        st.accounting.acct_open = false;
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.accounting.acct_open = false);
}

fn accounts(s: &State) -> &[serde_json::Value] {
    s.books.workspace["accounts"].as_array().map(|a| a.as_slice()).unwrap_or(&[])
}

pub fn open_journal(st: &mut State) {
    st.forms.clear_prefix("journal-");
    st.accounting.debit_account.clear();
    st.accounting.credit_account.clear();
    st.accounting.post_key = st.mint_key("journal");
    st.accounting.posting = false;
    st.accounting.modal_open = true;
}

fn is_dirty(st: &State) -> bool {
    st.forms.any_with_prefix("journal-") || !st.accounting.debit_account.is_empty() || !st.accounting.credit_account.is_empty()
}

pub fn close_or_confirm(st: &mut State) {
    if !is_dirty(st) {
        st.accounting.modal_open = false;
        return;
    }
    crate::ui::confirm::discard_form(st, |s| s.accounting.modal_open = false);
}

pub fn period_status_key(locked: bool) -> &'static str {
    if locked { "Locked" } else { "Open" }
}

/// Hint + empty-state title for the accounting tab.
pub fn tab_empty_copy(tab_idx: usize) -> (&'static str, &'static str) {
    match tab_idx {
        1 => ("The list of accounts your books use.", "No accounts"),
        2 => ("Your financial years.", "No fiscal periods"),
        _ => ("Every entry behind your numbers.", "No journals"),
    }
}

fn periods(s: &State) -> &[serde_json::Value] {
    s.books.workspace["periods"].as_array().map(|a| a.as_slice()).unwrap_or(&[])
}

fn journals(s: &State) -> Vec<serde_json::Value> {
    s.books.list("journals")
}

fn col_index(tab_idx: usize, col: &str) -> Option<usize> {
    match tab_idx {
        1 => match col {
            "code" => Some(0),
            "name" => Some(1),
            "class" => Some(2),
            "status" => Some(3),
            _ => None,
        },
        2 => match col {
            "name" => Some(0),
            "date" => Some(1),
            "status" => Some(2),
            _ => None,
        },
        _ => match col {
            "ref" => Some(0),
            "date" => Some(1),
            "desc" => Some(2),
            "debit" => Some(3),
            "credit" => Some(4),
            "status" => Some(5),
            _ => None,
        },
    }
}

fn build_rows(s: &State) -> Vec<Row> {
    match s.accounting.tab_idx {
        1 => accounts(s)
            .iter()
            .filter(|acc| s.accounting.acct_show_archived || !acc["archived"].as_bool().unwrap_or(false))
            .map(|acc| {
                let code = crate::data::s(&acc["code"]);
                let name = crate::data::s(&acc["name"]);
                let class = s.fmt.t(&crate::data::s(&acc["classification"]));
                let archived = if acc["archived"].as_bool().unwrap_or(false) {
                    s.fmt.t("Archived")
                } else {
                    String::new()
                };
                Row {
                    id: format!("acc-{}", crate::data::s(&acc["id"])),
                    cells: vec![code.clone(), name.clone(), class.clone(), archived.clone()],
                    keys: vec![
                        SortKey::text(&code),
                        SortKey::text(&name),
                        SortKey::text(&class),
                        SortKey::text(&archived),
                    ],
                }
            })
            .collect(),
        2 => periods(s)
            .iter()
            .map(|period| {
                let name = crate::data::s(&period["name"]);
                let date_raw = crate::data::s(&period["start_date"]);
                let locked = period["locked"].as_bool().unwrap_or(false);
                let status = s.fmt.t(period_status_key(locked));
                Row {
                    id: format!("period-{}", crate::data::s(&period["id"])),
                    cells: vec![name.clone(), s.fmt.date(&date_raw), status.clone()],
                    keys: vec![SortKey::text(&name), SortKey::text(&date_raw), SortKey::text(&status)],
                }
            })
            .collect(),
        _ => {
            let status = s.fmt.t("Posted");
            journals(s)
                .iter()
                .map(|j| {
                    let id = crate::data::s(&j["id"]);
                    let ref_text = format!("#{}", id.chars().take(8).collect::<String>());
                    let date_raw = crate::data::s(&j["date"]);
                    let desc = crate::data::s(&j["reason"]);
                    let debit_raw = crate::data::s(&j["total_debit"]);
                    let credit_raw = crate::data::s(&j["total_credit"]);
                    Row {
                        id: format!("journal-{id}"),
                        cells: vec![
                            ref_text.clone(),
                            s.fmt.date(&date_raw),
                            desc.clone(),
                            s.fmt.money(&debit_raw),
                            s.fmt.money(&credit_raw),
                            status.clone(),
                        ],
                        keys: vec![
                            SortKey::text(&ref_text),
                            SortKey::text(&date_raw),
                            SortKey::text(&desc),
                            SortKey::num_str(&debit_raw),
                            SortKey::num_str(&credit_raw),
                            SortKey::text(&status),
                        ],
                    }
                })
                .collect()
        }
    }
}

pub fn ensure(st: &mut State) {
    if st.accounting.rows_stamp == st.stamp {
        return;
    }
    st.accounting.rows = build_rows(st);
    st.accounting.rows_stamp = st.stamp;
    let tab_idx = st.accounting.tab_idx;
    reapply_sort(&st.accounting.table, &mut st.accounting.rows, |c| col_index(tab_idx, c));
}

pub fn apply_sort(st: &mut State, col: &str, dir: SortDir) {
    st.accounting.table.set_sort(col, dir);
    if let Some(idx) = col_index(st.accounting.tab_idx, col) {
        sort_rows(&mut st.accounting.rows, idx, dir);
    }
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let t0 = ui.t("Journals");
    let t1 = ui.t("Chart of accounts");
    let t2 = ui.t("Fiscal periods");
    let tabs = [
        (t0.as_str(), s.accounting.tab_idx == 0),
        (t1.as_str(), s.accounting.tab_idx == 1),
        (t2.as_str(), s.accounting.tab_idx == 2),
    ];
    let tab_bar = ui.tabs("accounting-tabs", &tabs, |st, idx| {
        st.accounting.tab_idx = idx;
        st.accounting.table = TableState::new();
        st.accounting.rows_stamp = 0;
        ensure(st);
    });

    let table = match s.accounting.tab_idx {
        1 => Table::new(
            |s: &State| &s.accounting.table,
            |s: &mut State| &mut s.accounting.table,
            |s: &State| s.accounting.rows.len(),
            |s: &State, i| Cow::Owned(s.accounting.rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
        )
        .name(ui.t("Chart of accounts"))
        .column(TableColumn::new("code", ui.t("Code")).width(100.0).min(72.0).sortable())
        .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
        .column(TableColumn::new("class", ui.t("Classification")).width(140.0).min(100.0).sortable())
        .column(TableColumn::new("status", ui.t("Status")).width(110.0).min(80.0).sortable())
        .cell(|s: &State, i, col| {
            let Some(row) = s.accounting.rows.get(i) else {
                return Cell::text("");
            };
            match col {
                "code" => Cell::text(&row.cells[0]),
                "name" => Cell::text(&row.cells[1]),
                "class" => Cell::text(&row.cells[2]).muted(),
                _ => Cell::text(&row.cells[3]).muted(),
            }
        })
        .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
        .on_select(|st, id, _| st.accounting.selected = id.to_string())
        .on_open(|st, id, _| open_ledger_for(st, id.strip_prefix("acc-").unwrap_or(id)))
        .zebra(true)
        .build(s, &ui.k)
        .key("coa-table")
        .grow(1.0)
        .min_h(0.0),
        2 => Table::new(
            |s: &State| &s.accounting.table,
            |s: &mut State| &mut s.accounting.table,
            |s: &State| s.accounting.rows.len(),
            |s: &State, i| Cow::Owned(s.accounting.rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
        )
        .name(ui.t("Fiscal periods"))
        .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
        .column(TableColumn::new("date", ui.t("Date")).width(120.0).min(96.0).sortable())
        .column(TableColumn::new("status", ui.t("Status")).width(120.0).min(88.0).sortable())
        .cell(|s: &State, i, col| {
            let Some(row) = s.accounting.rows.get(i) else {
                return Cell::text("");
            };
            match col {
                "name" => Cell::text(&row.cells[0]),
                "date" => Cell::text(&row.cells[1]).muted(),
                _ => Cell::text(&row.cells[2]).muted(),
            }
        })
        .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
        .on_select(|st, id, _| st.accounting.selected = id.to_string())
        .zebra(true)
        .build(s, &ui.k)
        .key("periods-table")
        .grow(1.0)
        .min_h(0.0),
        _ => Table::new(
            |s: &State| &s.accounting.table,
            |s: &mut State| &mut s.accounting.table,
            |s: &State| s.accounting.rows.len(),
            |s: &State, i| Cow::Owned(s.accounting.rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
        )
        .name(ui.t("Journals"))
        .column(TableColumn::new("ref", ui.t("Reference")).width(120.0).min(88.0).sortable())
        .column(TableColumn::new("date", ui.t("Date")).width(110.0).min(88.0).sortable())
        .column(TableColumn::new("desc", ui.t("Description")).flex(2.0).min(140.0).sortable())
        .column(
            TableColumn::new("debit", ui.t("Debit"))
                .width(140.0)
                .min(96.0)
                .align(Align::End)
                .sortable(),
        )
        .column(
            TableColumn::new("credit", ui.t("Credit"))
                .width(140.0)
                .min(96.0)
                .align(Align::End)
                .sortable(),
        )
        .column(TableColumn::new("status", ui.t("Status")).width(100.0).min(80.0).sortable())
        .cell(|s: &State, i, col| {
            let Some(row) = s.accounting.rows.get(i) else {
                return Cell::text("");
            };
            match col {
                "ref" => Cell::text(&row.cells[0]),
                "date" => Cell::text(&row.cells[1]).muted(),
                "desc" => Cell::text(&row.cells[2]),
                "debit" => Cell::text(&row.cells[3]).mono(),
                "credit" => Cell::text(&row.cells[4]).mono(),
                _ => Cell::text(&row.cells[5]).muted(),
            }
        })
        .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
        .on_select(|st, id, _| st.accounting.selected = id.to_string())
        .on_open(|st, id, _| crate::ui::detail::open(st, id.strip_prefix("journal-").unwrap_or(id)))
        .zebra(true)
        .build(s, &ui.k)
        .key("journals-table")
        .grow(1.0)
        .min_h(0.0),
    };

    let (hint, empty_title) = tab_empty_copy(s.accounting.tab_idx);
    let empty_count = s.accounting.rows.len();

    let hint_bar = if s.accounting.tab_idx == 1 {
        row().w_full().center_y().justify_between().child(ui.muted(ui.t(hint))).child(
            Switch::new(s.accounting.acct_show_archived)
                .label(ui.t("Show archived"))
                .on_toggle(|st: &mut State| {
                    st.accounting.acct_show_archived = !st.accounting.acct_show_archived;
                    st.accounting.rows_stamp = 0;
                    ensure(st);
                })
                .build(&ui.k)
                .key("acct-show-archived"),
        )
    } else {
        row().w_full().child(ui.muted(ui.t(hint)))
    };

    let mut body = col()
        .w_full()
        .gap(14.0)
        .grow(1.0)
        .min_h(0.0)
        .child(tab_bar)
        .child(hint_bar)
        .child(ui.fill_or_empty(empty_count, empty_title, hint, &icons::REPORTS, table));
    if s.accounting.tab_idx == 0
        && let Some(footer) = ui.load_more_footer(
            "journals-load-more",
            s.books.list("journals").len(),
            s.books.count("journal_count"),
            load_more_journals,
        )
    {
        body = body.child(footer);
    }

    if s.accounting.modal_open {
        mount_journal_dialog(s, ui, layer);
    }
    if s.accounting.acct_open {
        mount_account_dialog(s, ui, layer);
    }
    if s.accounting.acct_archive_open {
        mount_archive_dialog(s, ui, layer);
    }

    let period_locked = s
        .accounting
        .selected
        .strip_prefix("period-")
        .and_then(|id| {
            s.books.workspace["periods"]
                .as_array()
                .and_then(|rows| rows.iter().find(|p| crate::data::s(&p["id"]) == id))
        })
        .and_then(|p| p["locked"].as_bool())
        .unwrap_or(false);
    let period_btn_label = if period_locked {
        "Reopen fiscal period"
    } else {
        "Lock fiscal period"
    };

    let mut actions = vec![ui.btn(Btn::Primary, "New journal", Some(&icons::PLUS), open_journal)];
    if s.accounting.tab_idx == 1 {
        actions.push(ui.btn(Btn::Glass, "New account", Some(&icons::PLUS), open_account));
        actions.push(ui.btn_enabled(
            Btn::Glass,
            "Archive account",
            None,
            s.accounting.selected.starts_with("acc-"),
            open_archive_account,
        ));
    }
    actions.extend([
        ui.btn_enabled(Btn::Glass, "Open", None, s.accounting.selected.starts_with("journal-"), |st| {
            let id = st
                .accounting
                .selected
                .strip_prefix("journal-")
                .unwrap_or(&st.accounting.selected)
                .to_string();
            crate::ui::detail::open(st, &id);
        }),
        ui.btn_enabled(Btn::Glass, "Ledger", None, s.accounting.selected.starts_with("acc-"), open_ledger),
        ui.btn_enabled(
            Btn::Glass,
            "Reverse",
            None,
            s.accounting.selected.starts_with("journal-"),
            reverse_selected,
        ),
        ui.btn_enabled(
            Btn::Glass,
            period_btn_label,
            None,
            s.accounting.selected.starts_with("period-"),
            toggle_period,
        ),
        ui.btn(Btn::Glass, "New fiscal year", Some(&icons::PLUS), open_new_period),
    ]);

    ui.page(
        "accounting-screen",
        &ui.t("Accounting"),
        &ui.t("Ledger and accounting"),
        actions,
        body,
    )
}

fn mount_journal_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    use crate::ui::pickers;
    use serde_json::json;

    let mut portals = Vec::new();
    let accounts = pickers::account_options(s);
    let debit_open = s.selects.get("journal-debit").copied().unwrap_or(false);
    let credit_open = s.selects.get("journal-credit").copied().unwrap_or(false);
    let debit = pickers::select_field(
        ui,
        &mut portals,
        "journal-debit",
        &accounts,
        if s.accounting.debit_account.is_empty() {
            None
        } else {
            Some(s.accounting.debit_account.as_str())
        },
        &ui.t("Select account"),
        pickers::ACCOUNT_PICKER_WIDTH,
        debit_open,
        |st, id| st.accounting.debit_account = id.to_string(),
    );
    let credit = pickers::select_field(
        ui,
        &mut portals,
        "journal-credit",
        &accounts,
        if s.accounting.credit_account.is_empty() {
            None
        } else {
            Some(s.accounting.credit_account.as_str())
        },
        &ui.t("Select account"),
        pickers::ACCOUNT_PICKER_WIDTH,
        credit_open,
        |st, id| st.accounting.credit_account = id.to_string(),
    );
    let form = col()
        .gap(14.0)
        .child(ui.form_grid(ui.labeled("Debit", debit), ui.labeled("Credit", credit)))
        .child(ui.form_grid(
            ui.labeled("Amount", ui.money_field(s, "journal-amount")),
            ui.date_field(s, "journal-date", "Date"),
        ))
        .child(ui.labeled("Description", ui.field(s, "journal-reason", &ui.t("Note (optional)"))));

    ui.form_dialog(
        layer,
        &mut portals,
        "journal",
        &ui.t("New journal"),
        &ui.t("Post a balanced debit and credit."),
        true,
        DIALOG_LG,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close_or_confirm),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                if st.accounting.posting || !st.accounting.modal_open {
                    return;
                }
                st.accounting.posting = true;
                if st.accounting.debit_account.is_empty() || st.accounting.credit_account.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Select account"));
                    st.accounting.posting = false;
                    return;
                }
                let amt = match st.parse_positive_money(&st.forms.get("journal-amount")) {
                    Ok(v) => v,
                    Err(key) => {
                        st.toast(Tone::Danger, st.fmt.t(key));
                        st.accounting.posting = false;
                        return;
                    }
                };
                let Ok(date) = st.form_date("journal-date") else {
                    st.accounting.posting = false;
                    return;
                };
                let reason = {
                    let r = st.forms.get("journal-reason");
                    if r.trim().is_empty() { st.fmt.t("Manual journal") } else { r }
                };
                if st.accounting.post_key.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Reopen this form and try again."));
                    st.accounting.posting = false;
                    return;
                }
                let key = st.accounting.post_key.clone();
                let mut payload = json!({
                    "date": date,
                    "reason": reason,
                    "source_id": key,
                    "request_key": key,
                    "lines": [
                        { "account_id": st.accounting.debit_account, "debit": amt.to_string(), "credit": "0", "description": "" },
                        { "account_id": st.accounting.credit_account, "debit": "0", "credit": amt.to_string(), "description": "" }
                    ]
                });
                match st.query("journal_preview", payload.clone()) {
                    Ok(pr) => payload["expected_generation"] = pr["_generation"].clone(),
                    Err(e) => {
                        st.toast_err(e);
                        st.accounting.posting = false;
                        return;
                    }
                }
                match st.run("journal_post", payload, Some("Posted")) {
                    Ok(_) => {
                        st.accounting.modal_open = false;
                        st.accounting.posting = false;
                        st.accounting.post_key.clear();
                    }
                    Err(_) => {
                        // `run` already toasted the engine error.
                        st.accounting.posting = false;
                    }
                }
            }),
        ],
        close_or_confirm,
    );
}

fn load_more_journals(st: &mut State) {
    if let Err(e) = st.books.load_more(&st.engine, "journals") {
        st.toast_err(e);
        return;
    }
    st.stamp += 1;
    ensure(st);
}

pub(crate) fn open_ledger_for(st: &mut State, account_id: &str) {
    let account_id = account_id.trim();
    if account_id.is_empty() {
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    }
    let (from, to) = st.books.range();
    match st.query(
        "ledger",
        serde_json::json!({ "account_id": account_id, "from": from, "to": to, "offset": 0 }),
    ) {
        Ok(v) => {
            st.accounting.ledger_account_id = account_id.to_string();
            st.accounting.ledger_rows = v["rows"].as_array().cloned().unwrap_or_default();
            st.accounting.ledger_count = v["count"].as_u64().map(|n| n as usize).unwrap_or(st.accounting.ledger_rows.len());
            st.accounting.ledger_opening = crate::data::s(&v["opening_balance"]);
            st.accounting.ledger_selected.clear();
            st.accounting.ledger_table = TableState::new();
            st.accounting.ledger = v;
            st.accounting.ledger_open = true;
        }
        Err(e) => st.toast_err(e),
    }
}

fn load_more_ledger(st: &mut State) {
    if st.accounting.ledger_account_id.is_empty() {
        return;
    }
    let (from, to) = st.books.range();
    let offset = st.accounting.ledger_rows.len();
    match st.query(
        "ledger",
        serde_json::json!({
            "account_id": st.accounting.ledger_account_id,
            "from": from,
            "to": to,
            "offset": offset,
        }),
    ) {
        Ok(v) => {
            if let Some(rows) = v["rows"].as_array() {
                st.accounting.ledger_rows.extend(rows.iter().cloned());
            }
            if let Some(c) = v["count"].as_u64() {
                st.accounting.ledger_count = c as usize;
            }
        }
        Err(e) => st.toast_err(e),
    }
}

fn open_ledger(st: &mut State) {
    if !st.accounting.selected.starts_with("acc-") {
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    }
    let id = st
        .accounting
        .selected
        .strip_prefix("acc-")
        .unwrap_or(&st.accounting.selected)
        .to_string();
    open_ledger_for(st, &id);
}

fn reverse_selected(st: &mut State) {
    if !st.accounting.selected.starts_with("journal-") {
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    }
    let id = st
        .accounting
        .selected
        .strip_prefix("journal-")
        .unwrap_or(&st.accounting.selected)
        .to_string();
    let Some(journal) = st.books.list("journals").into_iter().find(|j| crate::data::s(&j["id"]) == id) else {
        st.toast(Tone::Info, st.fmt.t("No journals"));
        return;
    };
    let key = st.mint_key("rev");
    let date_raw = crate::data::s(&journal["date"]);
    let mut payload = serde_json::json!({
        "journal_id": id,
        "period_id": crate::data::s(&journal["period_id"]),
        "date": date_raw,
        "request_key": key,
        "source_id": key,
        "reason": st.fmt.t("Reverse entry")
    });
    match st.query("journal_reverse_preview", payload.clone()) {
        Ok(pr) => payload["expected_generation"] = pr["_generation"].clone(),
        Err(e) => {
            st.toast_err(e);
            return;
        }
    }
    let lines = vec![
        (st.fmt.t("Reference"), format!("#{}", id.chars().take(8).collect::<String>())),
        (st.fmt.t("Date"), st.fmt.date(&date_raw)),
        (st.fmt.t("Amount"), st.fmt.money(&crate::data::s(&journal["total_debit"]))),
    ];
    st.confirm = Some(crate::ui::confirm::Confirm::one(
        st.fmt.t("Reverse entry"),
        st.fmt.t("Creates an offsetting journal and preserves the original."),
        lines,
        "Reverse entry",
        st.fmt.t("Reverse entry"),
        true,
        move |st| {
            let _ = st.run("journal_reverse", payload.clone(), Some("Posted"));
        },
    ));
}

fn toggle_period(st: &mut State) {
    if !st.accounting.selected.starts_with("period-") {
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    }
    let id = st
        .accounting
        .selected
        .strip_prefix("period-")
        .unwrap_or(&st.accounting.selected)
        .to_string();
    let period = st.books.workspace["periods"]
        .as_array()
        .and_then(|rows| rows.iter().find(|p| crate::data::s(&p["id"]) == id))
        .cloned();
    let locked = period.as_ref().and_then(|p| p["locked"].as_bool()).unwrap_or(false);
    let period_name = period
        .as_ref()
        .map(|p| crate::data::s(&p["name"]))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| id.clone());
    let key = st.mint_key("lock");
    let action_key = if locked { "Reopen fiscal period" } else { "Lock fiscal period" };
    let message_key = if locked {
        "Entries can be posted again in this fiscal year once it is reopened."
    } else {
        "No entries can be posted in this fiscal year until it is reopened."
    };
    let confirm_key = if locked { "Reopen period" } else { "Lock period" };
    let lines = vec![(st.fmt.t("Fiscal period"), period_name)];
    st.confirm = Some(crate::ui::confirm::Confirm::one(
        st.fmt.t(action_key),
        st.fmt.t(message_key),
        lines,
        confirm_key,
        st.fmt.t(action_key),
        locked,
        move |st| {
            let _ = st.run(
                "period_lock",
                serde_json::json!({
                    "period_id": id,
                    "locked": !locked,
                    "reason": action_key,
                    "request_key": key,
                    "source_id": key
                }),
                Some("Posted"),
            );
        },
    ));
}

pub(crate) fn mount_ledger(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    if !s.accounting.ledger_open {
        return;
    }
    let mut body = col().w_full().gap(10.0).child(
        row()
            .key("ledger-opening-row")
            .w_full()
            .center_y()
            .justify_between()
            .child(ui.muted(ui.t("Opening balance")))
            .child(ui.muted(ui.f.money(&s.accounting.ledger_opening))),
    );

    if s.accounting.ledger_rows.is_empty() {
        body = body.child(ui.empty(&icons::REPORTS, "No entries in this period", "Ledger"));
    } else {
        let table = Table::new(
            |s: &State| &s.accounting.ledger_table,
            |s: &mut State| &mut s.accounting.ledger_table,
            |s: &State| s.accounting.ledger_rows.len(),
            |s: &State, i| {
                let jid = s
                    .accounting
                    .ledger_rows
                    .get(i)
                    .map(|r| crate::data::s(&r["journal_id"]))
                    .unwrap_or_default();
                Cow::Owned(format!("{i}:{jid}"))
            },
        )
        .name(ui.t("Ledger"))
        .column(TableColumn::new("date", ui.t("Date")).width(105.0).min(88.0))
        .column(TableColumn::new("ref", ui.t("Reference")).width(110.0).min(80.0))
        .column(TableColumn::new("desc", ui.t("Description")).flex(2.0).min(120.0))
        .column(TableColumn::new("debit", ui.t("Debit")).width(120.0).min(88.0).align(Align::End))
        .column(TableColumn::new("credit", ui.t("Credit")).width(120.0).min(88.0).align(Align::End))
        .column(
            TableColumn::new("balance", ui.t("Balance"))
                .width(130.0)
                .min(96.0)
                .align(Align::End),
        )
        .cell(|s: &State, i, col| {
            let Some(r) = s.accounting.ledger_rows.get(i) else {
                return Cell::text("");
            };
            match col {
                "date" => Cell::text(s.fmt.date(&crate::data::s(&r["date"]))).muted(),
                "ref" => Cell::text(crate::data::s(&r["reference"])),
                "desc" => Cell::text(crate::data::s(&r["description"])),
                "debit" => Cell::text(s.fmt.money(&crate::data::s(&r["debit"]))).mono(),
                "credit" => Cell::text(s.fmt.money(&crate::data::s(&r["credit"]))).mono(),
                _ => Cell::text(s.fmt.money(&crate::data::s(&r["balance"]))).mono(),
            }
        })
        .on_select(|st, id, _| {
            st.accounting.ledger_selected = id.split_once(':').map(|(_, jid)| jid.to_string()).unwrap_or_default();
        })
        .on_open(|st, id, _| {
            if let Some((_, jid)) = id.split_once(':') {
                crate::ui::detail::open(st, jid);
            }
        })
        .zebra(true)
        .build(s, &ui.k)
        .key("ledger-table")
        .h(300.0);

        body = body.child(table);
        if let Some(footer) = ui.load_more_footer(
            "ledger-load-more",
            s.accounting.ledger_rows.len(),
            s.accounting.ledger_count,
            load_more_ledger,
        ) {
            body = body.child(footer);
        }
    }

    let mut portals = Vec::new();
    ui.dialog_described(
        layer,
        &mut portals,
        "ledger",
        &ui.t("Ledger"),
        &ui.t("Every entry behind your numbers."),
        true,
        DIALOG_LG,
        body,
        vec![
            ui.btn_enabled(Btn::Glass, "Open", None, !s.accounting.ledger_selected.is_empty(), |st| {
                let jid = st.accounting.ledger_selected.clone();
                crate::ui::detail::open(st, &jid);
            }),
            ui.btn(Btn::Ghost, "Close", None, |st| st.accounting.ledger_open = false),
        ],
        |st| st.accounting.ledger_open = false,
    );
}

pub(crate) fn open_new_period(st: &mut State) {
    let latest_end = st.books.workspace["periods"]
        .as_array()
        .and_then(|rows| rows.iter().filter_map(|p| p["end_date"].as_str()).filter(|d| !d.is_empty()).max())
        .map(str::to_string);
    let today_date =
        chrono::NaiveDate::parse_from_str(&crate::data::today(), "%Y-%m-%d").unwrap_or_else(|_| chrono::Local::now().date_naive());
    let (name, start_iso, end_iso) = crate::ui::views::setup::next_fiscal_period(st.fmt.calendar, latest_end.as_deref(), today_date);
    st.forms.set("period-name", &name);
    st.accounting.next_period_start = start_iso;
    st.accounting.next_period_end = end_iso;
    st.accounting.period_add_busy = false;
    st.accounting.period_add_open = true;
}

pub(crate) fn mount_period_add_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    if !s.accounting.period_add_open {
        return;
    }
    let mut portals = Vec::new();
    let form = col()
        .gap(14.0)
        .child(ui.labeled("Name", ui.field(s, "period-name", &ui.t("Fiscal year"))))
        .child(ui.form_grid(
            ui.labeled("Start date", ui.body(s.fmt.date(&s.accounting.next_period_start))),
            ui.labeled("End date", ui.body(s.fmt.date(&s.accounting.next_period_end))),
        ));

    ui.form_dialog(
        layer,
        &mut portals,
        "period-add",
        &ui.t("New fiscal year"),
        &ui.t("Open the next fiscal year starting the day after the current one ends."),
        true,
        DIALOG_SM,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.accounting.period_add_open = false),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                if st.accounting.period_add_busy || !st.accounting.period_add_open {
                    return;
                }
                let name = st.forms.get("period-name").trim().to_string();
                if name.is_empty() {
                    st.toast(Tone::Warning, st.fmt.t("Name"));
                    return;
                }
                st.accounting.period_add_busy = true;
                let generation = crate::data::s(&st.books.workspace["_generation"]);
                let payload = serde_json::json!({
                    "name": name,
                    "start_date": st.accounting.next_period_start,
                    "end_date": st.accounting.next_period_end,
                    "expected_generation": generation,
                });
                match st.run("period_add", payload, Some("Fiscal year opened")) {
                    Ok(v) => {
                        if let Some(id) = v["period_id"].as_str() {
                            st.books.period_id = id.to_string();
                        }
                        st.accounting.period_add_open = false;
                        st.accounting.period_add_busy = false;
                        st.refresh();
                    }
                    Err(_) => {
                        st.accounting.period_add_busy = false;
                    }
                }
            }),
        ],
        |st| st.accounting.period_add_open = false,
    );
}

fn suggested_account_code(s: &State, class_name: &str, parent_id: &str) -> String {
    let max = accounts(s)
        .iter()
        .filter(|a| {
            if parent_id.is_empty() {
                crate::data::s(&a["classification"]) == class_name
            } else {
                crate::data::s(&a["parent_id"]) == parent_id
            }
        })
        .filter_map(|a| crate::data::s(&a["code"]).trim().parse::<u64>().ok())
        .max();
    max.map(|n| (n + 1).to_string()).unwrap_or_default()
}

fn mount_account_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    use crate::ui::pickers;

    let mut portals = Vec::new();
    let class_idx = s.accounting.acct_class_idx.min(ACCOUNT_CLASSES.len() - 1);
    let class_name = ACCOUNT_CLASSES[class_idx].0;
    let labels: Vec<String> = ACCOUNT_CLASSES.iter().map(|(c, _)| ui.t(c)).collect();
    let tabs: Vec<(&str, bool)> = labels.iter().enumerate().map(|(i, l)| (l.as_str(), i == class_idx)).collect();
    let class_tabs = ui.tabs("acct-class", &tabs, |st, idx| {
        st.accounting.acct_class_idx = idx;
        st.accounting.acct_parent.clear();
    });

    let mut parent_opts = vec![(String::new(), ui.t("Optional"))];
    for acc in accounts(s) {
        if !acc["archived"].as_bool().unwrap_or(false) && crate::data::s(&acc["classification"]) == class_name {
            let id = crate::data::s(&acc["id"]);
            let code = crate::data::s(&acc["code"]);
            let name = crate::data::s(&acc["name"]);
            parent_opts.push((id, format!("{code} · {name}")));
        }
    }
    let parent_open = s.selects.get("acct-parent").copied().unwrap_or(false);
    let current_parent = if s.accounting.acct_parent.is_empty() {
        None
    } else {
        Some(s.accounting.acct_parent.as_str())
    };
    let parent_picker = pickers::select_field(
        ui,
        &mut portals,
        "acct-parent",
        &parent_opts,
        current_parent,
        &ui.t("Optional"),
        pickers::ACCOUNT_PICKER_WIDTH,
        parent_open,
        |st, id| st.accounting.acct_parent = id.to_string(),
    );

    let code_placeholder = suggested_account_code(s, class_name, &s.accounting.acct_parent);
    let form = col()
        .gap(14.0)
        .child(ui.labeled("Type", class_tabs))
        .child(ui.labeled("Parent account", parent_picker))
        .child(ui.form_grid(
            ui.labeled("Account code", ui.field(s, "acct-code", &code_placeholder)),
            ui.labeled("Name", ui.field(s, "acct-name", &ui.t("Name"))),
        ));

    ui.form_dialog(
        layer,
        &mut portals,
        "acct",
        &ui.t("New account"),
        &ui.t("Add an account to your chart of accounts."),
        true,
        DIALOG_MD,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close_acct_or_confirm),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                if st.accounting.acct_busy || !st.accounting.acct_open {
                    return;
                }
                let code = st.forms.get("acct-code").trim().to_string();
                if code.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Account code"));
                    return;
                }
                let name = st.forms.get("acct-name").trim().to_string();
                if name.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Name"));
                    return;
                }
                st.accounting.acct_busy = true;
                let idx = st.accounting.acct_class_idx.min(ACCOUNT_CLASSES.len() - 1);
                let (classification, normal_balance) = ACCOUNT_CLASSES[idx];
                let generation = if st.accounting.acct_generation.is_empty() {
                    crate::data::s(&st.books.workspace["_generation"])
                } else {
                    st.accounting.acct_generation.clone()
                };
                let mut payload = serde_json::json!({
                    "code": code,
                    "name": name,
                    "classification": classification,
                    "normal_balance": normal_balance,
                    "postable": true,
                    "control": false,
                    "expected_generation": generation,
                });
                if !st.accounting.acct_parent.is_empty() {
                    payload["parent_id"] = serde_json::json!(st.accounting.acct_parent);
                }
                match st.run("account_add", payload, Some("Account added")) {
                    Ok(_) => {
                        st.accounting.acct_open = false;
                        st.accounting.acct_busy = false;
                        st.forms.clear_prefix("acct-");
                        st.accounting.acct_parent.clear();
                    }
                    Err(_) => {
                        st.accounting.acct_busy = false;
                    }
                }
            }),
        ],
        close_acct_or_confirm,
    );
}

fn open_archive_account(st: &mut State) {
    if !st.accounting.selected.starts_with("acc-") {
        st.toast(Tone::Info, st.fmt.t("Select a row first."));
        return;
    }
    st.forms.set("acct-archive-reason", "");
    st.accounting.acct_archive_busy = false;
    st.accounting.acct_archive_open = true;
}

fn mount_archive_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let mut portals = Vec::new();
    let form = col()
        .gap(14.0)
        .child(ui.labeled("Reason", ui.field(s, "acct-archive-reason", &ui.t("Reason"))));

    ui.form_dialog(
        layer,
        &mut portals,
        "acct-archive",
        &ui.t("Archive account"),
        &ui.t("Archived accounts stay in historical reports and leave pickers."),
        true,
        DIALOG_SM,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.accounting.acct_archive_open = false),
            ui.btn(Btn::Danger, "Archive", None, |st| {
                if st.accounting.acct_archive_busy || !st.accounting.acct_archive_open {
                    return;
                }
                let reason = st.forms.get("acct-archive-reason").trim().to_string();
                if reason.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Reason"));
                    return;
                }
                let account_id = st
                    .accounting
                    .selected
                    .strip_prefix("acc-")
                    .unwrap_or(&st.accounting.selected)
                    .to_string();
                st.accounting.acct_archive_busy = true;
                let generation = crate::data::s(&st.books.workspace["_generation"]);
                let payload = serde_json::json!({
                    "account_id": account_id,
                    "reason": reason,
                    "expected_generation": generation,
                });
                match st.run("account_archive", payload, Some("Saved")) {
                    Ok(_) => {
                        st.accounting.acct_archive_open = false;
                        st.accounting.acct_archive_busy = false;
                        st.forms.set("acct-archive-reason", "");
                    }
                    Err(_) => {
                        st.accounting.acct_archive_busy = false;
                    }
                }
            }),
        ],
        |st| st.accounting.acct_archive_open = false,
    );
}
