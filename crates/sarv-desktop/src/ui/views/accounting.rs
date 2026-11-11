//! Accounting screen: journals, chart of accounts and fiscal periods — kit Table + tabs.

use std::borrow::Cow;

use rune::{Align, Cx, El, col};
use rune_kit::{Cell, Table, TableColumn, TableState};

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;

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
    pub post_key: String,
    pub posting: bool,
}

fn accounts(s: &State) -> &[serde_json::Value] {
    s.books.workspace["accounts"].as_array().map(|a| a.as_slice()).unwrap_or(&[])
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
    });

    let table = match s.accounting.tab_idx {
        1 => Table::new(
            |s: &State| &s.accounting.table,
            |s: &mut State| &mut s.accounting.table,
            |s: &State| accounts(s).len(),
            |s: &State, i| Cow::Owned(format!("acc-{}", crate::data::s(&accounts(s)[i]["id"]))),
        )
        .name(ui.t("Chart of accounts"))
        .column(TableColumn::new("code", ui.t("Code")).width(100.0).min(72.0).sortable())
        .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
        .column(TableColumn::new("class", ui.t("Classification")).width(140.0).min(100.0))
        .cell(|s: &State, i, col| {
            let acc = &accounts(s)[i];
            match col {
                "code" => Cell::text(crate::data::s(&acc["code"])),
                "name" => Cell::text(crate::data::s(&acc["name"])),
                _ => Cell::text(s.fmt.t(&crate::data::s(&acc["classification"]))).muted(),
            }
        })
        .on_select(|st, id, _| st.accounting.selected = id.to_string())
        .zebra(true)
        .build(s, &ui.k)
        .key("coa-table")
        .grow(1.0)
        .min_h(0.0),
        2 => Table::new(
            |s: &State| &s.accounting.table,
            |s: &mut State| &mut s.accounting.table,
            |s: &State| periods(s).len(),
            |s: &State, i| Cow::Owned(format!("period-{}", crate::data::s(&periods(s)[i]["id"]))),
        )
        .name(ui.t("Fiscal periods"))
        .column(TableColumn::new("name", ui.t("Name")).flex(2.0).min(140.0).sortable())
        .column(TableColumn::new("date", ui.t("Date")).width(120.0).min(96.0))
        .column(TableColumn::new("status", ui.t("Status")).width(120.0).min(88.0))
        .cell(|s: &State, i, col| {
            let period = &periods(s)[i];
            match col {
                "name" => Cell::text(crate::data::s(&period["name"])),
                "date" => Cell::text(s.fmt.date(&crate::data::s(&period["start_date"]))).muted(),
                _ => {
                    let locked = period["locked"].as_bool().unwrap_or(false);
                    Cell::text(s.fmt.t(period_status_key(locked))).muted()
                }
            }
        })
        .on_select(|st, id, _| st.accounting.selected = id.to_string())
        .zebra(true)
        .build(s, &ui.k)
        .key("periods-table")
        .grow(1.0)
        .min_h(0.0),
        _ => Table::new(
            |s: &State| &s.accounting.table,
            |s: &mut State| &mut s.accounting.table,
            |s: &State| journals(s).len(),
            |s: &State, i| {
                let id = crate::data::s(&journals(s)[i]["id"]);
                Cow::Owned(format!("journal-{id}"))
            },
        )
        .name(ui.t("Journals"))
        .column(TableColumn::new("ref", ui.t("Reference")).width(120.0).min(88.0))
        .column(TableColumn::new("date", ui.t("Date")).width(110.0).min(88.0))
        .column(TableColumn::new("desc", ui.t("Description")).flex(2.0).min(140.0))
        .column(TableColumn::new("debit", ui.t("Debit")).width(140.0).min(96.0).align(Align::End))
        .column(TableColumn::new("credit", ui.t("Credit")).width(140.0).min(96.0).align(Align::End))
        .column(TableColumn::new("status", ui.t("Status")).width(100.0).min(80.0))
        .cell(|s: &State, i, col| {
            let list = journals(s);
            let Some(j) = list.get(i) else {
                return Cell::text("");
            };
            let id = crate::data::s(&j["id"]);
            match col {
                "ref" => Cell::text(format!("#{}", &id[..id.len().min(8)])),
                "date" => Cell::text(s.fmt.date(&crate::data::s(&j["date"]))).muted(),
                "desc" => Cell::text(crate::data::s(&j["reason"])),
                "debit" => Cell::text(s.fmt.money(&crate::data::s(&j["total_debit"]))).mono(),
                "credit" => Cell::text(s.fmt.money(&crate::data::s(&j["total_credit"]))).mono(),
                _ => Cell::text(s.fmt.t("Posted")).muted(),
            }
        })
        .on_select(|st, id, _| st.accounting.selected = id.to_string())
        .zebra(true)
        .build(s, &ui.k)
        .key("journals-table")
        .grow(1.0)
        .min_h(0.0),
    };

    let (hint, empty_title) = tab_empty_copy(s.accounting.tab_idx);
    let empty_count = match s.accounting.tab_idx {
        1 => accounts(s).len(),
        2 => periods(s).len(),
        _ => journals(s).len(),
    };

    let body = col()
        .w_full()
        .gap(14.0)
        .grow(1.0)
        .min_h(0.0)
        .child(tab_bar)
        .child(ui.muted(ui.t(hint)))
        .child(ui.fill_or_empty(empty_count, empty_title, hint, &icons::REPORTS, table));

    if s.accounting.modal_open {
        mount_journal_dialog(s, ui, layer);
    }
    if s.accounting.ledger_open {
        mount_ledger(s, ui, layer);
    }

    ui.page(
        "accounting-screen",
        &ui.t("Accounting"),
        &ui.t("Ledger and accounting"),
        vec![
            ui.btn(Btn::Primary, "New journal", Some(&icons::PLUS), |st| {
                st.accounting.modal_open = true;
                st.accounting.debit_account.clear();
                st.accounting.credit_account.clear();
                st.accounting.post_key =
                    format!("journal-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
                st.accounting.posting = false;
                st.forms.set("journal-amount", "");
                st.forms.set("journal-reason", "");
            }),
            ui.btn(Btn::Glass, "Ledger", None, open_ledger),
            ui.btn(Btn::Glass, "Reverse", None, reverse_selected),
            ui.btn(Btn::Glass, "Lock fiscal period", None, toggle_period),
        ],
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
        if s.accounting.debit_account.is_empty() { None } else { Some(s.accounting.debit_account.as_str()) },
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
        if s.accounting.credit_account.is_empty() { None } else { Some(s.accounting.credit_account.as_str()) },
        &ui.t("Select account"),
        pickers::ACCOUNT_PICKER_WIDTH,
        credit_open,
        |st, id| st.accounting.credit_account = id.to_string(),
    );
    let form = col()
        .gap(14.0)
        .child(ui.form_grid(ui.labeled("Debit", debit), ui.labeled("Credit", credit)))
        .child(ui.labeled("Amount", ui.field(s, "journal-amount", "۰")))
        .child(ui.labeled("Description", ui.field(s, "journal-reason", &ui.t("Note (optional)"))));

    ui.dialog_described(
        layer,
        &mut portals,
        "journal",
        &ui.t("New journal"),
        &ui.t("Post a balanced debit and credit."),
        true,
        900.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.accounting.modal_open = false),
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
                let amt = st.parse_money_input(&st.forms.get("journal-amount"));
                if amt <= 0 {
                    st.toast(Tone::Danger, st.fmt.t("Amount"));
                    st.accounting.posting = false;
                    return;
                }
                let reason = {
                    let r = st.forms.get("journal-reason");
                    if r.trim().is_empty() { "Manual journal".into() } else { r }
                };
                let key = if st.accounting.post_key.is_empty() {
                    format!("journal-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
                } else {
                    st.accounting.post_key.clone()
                };
                let mut payload = json!({
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
                    }
                    Err(_) => {
                        // `run` already toasted the engine error.
                        st.accounting.posting = false;
                    }
                }
            }),
        ],
        |st| st.accounting.modal_open = false,
    );
}

fn open_ledger(st: &mut State) {
    if !st.accounting.selected.starts_with("acc-") {
        st.toast(Tone::Info, st.fmt.t("Select account"));
        return;
    }
    let id = st.accounting.selected.strip_prefix("acc-").unwrap_or(&st.accounting.selected);
    let (from, to) = st.books.range();
    match st.query("ledger", serde_json::json!({ "account_id": id, "from": from, "to": to })) {
        Ok(v) => {
            st.accounting.ledger = v;
            st.accounting.ledger_open = true;
        }
        Err(e) => st.toast_err(e),
    }
}

fn reverse_selected(st: &mut State) {
    if !st.accounting.selected.starts_with("journal-") {
        st.toast(Tone::Info, st.fmt.t("No journals"));
        return;
    }
    let id = st.accounting.selected.strip_prefix("journal-").unwrap_or(&st.accounting.selected).to_string();
    let Some(journal) = st.books.list("journals").into_iter().find(|j| crate::data::s(&j["id"]) == id) else {
        st.toast(Tone::Info, st.fmt.t("No journals"));
        return;
    };
    let key = format!("rev-{id}");
    let mut payload = serde_json::json!({
        "journal_id": id,
        "period_id": crate::data::s(&journal["period_id"]),
        "date": crate::data::s(&journal["date"]),
        "request_key": key,
        "source_id": key,
        "reason": "Reverse entry"
    });
    match st.query("journal_reverse_preview", payload.clone()) {
        Ok(pr) => payload["expected_generation"] = pr["_generation"].clone(),
        Err(e) => {
            st.toast_err(e);
            return;
        }
    }
    let _ = st.run("journal_reverse", payload, Some("Posted"));
}

fn toggle_period(st: &mut State) {
    if !st.accounting.selected.starts_with("period-") {
        st.toast(Tone::Info, st.fmt.t("No fiscal periods"));
        return;
    }
    let id = st.accounting.selected.strip_prefix("period-").unwrap_or(&st.accounting.selected).to_string();
    let locked = st
        .books
        .workspace["periods"]
        .as_array()
        .and_then(|rows| rows.iter().find(|p| crate::data::s(&p["id"]) == id))
        .and_then(|p| p["locked"].as_bool())
        .unwrap_or(false);
    let key = format!("lock-{id}-{}", !locked);
    let _ = st.run(
        "period_lock",
        serde_json::json!({
            "period_id": id,
            "locked": !locked,
            "reason": if locked { "Reopen fiscal period" } else { "Lock fiscal period" },
            "request_key": key,
            "source_id": key
        }),
        Some("Posted"),
    );
}

fn mount_ledger(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    let rows = s.accounting.ledger["rows"].as_array().cloned().unwrap_or_default();
    let mut body = col().gap(8.0).child(ui.muted(format!("{} {}", ui.t("Ledger"), crate::data::s(&s.accounting.ledger["count"]))));
    for row in rows.iter().take(12) {
        let line = format!(
            "{}  {}  {}",
            s.fmt.date(&crate::data::s(&row["date"])),
            s.fmt.money(&crate::data::s(&row["debit"])),
            s.fmt.money(&crate::data::s(&row["credit"]))
        );
        body = body.child(ui.body(line));
    }
    if rows.is_empty() {
        body = body.child(ui.empty(&icons::REPORTS, "No entries in this period", "Ledger"));
    }
    let mut portals = Vec::new();
    ui.dialog_described(
        layer,
        &mut portals,
        "ledger",
        &ui.t("Ledger"),
        &ui.t("Every entry behind your numbers."),
        true,
        640.0,
        body,
        vec![ui.btn(Btn::Ghost, "Close", None, |st| st.accounting.ledger_open = false)],
        |st| st.accounting.ledger_open = false,
    );
}
