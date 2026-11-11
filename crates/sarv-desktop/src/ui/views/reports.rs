//! Reports — P&L, trial balance and party balances with kit Table + tabs.

use std::borrow::Cow;

use rune::{Align, Cx, El, col, row};
use rune_kit::{Cell, Table, TableColumn, TableState};

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;

#[derive(Default)]
pub struct ReportsState {
    pub tab_idx: usize,
    pub table: TableState,
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
    });

    let body = match s.reports.tab_idx {
        1 => {
            let hint = "Every account's balance — debits equal credits.";
            let table = Table::new(
                |s: &State| &s.reports.table,
                |s: &mut State| &mut s.reports.table,
                |s: &State| trial_rows(s).len(),
                |s: &State, i| Cow::Owned(format!("tb-{}", crate::data::s(&trial_rows(s)[i]["account_id"]))),
            )
            .name(ui.t("Trial balance"))
            .column(TableColumn::new("code", ui.t("Code")).width(100.0).min(72.0).sortable())
            .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
            .column(TableColumn::new("debit", ui.t("Debit")).width(140.0).min(96.0).align(Align::End))
            .column(TableColumn::new("credit", ui.t("Credit")).width(140.0).min(96.0).align(Align::End))
            .cell(|s: &State, i, col| {
                let r = &trial_rows(s)[i];
                match col {
                    "code" => Cell::text(crate::data::s(&r["code"])),
                    "name" => Cell::text(crate::data::s(&r["name"])),
                    "debit" => Cell::text(s.fmt.money(&crate::data::s(&r["debit"]))).mono(),
                    _ => Cell::text(s.fmt.money(&crate::data::s(&r["credit"]))).mono(),
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
                .child(ui.fill_or_empty(trial_rows(s).len(), "Nothing to show", hint, &icons::REPORTS, table))
        }
        2 => {
            let hint = "Who owes you and who you owe.";
            let table = Table::new(
                |s: &State| &s.reports.table,
                |s: &mut State| &mut s.reports.table,
                |s: &State| party_keys(s).len(),
                |s: &State, i| Cow::Owned(format!("party-bal-{}", party_keys(s)[i].0)),
            )
            .name(ui.t("Customers & Suppliers"))
            .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
            .column(TableColumn::new("role", ui.t("Role")).width(120.0).min(88.0))
            .column(TableColumn::new("balance", ui.t("Balance")).width(160.0).min(110.0).align(Align::End).sortable())
            .cell(|s: &State, i, col| {
                let (key, amount) = &party_keys(s)[i];
                let (role, id) = key.split_once(':').unwrap_or(("contact", key.as_str()));
                let name = s.books.party_name(id);
                match col {
                    "name" => Cell::text(if name.is_empty() { id.to_string() } else { name }),
                    "role" => Cell::text(if role == "customer" {
                        s.fmt.t("Customer")
                    } else {
                        s.fmt.t("Supplier")
                    })
                    .muted(),
                    _ => Cell::text(s.fmt.money(amount)).mono(),
                }
            })
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
                .child(ui.fill_or_empty(party_keys(s).len(), "Nothing to show", hint, &icons::REPORTS, table))
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
                .child(ui.metric("pl-rev", &icons::SALES, &ui.t("Revenue"), ui.f.money(&revenue), ui.t("Posted entries"), Tone::Success))
                .child(ui.metric("pl-exp", &icons::PURCHASES, &ui.t("Expenses"), ui.f.money(&expenses), ui.t("Posted entries"), Tone::Warning))
                .child(ui.metric("pl-net", &icons::REPORTS, &ui.t("Net profit"), ui.f.money(&profit), ui.t("Profit this period"), Tone::Info));

            let table = Table::new(
                |s: &State| &s.reports.table,
                |s: &mut State| &mut s.reports.table,
                |s: &State| pl_rows(s).len(),
                |s: &State, i| Cow::Owned(format!("pl-{}", crate::data::s(&pl_rows(s)[i]["account_id"]))),
            )
            .name(ui.t("Profit & Loss"))
            .column(TableColumn::new("code", ui.t("Code")).width(100.0).min(72.0).sortable())
            .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
            .column(TableColumn::new("class", ui.t("Classification")).width(120.0).min(96.0))
            .column(TableColumn::new("amount", ui.t("Amount")).width(160.0).min(100.0).align(Align::End).sortable())
            .cell(|s: &State, i, col| {
                let r = &pl_rows(s)[i];
                match col {
                    "code" => Cell::text(crate::data::s(&r["code"])),
                    "name" => Cell::text(crate::data::s(&r["name"])),
                    "class" => Cell::text(crate::data::s(&r["classification"])).muted(),
                    _ => Cell::text(s.fmt.money(&crate::data::s(&r["amount"]))).mono(),
                }
            })
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
                .child(ui.fill_or_empty(pl_rows(s).len(), "Nothing to show", hint, &icons::REPORTS, table))
        }
    };

    ui.page(
        "reports-screen",
        &ui.t("Reports"),
        &ui.t("Reports for this fiscal period"),
        vec![ui.btn(Btn::Glass, "Export CSV", Some(&icons::DOWNLOAD), export_csv)],
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
    let rows = sheet_rows(s);
    if rows.is_empty() {
        return col()
            .w_full()
            .gap(14.0)
            .grow(1.0)
            .min_h(0.0)
            .child(tab_bar)
            .child(ui.empty(&icons::REPORTS, "No balances through this date", "Balance sheet").grow(1.0));
    }
    let table = Table::new(
        |s: &State| &s.reports.table,
        |s: &mut State| &mut s.reports.table,
        |s: &State| sheet_rows(s).len(),
        |s: &State, i| Cow::Owned(format!("bs-{}", crate::data::s(&sheet_rows(s)[i].1["account_id"]))),
    )
    .name(ui.t("Balance sheet"))
    .column(TableColumn::new("class", ui.t("Classification")).width(120.0).min(88.0))
    .column(TableColumn::new("code", ui.t("Code")).width(100.0).min(72.0))
    .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0))
    .column(TableColumn::new("amount", ui.t("Amount")).width(160.0).min(110.0).align(Align::End))
    .cell(|s: &State, i, col| {
        let (class, row) = &sheet_rows(s)[i];
        match col {
            "class" => {
                let key = match class.as_str() {
                    "assets" => "Asset",
                    "liabilities" => "Liability",
                    "equity" => "Equity",
                    other => other,
                };
                Cell::text(s.fmt.t(key))
            }
            "code" => Cell::text(crate::data::s(&row["code"])),
            "name" => Cell::text(crate::data::s(&row["name"])),
            _ => Cell::text(s.fmt.money(&crate::data::s(&row["amount"]))).mono(),
        }
    })
    .zebra(true)
    .build(s, &ui.k)
    .key("balance-sheet-table")
    .grow(1.0)
    .min_h(0.0);
    let balanced = s.books.report["balance_sheet"]["balanced"].as_bool().unwrap_or(false);
    col()
        .w_full()
        .gap(14.0)
        .grow(1.0)
        .min_h(0.0)
        .child(tab_bar)
        .child(ui.muted(ui.t(if balanced { "Balanced" } else { "No balances through this date" })))
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
    if aging_rows(s).is_empty() {
        return col()
            .w_full()
            .gap(14.0)
            .grow(1.0)
            .min_h(0.0)
            .child(tab_bar)
            .child(ui.empty(&icons::REPORTS, "No outstanding balances — everything is settled.", "Aging").grow(1.0));
    }
    let table = Table::new(
        |s: &State| &s.reports.table,
        |s: &mut State| &mut s.reports.table,
        |s: &State| aging_rows(s).len(),
        |s: &State, i| Cow::Owned(format!("age-{}", crate::data::s(&aging_rows(s)[i]["party_id"]))),
    )
    .name(ui.t("Aging"))
    .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0))
    .column(TableColumn::new("side", ui.t("Role")).width(120.0).min(88.0))
    .column(TableColumn::new("total", ui.t("Balance")).width(160.0).min(110.0).align(Align::End))
    .cell(|s: &State, i, col| {
        let row = &aging_rows(s)[i];
        let id = crate::data::s(&row["party_id"]);
        match col {
            "name" => Cell::text(s.books.party_name(&id)),
            "side" => Cell::text(s.fmt.t(if row["side"] == "payable" { "Supplier" } else { "Customer" })).muted(),
            _ => Cell::text(s.fmt.money(&crate::data::s(&row["total"]))).mono(),
        }
    })
    .zebra(true)
    .build(s, &ui.k)
    .key("aging-table")
    .grow(1.0)
    .min_h(0.0);
    col().w_full().gap(14.0).grow(1.0).min_h(0.0).child(tab_bar).child(table)
}

fn export_csv(st: &mut State) {
    let dir = st.engine.dir.join("exports");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        st.toast(Tone::Danger, e.to_string());
        return;
    }
    let (from, to) = st.books.range();
    let mut written = Vec::new();
    for action in ["report_csv", "profit_csv"] {
        match st.query(action, serde_json::json!({ "from": from, "to": to })) {
            Ok(v) => {
                let name = crate::data::s(&v["filename"]);
                let content = v["content"].as_str().unwrap_or("").to_string();
                if name.is_empty() || content.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Report unavailable"));
                    return;
                }
                if let Err(e) = std::fs::write(dir.join(&name), content.as_bytes()) {
                    st.toast(Tone::Danger, e.to_string());
                    return;
                }
                written.push(name);
            }
            Err(e) => {
                st.toast_err(e);
                return;
            }
        }
    }
    st.toast(Tone::Success, format!("{} {}", st.fmt.t("Report exported"), written.join(", ")));
}
