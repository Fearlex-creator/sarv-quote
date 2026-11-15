//! Reports — P&L, trial balance and party balances with kit Table + tabs.

use std::borrow::Cow;

use rune::{Align, Cx, El, col, row};
use rune_kit::{Cell, SortDir, Table, TableColumn, TableState};

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;
use crate::ui::rows::{Row, SortKey, reapply_sort, sort_rows};

#[derive(Default)]
pub struct ReportsState {
    pub tab_idx: usize,
    pub table: TableState,
    pub selected_account_id: String,
    pub rows: Vec<Row>,
    pub rows_stamp: u64,
}

fn trial_rows(s: &State) -> Vec<serde_json::Value> {
    s.books.report["trial"]["rows"].as_array().cloned().unwrap_or_default()
}

fn party_keys(s: &State) -> Vec<(String, String)> {
    s.books.report["party_balances"]
        .as_object()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), crate::data::s(v))).collect())
        .unwrap_or_default()
}

fn pl_rows(s: &State) -> Vec<serde_json::Value> {
    s.books.report["profit_loss"]["rows"].as_array().cloned().unwrap_or_default()
}

fn col_index(tab_idx: usize, col: &str) -> Option<usize> {
    match tab_idx {
        1 => match col {
            "code" => Some(0),
            "name" => Some(1),
            "debit" => Some(2),
            "credit" => Some(3),
            _ => None,
        },
        2 => match col {
            "name" => Some(0),
            "role" => Some(1),
            "balance" => Some(2),
            _ => None,
        },
        3 => match col {
            "class" => Some(0),
            "code" => Some(1),
            "name" => Some(2),
            "amount" => Some(3),
            _ => None,
        },
        4 => match col {
            "name" => Some(0),
            "side" => Some(1),
            "total" => Some(2),
            _ => None,
        },
        _ => match col {
            "code" => Some(0),
            "name" => Some(1),
            "class" => Some(2),
            "amount" => Some(3),
            _ => None,
        },
    }
}

fn build_rows(s: &State) -> Vec<Row> {
    match s.reports.tab_idx {
        1 => trial_rows(s)
            .iter()
            .map(|r| {
                let code = crate::data::s(&r["code"]);
                let name = crate::data::s(&r["name"]);
                let debit_raw = crate::data::s(&r["debit"]);
                let credit_raw = crate::data::s(&r["credit"]);
                Row {
                    id: format!("tb-{}", crate::data::s(&r["account_id"])),
                    cells: vec![code.clone(), name.clone(), s.fmt.money(&debit_raw), s.fmt.money(&credit_raw)],
                    keys: vec![
                        SortKey::text(&code),
                        SortKey::text(&name),
                        SortKey::num_str(&debit_raw),
                        SortKey::num_str(&credit_raw),
                    ],
                }
            })
            .collect(),
        2 => party_keys(s)
            .iter()
            .map(|(key, amount_raw)| {
                let (role_raw, id) = key.split_once(':').unwrap_or(("contact", key.as_str()));
                let p_name = s.books.party_name(id);
                let name = if p_name.is_empty() { id.to_string() } else { p_name };
                let role = if role_raw == "customer" {
                    s.fmt.t("Customer")
                } else {
                    s.fmt.t("Supplier")
                };
                Row {
                    id: format!("party-bal-{key}"),
                    cells: vec![name.clone(), role.clone(), s.fmt.money(amount_raw)],
                    keys: vec![SortKey::text(&name), SortKey::text(&role), SortKey::num_str(amount_raw)],
                }
            })
            .collect(),
        3 => sheet_rows(s)
            .iter()
            .map(|(class_raw, row)| {
                let key = match class_raw.as_str() {
                    "assets" => "Asset",
                    "liabilities" => "Liability",
                    "equity" => "Equity",
                    other => other,
                };
                let class = s.fmt.t(key);
                let code = crate::data::s(&row["code"]);
                let name = crate::data::s(&row["name"]);
                let amount_raw = crate::data::s(&row["amount"]);
                Row {
                    id: format!("bs-{}", crate::data::s(&row["account_id"])),
                    cells: vec![class.clone(), code.clone(), name.clone(), s.fmt.money(&amount_raw)],
                    keys: vec![
                        SortKey::text(&class),
                        SortKey::text(&code),
                        SortKey::text(&name),
                        SortKey::num_str(&amount_raw),
                    ],
                }
            })
            .collect(),
        4 => aging_rows(s)
            .iter()
            .map(|row| {
                let id = crate::data::s(&row["party_id"]);
                let side_raw = crate::data::s(&row["side"]);
                let name = s.books.party_name(&id);
                let side = s.fmt.t(if side_raw == "payable" { "Supplier" } else { "Customer" });
                let total_raw = crate::data::s(&row["total"]);
                Row {
                    id: format!("age-{side_raw}-{id}"),
                    cells: vec![name.clone(), side.clone(), s.fmt.money(&total_raw)],
                    keys: vec![SortKey::text(&name), SortKey::text(&side), SortKey::num_str(&total_raw)],
                }
            })
            .collect(),
        _ => pl_rows(s)
            .iter()
            .map(|r| {
                let code = crate::data::s(&r["code"]);
                let name = crate::data::s(&r["name"]);
                let class_raw = crate::data::s(&r["classification"]);
                let class = s.fmt.t(&class_raw);
                let amount_raw = crate::data::s(&r["amount"]);
                Row {
                    id: format!("pl-{}", crate::data::s(&r["account_id"])),
                    cells: vec![code.clone(), name.clone(), class.clone(), s.fmt.money(&amount_raw)],
                    keys: vec![
                        SortKey::text(&code),
                        SortKey::text(&name),
                        SortKey::text(&class),
                        SortKey::num_str(&amount_raw),
                    ],
                }
            })
            .collect(),
    }
}

pub fn ensure(st: &mut State) {
    if st.reports.rows_stamp == st.stamp {
        return;
    }
    st.reports.rows = build_rows(st);
    st.reports.rows_stamp = st.stamp;
    let tab_idx = st.reports.tab_idx;
    reapply_sort(&st.reports.table, &mut st.reports.rows, |c| col_index(tab_idx, c));
}

pub fn apply_sort(st: &mut State, col: &str, dir: SortDir) {
    st.reports.table.set_sort(col, dir);
    if let Some(idx) = col_index(st.reports.tab_idx, col) {
        sort_rows(&mut st.reports.rows, idx, dir);
    }
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, _layer: &mut Vec<El>) -> El {
    let report = &s.books.report;

    let t0 = ui.t("Profit & Loss");
    let t1 = ui.t("Trial balance");
    let t2 = ui.t("Customers & Suppliers");
    let t3 = ui.t("Balance sheet");
    let t4 = ui.t("Aging");
    let tabs = [
        (t0.as_str(), s.reports.tab_idx == 0),
        (t1.as_str(), s.reports.tab_idx == 1),
        (t2.as_str(), s.reports.tab_idx == 2),
        (t3.as_str(), s.reports.tab_idx == 3),
        (t4.as_str(), s.reports.tab_idx == 4),
    ];
    let tab_bar = ui.tabs("reports-tabs", &tabs, |st, idx| {
        st.reports.tab_idx = idx;
        st.reports.table = TableState::new();
        st.reports.selected_account_id.clear();
        st.reports.rows_stamp = 0;
        ensure(st);
    });

    let body = match s.reports.tab_idx {
        1 => {
            let hint = "Every account's balance — debits equal credits.";
            let table = Table::new(
                |s: &State| &s.reports.table,
                |s: &mut State| &mut s.reports.table,
                |s: &State| s.reports.rows.len(),
                |s: &State, i| Cow::Owned(s.reports.rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
            )
            .name(ui.t("Trial balance"))
            .column(TableColumn::new("code", ui.t("Code")).width(100.0).min(72.0).sortable())
            .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
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
            .cell(|s: &State, i, col| {
                let Some(row) = s.reports.rows.get(i) else {
                    return Cell::text("");
                };
                match col {
                    "code" => Cell::text(&row.cells[0]),
                    "name" => Cell::text(&row.cells[1]),
                    "debit" => Cell::text(&row.cells[2]).mono(),
                    _ => Cell::text(&row.cells[3]).mono(),
                }
            })
            .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
            .on_select(|st, id, _| {
                st.reports.selected_account_id = id.strip_prefix("tb-").unwrap_or("").to_string();
            })
            .on_open(|st, id, _| {
                if let Some(aid) = id.strip_prefix("tb-") {
                    crate::ui::views::accounting::open_ledger_for(st, aid);
                }
            })
            .zebra(true)
            .build(s, &ui.k)
            .key("trial-table")
            .grow(1.0)
            .min_h(0.0);
            col()
                .w_full()
                .gap(14.0)
                .grow(1.0)
                .min_h(0.0)
                .child(tab_bar)
                .child(ui.muted(ui.t(hint)))
                .child(ui.fill_or_empty(s.reports.rows.len(), "Nothing to show", hint, &icons::REPORTS, table))
        }
        2 => {
            let hint = "Who owes you and who you owe.";
            let table = Table::new(
                |s: &State| &s.reports.table,
                |s: &mut State| &mut s.reports.table,
                |s: &State| s.reports.rows.len(),
                |s: &State, i| Cow::Owned(s.reports.rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
            )
            .name(ui.t("Customers & Suppliers"))
            .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
            .column(TableColumn::new("role", ui.t("Role")).width(120.0).min(88.0).sortable())
            .column(
                TableColumn::new("balance", ui.t("Balance"))
                    .width(160.0)
                    .min(110.0)
                    .align(Align::End)
                    .sortable(),
            )
            .cell(|s: &State, i, col| {
                let Some(row) = s.reports.rows.get(i) else {
                    return Cell::text("");
                };
                match col {
                    "name" => Cell::text(&row.cells[0]),
                    "role" => Cell::text(&row.cells[1]).muted(),
                    _ => Cell::text(&row.cells[2]).mono(),
                }
            })
            .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
            .zebra(true)
            .build(s, &ui.k)
            .key("party-balances-table")
            .grow(1.0)
            .min_h(0.0);
            col()
                .w_full()
                .gap(14.0)
                .grow(1.0)
                .min_h(0.0)
                .child(tab_bar)
                .child(ui.muted(ui.t(hint)))
                .child(ui.fill_or_empty(s.reports.rows.len(), "Nothing to show", hint, &icons::REPORTS, table))
        }
        3 => sheet_tab(s, ui, tab_bar),
        4 => aging_tab(s, ui, tab_bar),
        _ => {
            let revenue = crate::data::s(&report["profit_loss"]["revenue"]);
            let expenses = crate::data::s(&report["profit_loss"]["expenses"]);
            let profit = crate::data::s(&report["profit_loss"]["profit"]);
            let hint = "How much you earned and spent in this period.";

            let metrics = row()
                .w_full()
                .gap(12.0)
                .child(ui.metric(
                    "pl-rev",
                    &icons::SALES,
                    &ui.t("Revenue"),
                    ui.f.money(&revenue),
                    ui.t("Posted entries"),
                    Tone::Success,
                ))
                .child(ui.metric(
                    "pl-exp",
                    &icons::PURCHASES,
                    &ui.t("Expenses"),
                    ui.f.money(&expenses),
                    ui.t("Posted entries"),
                    Tone::Warning,
                ))
                .child(ui.metric(
                    "pl-net",
                    &icons::REPORTS,
                    &ui.t("Net profit"),
                    ui.f.money(&profit),
                    ui.t("Profit this period"),
                    Tone::Info,
                ));

            let table = Table::new(
                |s: &State| &s.reports.table,
                |s: &mut State| &mut s.reports.table,
                |s: &State| s.reports.rows.len(),
                |s: &State, i| Cow::Owned(s.reports.rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
            )
            .name(ui.t("Profit & Loss"))
            .column(TableColumn::new("code", ui.t("Code")).width(100.0).min(72.0).sortable())
            .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
            .column(TableColumn::new("class", ui.t("Classification")).width(120.0).min(96.0).sortable())
            .column(
                TableColumn::new("amount", ui.t("Amount"))
                    .width(160.0)
                    .min(100.0)
                    .align(Align::End)
                    .sortable(),
            )
            .cell(|s: &State, i, col| {
                let Some(row) = s.reports.rows.get(i) else {
                    return Cell::text("");
                };
                match col {
                    "code" => Cell::text(&row.cells[0]),
                    "name" => Cell::text(&row.cells[1]),
                    "class" => Cell::text(&row.cells[2]).muted(),
                    _ => Cell::text(&row.cells[3]).mono(),
                }
            })
            .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
            .zebra(true)
            .build(s, &ui.k)
            .key("pl-table")
            .grow(1.0)
            .min_h(0.0);

            col()
                .w_full()
                .gap(14.0)
                .grow(1.0)
                .min_h(0.0)
                .child(tab_bar)
                .child(ui.muted(ui.t(hint)))
                .child(metrics)
                .child(ui.fill_or_empty(s.reports.rows.len(), "Nothing to show", hint, &icons::REPORTS, table))
        }
    };

    ui.page(
        "reports-screen",
        &ui.t("Reports"),
        &ui.t("Reports for this fiscal period"),
        vec![
            ui.btn_enabled(
                Btn::Glass,
                "Ledger",
                None,
                !s.reports.selected_account_id.is_empty() && matches!(s.reports.tab_idx, 1 | 3),
                |st| {
                    let aid = st.reports.selected_account_id.clone();
                    crate::ui::views::accounting::open_ledger_for(st, &aid);
                },
            ),
            ui.btn(Btn::Glass, "Export CSV", Some(&icons::DOWNLOAD), export_csv),
        ],
        body,
    )
}

fn sheet_rows(s: &State) -> Vec<(String, serde_json::Value)> {
    let mut rows = Vec::new();
    for key in ["assets", "liabilities", "equity"] {
        if let Some(arr) = s.books.report["balance_sheet"][key].as_array() {
            for row in arr {
                rows.push((key.to_string(), row.clone()));
            }
        }
    }
    rows
}

fn sheet_tab(s: &State, ui: &Ui, tab_bar: El) -> El {
    if s.reports.rows.is_empty() {
        return col().w_full().gap(14.0).grow(1.0).min_h(0.0).child(tab_bar).child(
            ui.empty(&icons::REPORTS, "No balances through this date", "Balance sheet")
                .grow(1.0),
        );
    }
    let table = Table::new(
        |s: &State| &s.reports.table,
        |s: &mut State| &mut s.reports.table,
        |s: &State| s.reports.rows.len(),
        |s: &State, i| Cow::Owned(s.reports.rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
    )
    .name(ui.t("Balance sheet"))
    .column(TableColumn::new("class", ui.t("Classification")).width(120.0).min(88.0).sortable())
    .column(TableColumn::new("code", ui.t("Code")).width(100.0).min(72.0).sortable())
    .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
    .column(
        TableColumn::new("amount", ui.t("Amount"))
            .width(160.0)
            .min(110.0)
            .align(Align::End)
            .sortable(),
    )
    .cell(|s: &State, i, col| {
        let Some(row) = s.reports.rows.get(i) else {
            return Cell::text("");
        };
        match col {
            "class" => Cell::text(&row.cells[0]),
            "code" => Cell::text(&row.cells[1]),
            "name" => Cell::text(&row.cells[2]),
            _ => Cell::text(&row.cells[3]).mono(),
        }
    })
    .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
    .on_select(|st, id, _| {
        st.reports.selected_account_id = id.strip_prefix("bs-").unwrap_or("").to_string();
    })
    .on_open(|st, id, _| {
        if let Some(aid) = id.strip_prefix("bs-") {
            crate::ui::views::accounting::open_ledger_for(st, aid);
        }
    })
    .zebra(true)
    .build(s, &ui.k)
    .key("balance-sheet-table")
    .grow(1.0)
    .min_h(0.0);
    let balanced = s.books.report["balance_sheet"]["balanced"].as_bool().unwrap_or(false);
    let status = if balanced {
        ui.muted(ui.t("Balanced"))
    } else {
        ui.badge(Tone::Danger, ui.t("Out of balance"))
    };
    col()
        .w_full()
        .gap(14.0)
        .grow(1.0)
        .min_h(0.0)
        .child(tab_bar)
        .child(status)
        .child(table)
}

fn aging_rows(s: &State) -> Vec<serde_json::Value> {
    let mut rows = Vec::new();
    for side in ["receivable", "payable"] {
        if let Some(arr) = s.books.report["aging"][side]["rows"].as_array() {
            for row in arr {
                let mut row = row.clone();
                row["side"] = serde_json::json!(side);
                rows.push(row);
            }
        }
    }
    rows
}

fn aging_tab(s: &State, ui: &Ui, tab_bar: El) -> El {
    if s.reports.rows.is_empty() {
        return col().w_full().gap(14.0).grow(1.0).min_h(0.0).child(tab_bar).child(
            ui.empty(&icons::REPORTS, "No outstanding balances — everything is settled.", "Aging")
                .grow(1.0),
        );
    }
    let table = Table::new(
        |s: &State| &s.reports.table,
        |s: &mut State| &mut s.reports.table,
        |s: &State| s.reports.rows.len(),
        |s: &State, i| Cow::Owned(s.reports.rows.get(i).map(|r| r.id.clone()).unwrap_or_default()),
    )
    .name(ui.t("Aging"))
    .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
    .column(TableColumn::new("side", ui.t("Role")).width(120.0).min(88.0).sortable())
    .column(
        TableColumn::new("total", ui.t("Balance"))
            .width(160.0)
            .min(110.0)
            .align(Align::End)
            .sortable(),
    )
    .cell(|s: &State, i, col| {
        let Some(row) = s.reports.rows.get(i) else {
            return Cell::text("");
        };
        match col {
            "name" => Cell::text(&row.cells[0]),
            "side" => Cell::text(&row.cells[1]).muted(),
            _ => Cell::text(&row.cells[2]).mono(),
        }
    })
    .on_sort(|st, col, dir, _tx| apply_sort(st, col, dir))
    .zebra(true)
    .build(s, &ui.k)
    .key("aging-table")
    .grow(1.0)
    .min_h(0.0);
    col().w_full().gap(14.0).grow(1.0).min_h(0.0).child(tab_bar).child(table)
}

fn export_action_csv(st: &mut State, action: &str, from: &str, to: &str) -> bool {
    match st.query(action, serde_json::json!({ "from": from, "to": to })) {
        Ok(v) => {
            let name = crate::data::s(&v["filename"]);
            let content = v["content"].as_str().unwrap_or("");
            if name.is_empty() || content.is_empty() {
                st.toast(Tone::Danger, st.fmt.t("Report unavailable"));
                return false;
            }
            match crate::ui::documents::save_text_export(st, &name, content) {
                Ok(path) => {
                    st.toast_file(path);
                    true
                }
                Err(e) => {
                    st.toast(Tone::Danger, e);
                    false
                }
            }
        }
        Err(e) => {
            st.toast_err(e);
            false
        }
    }
}

fn csv_cell_safe(raw: &str) -> String {
    let safe = if raw
        .trim_start_matches(|c: char| c.is_whitespace() || c.is_control() || c == '\u{feff}')
        .starts_with(['=', '+', '-', '@'])
    {
        format!("'{raw}")
    } else {
        raw.to_string()
    };
    if safe.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", safe.replace('"', "\"\""))
    } else {
        safe
    }
}

fn export_csv(st: &mut State) {
    let (from, to) = st.books.range();
    match st.reports.tab_idx {
        0 => {
            export_action_csv(st, "profit_csv", &from, &to);
        }
        1 => {
            if export_action_csv(st, "report_csv", &from, &to) {
                match crate::ui::documents::write_trial_balance_pdf(st, &from, &to) {
                    Ok(path) => st.toast_file(path),
                    Err(e) => st.toast(Tone::Danger, e),
                }
            }
        }
        2 => {
            let mut out = String::from("\u{feff}Party,Role,Balance IRR\n");
            for (key, amount) in party_keys(st) {
                let (role, id) = key.split_once(':').unwrap_or(("contact", key.as_str()));
                let name = st.books.party_name(id);
                let label = if name.is_empty() { id.to_string() } else { name };
                out.push_str(&format!(
                    "{},{},{}\n",
                    csv_cell_safe(&label),
                    csv_cell_safe(role),
                    csv_cell_safe(&amount)
                ));
            }
            match crate::ui::documents::save_text_export(st, &format!("parties-{to}.csv"), &out) {
                Ok(path) => st.toast_file(path),
                Err(e) => st.toast(Tone::Danger, e),
            }
        }
        3 => {
            export_action_csv(st, "balance_sheet_csv", &from, &to);
        }
        4 => {
            export_action_csv(st, "aging_csv", &from, &to);
        }
        _ => st.toast(Tone::Danger, st.fmt.t("Report unavailable")),
    }
}
