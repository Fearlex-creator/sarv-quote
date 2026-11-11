//! First-run company setup: create a company and open the books with zero balances.

use chrono::Datelike;
use rune::{Cx, El, col, row};
use serde_json::json;

use crate::app::{Page, State, Tone};
use crate::i18n::{CalendarKind, Lang};
use crate::ui::glass::{Btn, Ui};
use crate::ui::icons;

/// Inclusive fiscal period for `today`, stored as ISO. Jalali years use the civil Nowruz range.
pub fn fiscal_period(calendar: CalendarKind, today: chrono::NaiveDate) -> (String, String, String) {
    if calendar == CalendarKind::Jalali {
        if let Ok(j) = sarv_quote::calendar::from_gregorian(today) {
            if let Ok((start, end)) = sarv_quote::calendar::jalali_fiscal_year(j.year()) {
                return (
                    j.year().to_string(),
                    start.format("%Y-%m-%d").to_string(),
                    end.format("%Y-%m-%d").to_string(),
                );
            }
        }
    }
    let y = today.year();
    (y.to_string(), format!("{y}-01-01"), format!("{y}-12-31"))
}

fn iso_or_parsed(s: &State, raw: &str, fallback: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return fallback.to_string();
    }
    let cal = if s.fmt.calendar == CalendarKind::Jalali {
        sarv_quote::calendar::Calendar::Jalali
    } else {
        sarv_quote::calendar::Calendar::Gregorian
    };
    sarv_quote::calendar::parse_date(raw, cal)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| raw.to_string())
}

#[derive(Default)]
pub struct SetupState {
    pub step: usize,
}

pub fn view(s: &State, _cx: &mut Cx<State>, ui: &Ui, _layer: &mut Vec<El>) -> El {
    if s.books.has_company() && !s.books.initialized() {
        return opening_step(s, ui);
    }
    if s.setup.step == 0 {
        return welcome(s, ui);
    }

    let fiscal = fiscal_period(s.fmt.calendar, chrono::Local::now().date_naive());
    let form = ui
        .kit_card()
        .title(ui.t("Set up company"), Some(&icons::BUILDING))
        .description(ui.t("Give your business a home in Sarv."))
        .child(ui.labeled("Name", ui.field(s, "setup-name", &ui.t("Your business"))))
        .child(ui.labeled("Fiscal year", ui.field(s, "setup-fiscal", &fiscal.0)))
        .child(
            row()
                .w_full()
                .gap(12.0)
                .child(ui.labeled("Start date", ui.field(s, "setup-start", &s.fmt.date(&fiscal.1))))
                .child(ui.labeled("End date", ui.field(s, "setup-end", &s.fmt.date(&fiscal.2)))),
        )
        .child(language_row(s, ui))
        .child(theme_row(s, ui))
        .build(&ui.k);

    let create = ui.btn(Btn::Primary, "Create a company and fiscal period", Some(&icons::CHECK), |st| {
        let name = st.forms.get("setup-name").trim().to_string();
        if name.is_empty() {
            st.toast(Tone::Warning, st.fmt.t("Name"));
            return;
        }
        let defaults = fiscal_period(st.fmt.calendar, chrono::Local::now().date_naive());
        let fiscal = {
            let f = st.forms.get("setup-fiscal").trim().to_string();
            if f.is_empty() { defaults.0.clone() } else { crate::i18n::latin_digits(&f) }
        };
        let start = iso_or_parsed(st, &st.forms.get("setup-start"), &defaults.1);
        let end = iso_or_parsed(st, &st.forms.get("setup-end"), &defaults.2);
        let locale = match st.lang() {
            Lang::Fa => "fa",
            Lang::En => "en",
        };
        let res = st.run(
            "setup_company",
            json!({
                "name": name,
                "fiscal_name": fiscal,
                "start_date": start,
                "end_date": end,
                "locale": locale,
                "timezone": "AsiaTehran",
            }),
            Some("Create a company and fiscal period"),
        );
        if let Ok(v) = res {
            if let Some(id) = v["company_id"].as_str() {
                st.select_company(id);
            }
        }
    });

    let hero = col()
        .w_full()
        .max_w(560.0)
        .gap(16.0)
        .child(form)
        .child(
            row()
                .w_full()
                .gap(8.0)
                .child(ui.btn(Btn::Ghost, "Back", None, |st| st.setup.step = 0))
                .child(create),
        );

    stage("setup-screen", hero)
}

fn welcome(s: &State, ui: &Ui) -> El {
    let format = if s.lang() == Lang::Fa {
        "Persian · Jalali calendar · Persian digits"
    } else {
        "English · Gregorian calendar · Latin digits"
    };
    let steps = [
        ("Company", "Give your business a home in Sarv."),
        ("Fiscal period", "Choose the dates that fit your financial year."),
        ("Opening balances", "Start fresh or bring in your existing balances."),
    ];
    let mut list = col().gap(14.0);
    for (title, detail) in steps {
        list = list.child(
            col()
                .gap(2.0)
                .child(ui.body(ui.t(title)))
                .child(ui.muted(ui.t(detail))),
        );
    }
    let card = ui
        .kit_card()
        .title(ui.t("What you’ll set up"), None)
        .description(ui.t(format))
        .child(list)
        .child(language_row(s, ui))
        .child(theme_row(s, ui))
        .action(ui.btn(Btn::Primary, "Get started", Some(&icons::ARROW), |st| st.setup.step = 1))
        .build(&ui.k);

    stage(
        "setup-welcome",
        col()
            .w_full()
            .max_w(520.0)
            .gap(16.0)
            .child(ui.title(ui.t("Welcome to Sarv")))
            .child(ui.muted(ui.t(
                "Set up your company, choose your financial year, and begin with balances you can trust.",
            )))
            .child(card)
            .child(ui.muted(ui.t("Works offline · Saved on this device"))),
    )
}

fn language_row(s: &State, ui: &Ui) -> El {
    let fa = ui.t("Persian");
    let items = [(fa.as_str(), s.lang() == Lang::Fa), ("English", s.lang() == Lang::En)];
    let ctrl = ui.tabs("setup-lang", &items, |st, idx| {
        if idx == 0 {
            st.set_lang(Lang::Fa);
            st.fmt.calendar = CalendarKind::Jalali;
        } else {
            st.set_lang(Lang::En);
            st.fmt.calendar = CalendarKind::Gregorian;
        }
    });
    row().w_full().gap(16.0).center_y().child(ui.muted(ui.t("Interface language")).grow(1.0)).child(ctrl)
}

fn theme_row(s: &State, ui: &Ui) -> El {
    let dark = ui.t("Dark");
    let light = ui.t("Light");
    let items = [(dark.as_str(), s.dark), (light.as_str(), !s.dark)];
    let ctrl = ui.tabs("setup-theme", &items, |st, idx| st.dark = idx == 0);
    row().w_full().gap(16.0).center_y().child(ui.muted(ui.t("Theme")).grow(1.0)).child(ctrl)
}

fn stage(key: &str, body: El) -> El {
    col().key(key).w_full().h_full().center().padding(32.0).child(body)
}

fn opening_step(s: &State, ui: &Ui) -> El {
    let card = ui
        .kit_card()
        .title(ui.t("Record your opening balances"), Some(&icons::SHIELD))
        .description(ui.t("Start fresh or bring in your existing balances."))
        .child(ui.form_grid(
            ui.labeled("Cash", ui.field(s, "opening-cash", "0")),
            ui.labeled("Bank", ui.field(s, "opening-bank", "0")),
        ))
        .action(ui.btn(Btn::Primary, "Start fresh", Some(&icons::CHECK), |st| {
            let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
            let source = format!("open-{stamp}");
            let cash = st.parse_money_input(&st.forms.get("opening-cash"));
            let bank = st.parse_money_input(&st.forms.get("opening-bank"));
            let res = if cash == 0 && bank == 0 {
                st.run(
                    "initialize_zero",
                    json!({
                        "confirmed": true,
                        "source_id": source,
                        "request_key": format!("req-{stamp}"),
                    }),
                    Some("Start with zero balances"),
                )
            } else {
                post_opening(st, cash, bank, &source)
            };
            if res.is_ok() {
                st.nav(Page::Overview);
            }
        }))
        .build(&ui.k);

    stage(
        "setup-opening",
        col()
            .w_full()
            .max_w(480.0)
            .gap(16.0)
            .child(ui.title(ui.t("Opening balances")))
            .child(ui.muted(ui.t("Start fresh or bring in your existing balances.")))
            .child(card)
            .child(ui.muted(ui.t("Works offline · Saved on this device"))),
    )
}

const OPENING_COLUMNS: &[&str] = &[
    "version",
    "kind",
    "id",
    "company_id",
    "name",
    "role",
    "amount",
    "credit",
    "unit",
    "quantity",
    "numerator",
    "denominator",
    "party_id",
    "item_id",
    "warehouse_id",
    "unit_id",
    "base_unit_id",
    "customer",
    "supplier",
    "indivisible",
    "reference",
    "settlement_reference",
    "bank_reference",
    "holder",
    "state",
    "document_date",
    "due_date",
    "as_of",
    "statement_amount",
];

fn csv_row(fields: &[(&str, &str)]) -> String {
    OPENING_COLUMNS
        .iter()
        .map(|col| fields.iter().find(|(k, _)| k == col).map(|(_, v)| *v).unwrap_or(""))
        .collect::<Vec<_>>()
        .join(",")
}

fn opening_csv(company_id: &str, as_of: &str, cash: i64, bank: i64) -> String {
    let p = if company_id.len() >= 8 { &company_id[..8] } else { company_id };
    let ledger = |role: &str, debit: i64, credit: i64| {
        csv_row(&[
            ("version", "sarv-opening-v1"),
            ("kind", "ledger"),
            ("id", &format!("{p}-GL-{role}")),
            ("company_id", company_id),
            ("role", role),
            ("amount", &debit.to_string()),
            ("credit", &credit.to_string()),
            ("unit", "IRR"),
            ("reference", &format!("{p}-OPEN-{role}")),
        ])
    };
    let mut rows = vec![
        OPENING_COLUMNS.join(","),
        csv_row(&[
            ("version", "sarv-opening-v1"),
            ("kind", "opening"),
            ("id", &format!("{p}-OPEN")),
            ("company_id", company_id),
            ("name", "Opening balances"),
            ("as_of", as_of),
        ]),
    ];
    let cash_s = cash.to_string();
    let bank_s = bank.to_string();
    rows.push(ledger("cash", cash, 0));
    rows.push(ledger("bank", bank, 0));
    for role in [
        "receivables",
        "inventory",
        "cheques_receivable",
        "payables",
        "cheques_payable",
        "grni",
        "supplier_advances",
        "customer_advances",
        "cogs",
        "inventory_loss",
    ] {
        rows.push(ledger(role, 0, 0));
    }
    rows.push(ledger("owner_capital", 0, cash + bank));
    if cash > 0 {
        rows.push(csv_row(&[
            ("version", "sarv-opening-v1"),
            ("kind", "cash"),
            ("id", &format!("{p}-C1")),
            ("company_id", company_id),
            ("name", "Main Cash"),
            ("amount", &cash_s),
            ("unit", "IRR"),
            ("reference", &format!("{p}-CASH-1")),
        ]));
    }
    if bank > 0 {
        rows.push(csv_row(&[
            ("version", "sarv-opening-v1"),
            ("kind", "bank"),
            ("id", &format!("{p}-B1")),
            ("company_id", company_id),
            ("name", "Main Bank"),
            ("amount", &bank_s),
            ("unit", "IRR"),
            ("reference", &format!("{p}-BANK-1")),
            ("bank_reference", "MAIN-BANK"),
            ("statement_amount", &bank_s),
        ]));
    }
    rows.join("\n")
}

fn post_opening(st: &mut State, cash: i64, bank: i64, source: &str) -> Result<serde_json::Value, String> {
    let as_of = st.books.range().0;
    let csv = opening_csv(&st.books.company_id, &as_of, cash, bank);
    let review = st.query(
        "opening_csv_preview",
        json!({ "company_id": st.books.company_id, "csv": csv, "request_key": source }),
    )?;
    let balanced = review["balanced"].as_bool().unwrap_or(false);
    let reconciled = review["reconciled"].as_bool().unwrap_or(false);
    if !balanced || !reconciled {
        let msg = st.fmt.t("Opening balances did not reconcile");
        st.toast(Tone::Danger, msg.clone());
        return Err(msg);
    }
    st.run(
        "opening_csv_post",
        json!({
            "company_id": st.books.company_id,
            "csv": csv,
            "request_key": source,
            "expected_generation": review["_generation"],
        }),
        Some("Record your opening balances"),
    )
}
