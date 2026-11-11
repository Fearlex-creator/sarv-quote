//! Banking and cash accounts — kit Table + metric cards.

use std::borrow::Cow;

use rune::{Cx, El, col, row};
use rune_kit::{Cell, Table, TableColumn, TableState};

use crate::app::{State, Tone};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;

#[derive(Default)]
pub struct BankingState {
    pub modal_open: bool,
    pub table: TableState,
    pub from_id: String,
    pub to_id: String,
    /// 0 transfer, 1 deposit (money in), 2 withdrawal (money out).
    pub kind: usize,
    pub post_key: String,
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

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, layer: &mut Vec<El>) -> El {
    let table = Table::new(
        |s: &State| &s.banking.table,
        |s: &mut State| &mut s.banking.table,
        |s: &State| bank_rows(s).len(),
        |s: &State, i| Cow::Owned(format!("bank-acc-{}", crate::data::s(&bank_rows(s)[i]["account"]["id"]))),
    )
    .name(ui.t("Banking"))
    .column(TableColumn::new("code", ui.t("Code")).width(100.0).min(72.0).sortable())
    .column(TableColumn::new("name", ui.t("Account")).flex(2.0).min(140.0).sortable())
    .column(TableColumn::new("class", ui.t("Classification")).width(140.0).min(100.0))
    .column(TableColumn::new("balance", ui.t("Balance")).width(160.0).min(110.0).align(rune::Align::End))
    .cell(|s: &State, i, col| {
        let rows = bank_rows(s);
        let Some(row) = rows.get(i) else {
            return Cell::text("");
        };
        let acc = &row["account"];
        match col {
            "code" => Cell::text(crate::data::s(&acc["code"])),
            "name" => Cell::text(crate::data::s(&acc["name"])),
            "balance" => Cell::text(s.fmt.money(&crate::data::s(&row["balance"]))).mono(),
            _ => Cell::text(s.fmt.t(&crate::data::s(&row["role"]))).muted(),
        }
    })
    .zebra(true)
    .build(s, &ui.k)
    .key("banking-table")
    .grow(1.0)
    .min_h(0.0);

    let kpis = row().w_full().gap(12.0).child(ui.metric(
        "kpi-bank-bal",
        &icons::WALLET,
        &ui.t("Cash and bank"),
        ui.f.money(&s.books.position("cash")),
        ui.t("Verified balance"),
        Tone::Info,
    ));

    let body = col()
        .w_full()
        .gap(16.0)
        .grow(1.0)
        .min_h(0.0)
        .child(kpis)
        .child(ui.fill_or_empty(bank_rows(s).len(), "No accounts", "Cash and bank", &icons::WALLET, table));

    if s.banking.modal_open {
        mount_transfer_dialog(s, ui, layer);
    }

    ui.page(
        "banking-screen",
        &ui.t("Banking"),
        &ui.t("Banking and payments"),
        vec![
            ui.btn(Btn::Primary, "Transfer", Some(&icons::ARROW), |st| open_move(st, 0)),
            ui.btn(Btn::Glass, "Money in", None, |st| open_move(st, 1)),
            ui.btn(Btn::Glass, "Money out", None, |st| open_move(st, 2)),
        ],
        body,
    )
}

fn mount_transfer_dialog(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    use crate::ui::pickers;
    use serde_json::json;

    let mut portals = Vec::new();
    let funds = funds_options(s);
    let other = if s.banking.kind == 0 { funds.clone() } else { counterpart_options(s) };
    let accounts = funds;
    let targets = other;
    let from_open = s.selects.get("bank-from").copied().unwrap_or(false);
    let to_open = s.selects.get("bank-to").copied().unwrap_or(false);
    let from = pickers::select_field(
        ui,
        &mut portals,
        "bank-from",
        &accounts,
        if s.banking.from_id.is_empty() { None } else { Some(s.banking.from_id.as_str()) },
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
        if s.banking.to_id.is_empty() { None } else { Some(s.banking.to_id.as_str()) },
        &ui.t("Select account"),
        pickers::ACCOUNT_PICKER_WIDTH,
        to_open,
        |st, id| st.banking.to_id = id.to_string(),
    );
    let form = col()
        .gap(14.0)
        .child(ui.form_grid(ui.labeled("From", from), ui.labeled("To", to)))
        .child(ui.labeled("Amount", ui.field(s, "bank-amount", "۰")))
        .child(ui.labeled("Description", ui.field(s, "bank-reason", &ui.t("Note (optional)"))));

    ui.dialog_described(
        layer,
        &mut portals,
        "banking-move",
        &ui.t("Transfer"),
        &ui.t("Move money between cash and bank accounts."),
        true,
        900.0,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, |st| st.banking.modal_open = false),
            ui.btn(Btn::Primary, "Save", Some(&icons::CHECK), |st| {
                if st.banking.from_id.is_empty() || st.banking.to_id.is_empty() {
                    st.toast(Tone::Danger, st.fmt.t("Select account"));
                    return;
                }
                let amt = st.parse_money_input(&st.forms.get("bank-amount"));
                if amt <= 0 {
                    st.toast(Tone::Danger, st.fmt.t("Amount"));
                    return;
                }
                let reason = {
                    let r = st.forms.get("bank-reason");
                    if r.trim().is_empty() { "Transfer".into() } else { r }
                };
                let kind = match st.banking.kind {
                    1 => "deposit",
                    2 => "withdrawal",
                    _ => "transfer",
                };
                let key = if st.banking.post_key.is_empty() {
                    format!("bank-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
                } else {
                    st.banking.post_key.clone()
                };
                let mut payload = json!({
                    "kind": kind,
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
                        return;
                    }
                }
                match st.run("banking_post", payload, Some("Posted")) {
                    Ok(_) => st.banking.modal_open = false,
                    Err(e) => st.toast_err(e),
                }
            }),
        ],
        |st| st.banking.modal_open = false,
    );
}

fn open_move(st: &mut State, kind: usize) {
    st.banking.modal_open = true;
    st.banking.kind = kind;
    st.banking.from_id.clear();
    st.banking.to_id.clear();
    st.banking.post_key = format!("bank-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
    st.forms.set("bank-amount", "");
    st.forms.set("bank-reason", "");
}
