use crate::domain::*;
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
pub const MAX_IMPORT: usize = 10 * 1024 * 1024;
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn product_fingerprint(products: &[Product]) -> Result<String> {
    let mut p = products.to_vec();
    p.sort_by_key(|p| normalize(&p.code));
    Ok(hash(&serde_json::to_vec(&p)?))
}
// Keep the stored v1 fingerprint unchanged: existing backups checksum that exact
// representation. This second key detects an equivalent CSV whose row order,
// alias order, digit script, or timestamp offset was changed.
fn equivalent_fingerprint(products: &[Product]) -> Result<String> {
    let mut products = products.to_vec();
    for product in &mut products {
        product.code = normalize(&product.code);
        product.name = normalize(&product.name);
        product.measure = normalize(&product.measure);
        product.aliases = product.aliases.iter().map(|a| normalize(a)).collect();
        product.aliases.sort();
        product.aliases.dedup();
        product.source_at = chrono::DateTime::parse_from_rfc3339(&product.source_at)?
            .with_timezone(&chrono::Utc)
            .to_rfc3339();
    }
    product_fingerprint(&products)
}
pub const MAX_DIAGNOSTICS: usize = 100;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsvDiagnostic {
    pub row: usize,
    pub column: String,
    pub reason: String,
}
#[derive(Debug)]
pub struct CsvReport<T> {
    pub value: Option<T>,
    pub diagnostics: Vec<CsvDiagnostic>,
    pub truncated: bool,
}
impl<T> CsvReport<T> {
    fn new() -> Self {
        Self {
            value: None,
            diagnostics: vec![],
            truncated: false,
        }
    }
    fn error(&mut self, row: usize, column: &str, reason: impl ToString) {
        if self.diagnostics.len() < MAX_DIAGNOSTICS {
            self.diagnostics.push(CsvDiagnostic {
                row,
                column: column.into(),
                reason: reason.to_string(),
            });
        } else {
            self.truncated = true;
        }
    }
    pub fn into_result(self) -> Result<T> {
        if let Some(value) = self.value {
            return Ok(value);
        }
        let mut reasons = self
            .diagnostics
            .iter()
            .map(|d| format!("ردیف {}، ستون {}: {}", d.row, d.column, d.reason))
            .collect::<Vec<_>>()
            .join("\n");
        if self.truncated {
            reasons.push_str("\nخطاهای بیشتر نمایش داده نشد؛ پس از اصلاح دوباره بررسی کنید");
        }
        Err(anyhow::anyhow!(reasons))
    }
}
pub fn catalog_csv(input: &str, name: &str) -> Result<Catalog> {
    catalog_csv_report(input, name).into_result()
}
pub fn catalog_csv_report(input: &str, name: &str) -> CsvReport<Catalog> {
    let mut report = CsvReport::new();
    if input.len() > MAX_IMPORT {
        report.error(0, "file", "فایل بیش از ۱۰ مگابایت است");
        return report;
    }
    if let Err(e) = text(name) {
        report.error(0, "name", e);
    }
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(input.trim_start_matches('\u{feff}').as_bytes());
    match rdr.headers() {
        Ok(headers)
            if headers.iter().eq([
                "code",
                "name",
                "measure",
                "price",
                "currency",
                "source_at",
                "aliases",
            ]) => {}
        Ok(_) => {
            report.error(
                1,
                "header",
                "سرستون‌ها باید code,name,measure,price,currency,source_at,aliases باشند",
            );
            return report;
        }
        Err(e) => {
            report.error(1, "header", e);
            return report;
        }
    }
    let mut products = vec![];
    let mut codes = HashSet::new();
    for (index, row) in rdr.records().enumerate() {
        if index >= 50_000 {
            report.error(index + 2, "row", "حداکثر ۵۰ هزار کالا");
            break;
        }
        let row = match row {
            Ok(row) => row,
            Err(e) => {
                report.error(index + 2, "row", e);
                if report.diagnostics.len() >= MAX_DIAGNOSTICS {
                    report.truncated = true;
                    break;
                }
                continue;
            }
        };
        let line = row
            .position()
            .map_or(index as u64 + 2, |position| position.line()) as usize;
        let before = report.diagnostics.len();
        for (column, value) in [("code", &row[0]), ("name", &row[1]), ("measure", &row[2])] {
            if let Err(e) = text(value) {
                report.error(line, column, e);
            }
        }
        if !codes.insert(normalize(&row[0])) {
            report.error(line, "code", "کد تکراری");
        }
        let unit = match Unit::parse(&row[4]) {
            Ok(unit) => Some(unit),
            Err(e) => {
                report.error(line, "currency", e);
                None
            }
        };
        let price = unit.and_then(|unit| match unit.amount(&row[3]) {
            Ok(price) => Some(price),
            Err(e) => {
                report.error(line, "price", e);
                None
            }
        });
        if let Err(e) = timestamp(&row[5]) {
            report.error(line, "source_at", e);
        }
        let mut aliases = vec![];
        let mut alias_keys = HashSet::new();
        for alias in row[6].split('|').filter(|a| !a.trim().is_empty()) {
            if let Err(e) = text(alias) {
                report.error(line, "aliases", e);
            } else {
                let key = normalize(alias);
                if alias_keys.insert(key.clone()) {
                    aliases.push(key);
                }
            }
            if aliases.len() > 1000 || report.diagnostics.len() >= MAX_DIAGNOSTICS {
                break;
            }
        }
        aliases.sort();
        aliases.dedup();
        if aliases.len() > 1000 {
            report.error(line, "aliases", "حداکثر ۱۰۰۰ نام جایگزین برای هر کالا");
        }
        if report.diagnostics.len() == before
            && let (Some(source_unit), Some(price_rial)) = (unit, price)
        {
            products.push(Product {
                code: digits(&row[0]).to_uppercase(),
                name: row[1].into(),
                measure: row[2].into(),
                price_rial,
                source_unit,
                source_at: row[5].into(),
                aliases,
            });
        }
        if report.diagnostics.len() >= MAX_DIAGNOSTICS {
            report.truncated = true;
            break;
        }
    }
    if products.is_empty() && report.diagnostics.is_empty() {
        report.error(2, "row", "فهرست خالی است");
    }
    if report.diagnostics.is_empty() {
        match product_fingerprint(&products) {
            Ok(fingerprint) => {
                report.value = Some(Catalog {
                    id: id(),
                    name: name.trim().into(),
                    imported_at: now(),
                    fingerprint,
                    products,
                })
            }
            Err(e) => report.error(0, "file", e),
        }
    }
    report
}
pub fn accept_catalog(state: &mut State, catalog: Catalog) -> Result<String> {
    let key = equivalent_fingerprint(&catalog.products)?;
    ensure!(
        !state
            .catalogs
            .iter()
            .any(|c| c.fingerprint == catalog.fingerprint),
        "این فهرست قبلاً وارد شده است"
    );
    for previous in &state.catalogs {
        ensure!(
            equivalent_fingerprint(&previous.products)? != key,
            "این فهرست با همین قیمت‌ها و منبع قبلاً وارد شده است"
        );
    }
    // Import approval is failure-atomic even for callers outside Store::change.
    let mut candidate = state.clone();
    let id = catalog.id.clone();
    candidate.catalogs.push(catalog);
    candidate.draft.catalog_id = id.clone();
    candidate
        .draft
        .lines
        .iter_mut()
        .for_each(|l| l.chosen_code = None);
    candidate.draft.stale_ack = false;
    candidate.validate()?;
    *state = candidate;
    Ok(id)
}
/// CSV item,quantity; repeated codes remain separate customer lines.
pub fn order_csv(input: &str) -> Result<Vec<OrderLine>> {
    order_csv_report(input).into_result()
}
pub fn order_csv_report(input: &str) -> CsvReport<Vec<OrderLine>> {
    let mut report = CsvReport::new();
    if input.len() > MAX_IMPORT {
        report.error(0, "file", "فایل بیش از ۱۰ مگابایت است");
        return report;
    }
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(input.trim_start_matches('\u{feff}').as_bytes());
    match rdr.headers() {
        Ok(headers) if headers.iter().eq(["item", "quantity"]) => {}
        Ok(_) => {
            report.error(1, "header", "سرستون سفارش باید item,quantity باشد");
            return report;
        }
        Err(e) => {
            report.error(1, "header", e);
            return report;
        }
    }
    let mut lines = vec![];
    for (index, row) in rdr.records().enumerate() {
        if index >= MAX_QUOTE_LINES {
            report.error(index + 2, "row", "حداکثر ۱۰ هزار ردیف");
            break;
        }
        let row = match row {
            Ok(row) => row,
            Err(e) => {
                report.error(index + 2, "row", e);
                if report.diagnostics.len() >= MAX_DIAGNOSTICS {
                    report.truncated = true;
                    break;
                }
                continue;
            }
        };
        let line = row
            .position()
            .map_or(index as u64 + 2, |position| position.line()) as usize;
        let before = report.diagnostics.len();
        if let Err(e) = text(&row[0]) {
            report.error(line, "item", e);
        }
        let qty = match quantity(&row[1]) {
            Ok(qty) => Some(qty),
            Err(e) => {
                report.error(line, "quantity", e);
                None
            }
        };
        if report.diagnostics.len() == before
            && let Some(quantity_milli) = qty
        {
            lines.push(OrderLine {
                query: row[0].into(),
                quantity_milli,
                chosen_code: None,
            });
        }
        if report.diagnostics.len() >= MAX_DIAGNOSTICS {
            report.truncated = true;
            break;
        }
    }
    if lines.is_empty() && report.diagnostics.is_empty() {
        report.error(2, "row", "سفارش خالی است");
    }
    if report.diagnostics.is_empty() {
        report.value = Some(lines);
    }
    report
}
