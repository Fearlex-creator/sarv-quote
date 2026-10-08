//! Versioned, bounded opening CSV staging. Parsing never mutates the books.
use crate::{
    accounting::{AccountingState, Money, MoneyUnit},
    core::*,
    domain::{digits, quantity, text},
    imports::{CsvDiagnostic, CsvReport, MAX_DIAGNOSTICS, MAX_IMPORT},
};
use anyhow::{Result, ensure};
use std::{collections::HashSet, path::Path};

pub const OPENING_CSV_VERSION: &str = "sarv-opening-v1";
pub const OPENING_CSV_HEADER: [&str; 29] = [
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
const MAX_ROWS: usize = 50_000;
fn error<T>(report: &mut CsvReport<T>, row: usize, column: &str, reason: impl ToString) {
    if report.diagnostics.len() < MAX_DIAGNOSTICS {
        report.diagnostics.push(CsvDiagnostic {
            row,
            column: column.into(),
            reason: reason.to_string(),
        });
    } else {
        report.truncated = true;
    }
}
struct Row {
    line: usize,
    values: csv::StringRecord,
}
impl Row {
    fn get(&self, key: &str) -> &str {
        self.values[OPENING_CSV_HEADER.iter().position(|h| *h == key).unwrap()].trim()
    }
    fn required(&self, key: &str) -> Result<String> {
        let v = self.get(key);
        text(v)?;
        Ok(v.into())
    }
    fn boolean(&self, key: &str) -> Result<bool> {
        match self.get(key) {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => anyhow::bail!("true یا false را صریح وارد کنید"),
        }
    }
    fn integer(&self, key: &str) -> Result<i64> {
        let s = digits(self.get(key));
        ensure!(
            !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()),
            "عدد صحیح مثبت لازم است"
        );
        let n = s.parse::<i64>()?;
        ensure!(n > 0, "نسبت باید مثبت باشد");
        Ok(n)
    }
    fn money(&self, key: &str) -> Result<Money> {
        let unit = match self.get("unit") {
            "IRR" | "rial" | "ریال" => MoneyUnit::Rial,
            "toman" | "تومان" => MoneyUnit::Toman,
            _ => anyhow::bail!("واحد مبلغ باید IRR/rial/ریال یا toman/تومان باشد"),
        };
        let m = Money::parse(self.get(key), unit)?;
        ensure!(
            m.rials() >= 0 && m.rials() <= 9_000_000_000_000_000,
            "مبلغ باید نامنفی و در دامنهٔ مجاز باشد"
        );
        Ok(m)
    }
    fn date(&self, key: &str) -> Result<String> {
        let s = digits(self.get(key));
        let d = crate::calendar::parse_date(&s, crate::calendar::Calendar::Gregorian)?;
        ensure!(
            crate::calendar::format_date(d, crate::calendar::Calendar::Gregorian)? == s,
            "تاریخ باید YYYY-MM-DD میلادی باشد"
        );
        Ok(s)
    }
}
/// Pure stage: source metadata, identities, exact amounts, references and kernel
/// controls are validated against the immutable caller snapshot. A mismatch is
/// returned for review by `opening_preview`; posting remains a separate command.
pub fn opening_csv_report(
    state: &AccountingState,
    company_id: &str,
    input: &str,
    actor: &str,
    request_key: &str,
) -> CsvReport<OpeningCommand> {
    let mut report = CsvReport {
        value: None,
        diagnostics: vec![],
        truncated: false,
    };
    if input.len() > MAX_IMPORT {
        error(&mut report, 0, "file", "فایل بیش از ۱۰ مگابایت است");
        return report;
    }
    let Some(setup) = state
        .core
        .setups
        .iter()
        .find(|s| s.command.company_id == company_id)
    else {
        error(
            &mut report,
            0,
            "company_id",
            "ابتدا سیاست و نگاشت شرکت را تنظیم کنید",
        );
        return report;
    };
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::None)
        .from_reader(input.trim_start_matches('\u{feff}').as_bytes());
    match reader.headers() {
        Ok(h) if h.iter().eq(OPENING_CSV_HEADER) => {}
        Ok(_) => {
            error(
                &mut report,
                1,
                "header",
                format!(
                    "سرستون نسخهٔ ۱ باید دقیقاً {} باشد",
                    OPENING_CSV_HEADER.join(",")
                ),
            );
            return report;
        }
        Err(e) => {
            error(&mut report, 1, "header", e);
            return report;
        }
    }
    let mut rows = Vec::new();
    let mut ids = HashSet::new();
    for (index, record) in reader.records().enumerate() {
        if index >= MAX_ROWS {
            error(
                &mut report,
                index + 2,
                "row",
                "حداکثر ۵۰ هزار ردیف در هر فایل",
            );
            break;
        }
        let values = match record {
            Ok(r) => r,
            Err(e) => {
                error(&mut report, index + 2, "row", e);
                continue;
            }
        };
        let line = values
            .position()
            .map_or(index + 2, |p| (p.line() as usize).max(index + 2));
        let row = Row { line, values };
        if row.get("version") != OPENING_CSV_VERSION {
            error(
                &mut report,
                line,
                "version",
                "نسخهٔ پشتیبانی‌شده sarv-opening-v1 است",
            );
        }
        if row.get("company_id") != company_id {
            error(
                &mut report,
                line,
                "company_id",
                "شناسهٔ شرکت با شرکت انتخاب‌شده یکسان نیست",
            );
        }
        match row.required("id") {
            Ok(id) => {
                if !ids.insert(id) {
                    error(
                        &mut report,
                        line,
                        "id",
                        "شناسهٔ ردیف تکراری است؛ ادغام خودکار انجام نمی‌شود",
                    );
                }
            }
            Err(e) => error(&mut report, line, "id", e),
        };
        for (key, v) in OPENING_CSV_HEADER.iter().zip(row.values.iter()) {
            if !v.is_empty()
                && let Err(e) = text(v)
            {
                error(&mut report, line, key, e);
            }
        }
        rows.push(row);
        if report.diagnostics.len() >= MAX_DIAGNOSTICS {
            report.truncated = true;
            break;
        }
    }
    let metadata: Vec<_> = rows.iter().filter(|r| r.get("kind") == "opening").collect();
    if metadata.len() != 1 {
        error(
            &mut report,
            0,
            "kind",
            "دقیقاً یک ردیف opening با تاریخ و مرجع منبع لازم است",
        );
        return report;
    }
    let meta = metadata[0];
    let mut command = OpeningCommand {
        company_id: company_id.into(),
        period_id: setup.command.period_id.clone(),
        setup_id: setup.id.clone(),
        as_of: String::new(),
        source_id: meta.get("id").into(),
        request_key: request_key.into(),
        actor: actor.into(),
        reason: meta.get("name").into(),
        import_source: Some(OpeningImportSource {
            sha256: crate::imports::hash(input.as_bytes()),
            rows: rows
                .iter()
                .map(|r| OpeningSourceRow {
                    row: r.line,
                    id: r.get("id").into(),
                    kind: r.get("kind").into(),
                    original_values: r.values.iter().map(String::from).collect(),
                })
                .collect(),
        }),
        balances: vec![],
        parties: vec![],
        units: vec![],
        items: vec![],
        warehouses: vec![],
        cash: vec![],
        banks: vec![],
        open_items: vec![],
        advances: vec![],
        cheques: vec![],
        stock: vec![],
        grni: vec![],
    };
    match meta.date("as_of") {
        Ok(d) => command.as_of = d,
        Err(e) => error(&mut report, meta.line, "as_of", e),
    };
    let mut packs = Vec::new();
    let mut ledger_roles = HashSet::new();
    for row in &rows {
        let kind = row.get("kind");
        let fields: &[&str] = match kind {
            "opening" => &["name", "as_of"],
            "ledger" => &["role", "amount", "credit", "unit", "reference"],
            "party" => &["name", "customer", "supplier"],
            "unit" => &["name"],
            "item" => &["name", "reference", "base_unit_id", "indivisible"],
            "pack" => &["item_id", "unit_id", "numerator", "denominator"],
            "warehouse" => &["name"],
            "cash" => &["name", "amount", "unit", "reference"],
            "bank" => &[
                "name",
                "amount",
                "statement_amount",
                "unit",
                "reference",
                "bank_reference",
            ],
            "ar" | "ap" => &[
                "party_id",
                "amount",
                "unit",
                "reference",
                "document_date",
                "due_date",
            ],
            "supplier_advance" | "customer_advance" => {
                &["party_id", "amount", "unit", "reference", "document_date"]
            }
            "cheque_receivable" | "cheque_payable" => &[
                "party_id",
                "amount",
                "unit",
                "reference",
                "settlement_reference",
                "bank_reference",
                "holder",
                "state",
                "document_date",
                "due_date",
            ],
            "stock" => &[
                "item_id",
                "warehouse_id",
                "unit_id",
                "quantity",
                "amount",
                "unit",
                "reference",
            ],
            "grni" => &[
                "party_id",
                "item_id",
                "warehouse_id",
                "unit_id",
                "quantity",
                "amount",
                "unit",
                "reference",
                "document_date",
            ],
            _ => {
                error(
                    &mut report,
                    row.line,
                    "kind",
                    "نوع ردیف شناخته‌شده نیست؛ راهنمای قالب را ببینید",
                );
                continue;
            }
        };
        let before = report.diagnostics.len();
        for key in fields {
            if row.get(key).is_empty() {
                error(
                    &mut report,
                    row.line,
                    key,
                    "این ستون برای نوع ردیف الزامی است",
                );
            }
        }
        for key in &OPENING_CSV_HEADER[4..] {
            if !fields.contains(key) && !row.get(key).is_empty() {
                error(
                    &mut report,
                    row.line,
                    key,
                    "این ستون برای نوع ردیف مجاز نیست؛ مقدار را حذف کنید",
                );
            }
        }
        for key in fields {
            let result = match *key {
                "amount" | "credit" | "statement_amount" => row.money(key).map(|_| ()),
                "quantity" => quantity(row.get(key)).map(|_| ()),
                "numerator" | "denominator" => row.integer(key).map(|_| ()),
                "customer" | "supplier" | "indivisible" => row.boolean(key).map(|_| ()),
                "document_date" | "due_date" | "as_of" => row.date(key).map(|_| ()),
                _ => Ok(()),
            };
            if let Err(e) = result {
                error(&mut report, row.line, key, e);
            }
        }
        if before != report.diagnostics.len() {
            continue;
        }
        let result = (|| -> Result<()> {
            let id = row.get("id").to_string();
            let co = company_id.to_string();
            let reference = row.get("reference").to_string();
            match kind {
                "opening" => {
                    text(&command.reason)?;
                    text(actor)?;
                    text(request_key)?;
                }
                "ledger" => {
                    let role = CoreRole::ALL
                        .iter()
                        .find(|r| r.key() == row.get("role"))
                        .copied()
                        .ok_or_else(|| anyhow::anyhow!("نقش حساب نامعتبر است"))?;
                    ensure!(ledger_roles.insert(role.key()), "نقش دفتر تکراری است");
                    let mapping = setup
                        .command
                        .mappings
                        .iter()
                        .find(|m| m.role == role)
                        .ok_or_else(|| anyhow::anyhow!("نقش حساب نگاشت نشده است"))?;
                    command.balances.push(OpeningBalance {
                        account_id: mapping.account_id.clone(),
                        debit: row.money("amount")?,
                        credit: row.money("credit")?,
                        source_reference: reference,
                    });
                }
                "party" => command.parties.push(CoreParty {
                    id,
                    company_id: co,
                    name: row.get("name").into(),
                    customer: row.boolean("customer")?,
                    supplier: row.boolean("supplier")?,
                }),
                "unit" => command.units.push(CoreUnit {
                    id,
                    company_id: co,
                    name: row.get("name").into(),
                }),
                "item" => command.items.push(CoreItem {
                    id,
                    company_id: co,
                    sku: reference,
                    name: row.get("name").into(),
                    base_unit_id: row.get("base_unit_id").into(),
                    indivisible: row.boolean("indivisible")?,
                    packs: vec![],
                }),
                "pack" => packs.push((
                    row.line,
                    row.get("item_id").to_string(),
                    PackRatio {
                        unit_id: row.get("unit_id").into(),
                        numerator: row.integer("numerator")?,
                        denominator: row.integer("denominator")?,
                    },
                )),
                "warehouse" => command.warehouses.push(CoreWarehouse {
                    id,
                    company_id: co,
                    name: row.get("name").into(),
                }),
                "cash" => command.cash.push(OpeningCash {
                    id,
                    company_id: co,
                    name: row.get("name").into(),
                    balance: row.money("amount")?,
                    source_reference: reference,
                }),
                "bank" => command.banks.push(OpeningBank {
                    id,
                    company_id: co,
                    name: row.get("name").into(),
                    bank_reference: row.get("bank_reference").into(),
                    balance: row.money("amount")?,
                    statement_balance: row.money("statement_amount")?,
                    source_reference: reference,
                }),
                "ar" | "ap" => command.open_items.push(OpeningOpenItem {
                    id,
                    company_id: co,
                    party_id: row.get("party_id").into(),
                    side: if kind == "ar" {
                        OpenItemSide::Receivable
                    } else {
                        OpenItemSide::Payable
                    },
                    document_reference: reference,
                    document_date: row.date("document_date")?,
                    due_date: row.date("due_date")?,
                    remaining: row.money("amount")?,
                }),
                "supplier_advance" | "customer_advance" => command.advances.push(OpeningAdvance {
                    id,
                    company_id: co,
                    party_id: row.get("party_id").into(),
                    side: if kind == "supplier_advance" {
                        AdvanceSide::Supplier
                    } else {
                        AdvanceSide::Customer
                    },
                    source_reference: reference,
                    document_date: row.date("document_date")?,
                    remaining: row.money("amount")?,
                }),
                "cheque_receivable" | "cheque_payable" => {
                    let receive = kind == "cheque_receivable";
                    ensure!(
                        row.get("state") == if receive { "held" } else { "issued" },
                        "وضعیت چک دریافتنی held و پرداختنی issued لازم است"
                    );
                    command.cheques.push(OpeningCheque {
                        id,
                        company_id: co,
                        party_id: row.get("party_id").into(),
                        direction: if receive {
                            ChequeDirection::Receivable
                        } else {
                            ChequeDirection::Payable
                        },
                        state: if receive {
                            OpeningChequeState::Held
                        } else {
                            OpeningChequeState::Issued
                        },
                        bank_reference: row.get("bank_reference").into(),
                        holder: row.get("holder").into(),
                        cheque_reference: reference,
                        settlement_reference: row.get("settlement_reference").into(),
                        issue_date: row.date("document_date")?,
                        due_date: row.date("due_date")?,
                        amount: row.money("amount")?,
                    });
                }
                "stock" => command.stock.push(OpeningStock {
                    id,
                    company_id: co,
                    item_id: row.get("item_id").into(),
                    warehouse_id: row.get("warehouse_id").into(),
                    unit_id: row.get("unit_id").into(),
                    quantity_thousandths: quantity(row.get("quantity"))?,
                    value: row.money("amount")?,
                    source_reference: reference,
                }),
                "grni" => command.grni.push(OpeningGrni {
                    id,
                    company_id: co,
                    party_id: row.get("party_id").into(),
                    item_id: row.get("item_id").into(),
                    warehouse_id: row.get("warehouse_id").into(),
                    unit_id: row.get("unit_id").into(),
                    quantity_thousandths: quantity(row.get("quantity"))?,
                    value: row.money("amount")?,
                    receipt_reference: reference,
                    received_date: row.date("document_date")?,
                }),
                _ => unreachable!(),
            }
            Ok(())
        })();
        if let Err(e) = result {
            error(&mut report, row.line, kind, e);
        }
    }
    for (line, item, pack) in packs {
        if let Some(i) = command.items.iter_mut().find(|i| i.id == item) {
            i.packs.push(pack);
        } else {
            error(&mut report, line, "item_id", "کالای این بسته تعریف نشده است");
        }
    }
    // Resolve references independently of source order, attaching actionable rows.
    let parties: HashSet<_> = command.parties.iter().map(|x| x.id.as_str()).collect();
    let items: HashSet<_> = command.items.iter().map(|x| x.id.as_str()).collect();
    let units: HashSet<_> = command.units.iter().map(|x| x.id.as_str()).collect();
    let warehouses: HashSet<_> = command.warehouses.iter().map(|x| x.id.as_str()).collect();
    for row in &rows {
        for (key, set) in [
            ("party_id", &parties),
            ("item_id", &items),
            ("unit_id", &units),
            ("base_unit_id", &units),
            ("warehouse_id", &warehouses),
        ] {
            let value = row.get(key);
            if !value.is_empty() && !set.contains(value) {
                error(
                    &mut report,
                    row.line,
                    key,
                    "شناسه در همین فایل و شرکت تعریف نشده است",
                );
            }
        }
    }
    let mut sources = HashSet::new();
    let mut cheque_refs = HashSet::new();
    for row in &rows {
        let kind = row.get("kind");
        if matches!(kind, "stock" | "grni")
            && let Ok(q) = quantity(row.get("quantity"))
            && let Err(e) = command.base_quantity(row.get("item_id"), row.get("unit_id"), q)
        {
            error(&mut report, row.line, "unit_id", e);
        }
        let party_id = row.get("party_id");
        if !party_id.is_empty()
            && let Some(p) = command.parties.iter().find(|p| p.id == party_id)
        {
            let customer = matches!(kind, "ar" | "customer_advance" | "cheque_receivable");
            if (customer && !p.customer) || (!customer && !p.supplier) {
                error(
                    &mut report,
                    row.line,
                    "party_id",
                    "نقش مشتری یا تأمین‌کنندهٔ شخص برای این ردیف مجاز نیست",
                );
            }
        }
        if let Ok(date) = row.date("document_date") {
            if date > command.as_of {
                error(
                    &mut report,
                    row.line,
                    "document_date",
                    "تاریخ منبع تاریخی نباید پس از افتتاحیه باشد",
                );
            }
            if let Ok(due) = row.date("due_date")
                && due < date
            {
                error(
                    &mut report,
                    row.line,
                    "due_date",
                    "سررسید نباید پیش از تاریخ منبع باشد",
                );
            }
        }
        if kind == "bank"
            && let (Ok(book), Ok(statement)) = (row.money("amount"), row.money("statement_amount"))
            && book != statement
        {
            error(
                &mut report,
                row.line,
                "statement_amount",
                "ماندهٔ مستقل بانک و صورت‌حساب برابر نیست",
            );
        }
        if matches!(
            kind,
            "cash"
                | "bank"
                | "ar"
                | "ap"
                | "customer_advance"
                | "supplier_advance"
                | "stock"
                | "grni"
        ) {
            let party = if matches!(
                kind,
                "ar" | "ap" | "customer_advance" | "supplier_advance" | "grni"
            ) {
                party_id
            } else {
                ""
            };
            if !sources.insert((
                kind.to_string(),
                party.to_string(),
                crate::domain::normalize(row.get("reference")),
            )) {
                error(
                    &mut report,
                    row.line,
                    "reference",
                    "مرجع مالی تکراری است؛ ردیف را صریح اصلاح کنید",
                );
            }
        }
        if matches!(kind, "cheque_receivable" | "cheque_payable") {
            if !cheque_refs.insert((
                crate::domain::normalize(row.get("bank_reference")),
                crate::domain::normalize(row.get("reference")),
            )) {
                error(
                    &mut report,
                    row.line,
                    "reference",
                    "شناسهٔ بانکی چک تکراری است",
                );
            }
            if command.open_items.iter().any(|x| {
                x.party_id == party_id
                    && (x.side == OpenItemSide::Receivable) == (kind == "cheque_receivable")
                    && crate::domain::normalize(&x.document_reference)
                        == crate::domain::normalize(row.get("settlement_reference"))
            }) {
                error(
                    &mut report,
                    row.line,
                    "settlement_reference",
                    "مرجع تاریخی چک نباید دوباره به‌عنوان فاکتور باز وارد شود",
                );
            }
        }
    }
    if report.diagnostics.is_empty() {
        match state.opening_preview(&command) {
            Ok(_) => report.value = Some(command),
            Err(e) => error(&mut report, 0, "opening", e),
        }
    }
    report
}
pub fn opening_csv(
    state: &AccountingState,
    company_id: &str,
    input: &str,
    actor: &str,
    request_key: &str,
) -> Result<OpeningCommand> {
    opening_csv_report(state, company_id, input, actor, request_key).into_result()
}
pub fn opening_file_report(
    state: &AccountingState,
    company_id: &str,
    path: &Path,
    actor: &str,
    request_key: &str,
) -> CsvReport<OpeningCommand> {
    match crate::store::read_import(path) {
        Ok(input) => opening_csv_report(state, company_id, &input, actor, request_key),
        Err(e) => {
            let mut r = CsvReport {
                value: None,
                diagnostics: vec![],
                truncated: false,
            };
            error(&mut r, 0, "file", e);
            r
        }
    }
}
