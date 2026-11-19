//! SQL Server migration importer for legacy Iranian accounting books (Holoo, Sepidar, Hesabfa, generic T-SQL & CSV).
//! Parsing is bounded, read-only, and never mutates books until explicitly posted.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Result, bail, ensure};
use serde::Serialize;

use crate::accounting::{Money, MoneyUnit};
use crate::contact_profile::PartyProfile;
use crate::core::{CoreItem, CoreParty, CoreUnit};
use crate::domain::{State, digits};
use crate::procurement::{ProcurementCommand, ProcurementContext, RegisterResources};
use crate::product_profile::ProductProfile;

/// Maximum input size (32 MiB) accepted for a single SQL/CSV import file.
pub const MAX_SQL_IMPORT_BYTES: usize = 32 * 1024 * 1024;
const MAX_WARNINGS: usize = 25;

#[derive(Clone, Debug, Serialize)]
pub struct ImportedContact {
    pub code: String,
    pub name: String,
    pub customer: bool,
    pub supplier: bool,
    pub phone: String,
    pub mobile: String,
    pub address: String,
    pub national_id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImportedItem {
    pub code: String,
    pub name: String,
    pub unit_name: String,
    pub sale_price_rials: Option<i64>,
    pub purchase_price_rials: Option<i64>,
    pub barcode: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SqlImportPlan {
    pub detected_format: String,
    pub contacts: Vec<ImportedContact>,
    pub items: Vec<ImportedItem>,
    pub units: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SqlImportPreview {
    pub detected_format: String,
    pub contacts_count: usize,
    pub items_count: usize,
    pub units_count: usize,
    pub existing_contacts_skipped: usize,
    pub existing_items_skipped: usize,
    pub sample_contacts: Vec<String>,
    pub sample_items: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SqlImportResult {
    pub detected_format: String,
    pub imported_contacts: usize,
    pub imported_items: usize,
    pub imported_units: usize,
    pub skipped_duplicates: usize,
}

/// Read a SQL Server export file (`.sql` or `.csv`) from disk with UTF-8 / UTF-16LE / UTF-16BE BOM auto-detection.
/// Rejects raw binary `.bak` files with an actionable explanation.
pub fn read_sql_export_file(path: &Path) -> Result<String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let meta = std::fs::metadata(path).map_err(|e| anyhow::anyhow!("Cannot open import file: {e}"))?;
    ensure!(meta.is_file(), "Import path is not a file");
    ensure!(
        meta.len() as usize <= MAX_SQL_IMPORT_BYTES,
        "Import file exceeds the 32 MiB limit. Export contacts and products in smaller .sql or .csv batches."
    );
    let bytes = std::fs::read(path).map_err(|e| anyhow::anyhow!("Cannot read import file: {e}"))?;
    if ext == "bak" || bytes.starts_with(b"TAPE") {
        bail!(
            "Raw SQL Server .bak files are binary backups. Restore the .bak in SQL Server Management Studio (SSMS) and run Tasks -> Generate Scripts (Data only) to save a .sql file, or export tables to .csv, then import that file here."
        );
    }
    decode_text_bytes(&bytes)
}

/// Decode UTF-8, UTF-8 BOM, UTF-16LE BOM (SSMS default), or UTF-16BE BOM into a Rust `String`.
pub fn decode_text_bytes(bytes: &[u8]) -> Result<String> {
    if bytes.starts_with(b"TAPE") {
        bail!(
            "Raw SQL Server .bak files are binary backups. Export data as a .sql script (SSMS Generate Scripts -> Data only) or .csv first."
        );
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..].as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
        return Ok(String::from_utf16_lossy(&units));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = bytes[2..].as_chunks::<2>().0.iter().map(|&c| u16::from_be_bytes(c)).collect();
        return Ok(String::from_utf16_lossy(&units));
    }
    let slice = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    String::from_utf8(slice.to_vec())
        .map_err(|_| anyhow::anyhow!("File is not valid UTF-8 or UTF-16 text. Re-save as UTF-8 or Unicode .sql/.csv."))
}

/// Parse T-SQL `INSERT` script or CSV text into a validated `SqlImportPlan`.
pub fn parse_sql_or_csv(text: &str, amount_unit: MoneyUnit) -> Result<SqlImportPlan> {
    ensure!(!text.trim().is_empty(), "Import file is empty");
    let upper = text.to_ascii_uppercase();
    if upper.contains("INSERT INTO") || upper.contains("INSERT [") {
        parse_tsql_script(text, amount_unit)
    } else {
        parse_csv_text(text, amount_unit)
    }
}

fn push_warning(warnings: &mut Vec<String>, msg: impl Into<String>) {
    if warnings.len() < MAX_WARNINGS {
        warnings.push(msg.into());
    }
}

fn clean_field(raw: &str, max_chars: usize) -> String {
    let cleaned: String = raw.trim().chars().filter(|c| !c.is_control()).take(max_chars).collect();
    cleaned.trim().to_string()
}

fn parse_optional_money(raw: &str, unit: MoneyUnit) -> Option<i64> {
    let s = digits(raw).trim().replace(',', "");
    if s.is_empty() || s == "0" || s == "0.0" || s == "0.00" {
        return None;
    }
    // Strip trailing `.00` from SQL decimal literals
    let normalized = if let Some((whole, frac)) = s.split_once('.') {
        if frac.chars().all(|c| c == '0') { whole } else { s.as_str() }
    } else {
        s.as_str()
    };
    Money::parse(normalized, unit).ok().map(|m| m.rials()).filter(|&r| r > 0)
}

fn strip_ident(raw: &str) -> String {
    let mut s = raw.trim();
    if let Some(inner) = s.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        s = inner;
    } else if let Some(inner) = s.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        s = inner;
    } else if let Some(inner) = s.strip_prefix('`').and_then(|v| v.strip_suffix('`')) {
        s = inner;
    }
    s.trim().to_string()
}

fn normalize_table_name(raw: &str) -> String {
    let parts: Vec<String> = raw.split('.').map(strip_ident).filter(|p| !p.is_empty()).collect();
    if parts.len() >= 2 && !parts[parts.len() - 2].eq_ignore_ascii_case("dbo") {
        format!(
            "{}.{}",
            parts[parts.len() - 2].to_ascii_lowercase(),
            parts[parts.len() - 1].to_ascii_lowercase()
        )
    } else {
        parts.last().cloned().unwrap_or_default().to_ascii_lowercase()
    }
}

fn is_contact_table(table: &str) -> bool {
    matches!(
        table,
        "customer"
            | "customers"
            | "c_code"
            | "ashkhas"
            | "party"
            | "parties"
            | "gnr.party"
            | "contact"
            | "contacts"
            | "supplier"
            | "suppliers"
            | "vendor"
            | "vendors"
            | "person"
            | "persons"
            | "dl"
    )
}

fn is_item_table(table: &str) -> bool {
    matches!(
        table,
        "article" | "a_code" | "kala" | "item" | "items" | "inv.item" | "product" | "products" | "goods" | "merchandise"
    )
}

fn detect_format(tables: &HashSet<String>) -> String {
    if tables
        .iter()
        .any(|t| matches!(t.as_str(), "article" | "a_code" | "c_code" | "customer"))
    {
        "holoo".to_string()
    } else if tables
        .iter()
        .any(|t| t.starts_with("gnr.") || t.starts_with("inv.") || t.starts_with("acc."))
    {
        "sepidar".to_string()
    } else {
        "generic_sql".to_string()
    }
}

/// Strip `--` line comments and `/* ... */` block comments outside of single-quoted SQL strings.
fn strip_sql_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut in_str = false;
    while i < chars.len() {
        let c = chars[i];
        if in_str {
            out.push(c);
            if c == '\'' {
                if i + 1 < chars.len() && chars[i + 1] == '\'' {
                    out.push('\'');
                    i += 2;
                    continue;
                }
                in_str = false;
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            in_str = true;
            out.push(c);
            i += 1;
        } else if c == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(chars.len());
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Parse one or more parenthesized tuples `(a, N'b', NULL), (c, N'd', 100)` after `VALUES`.
fn parse_value_tuples(input: &str) -> Vec<Vec<String>> {
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut tuples = Vec::new();
    while i < chars.len() {
        while i < chars.len() && chars[i] != '(' {
            if chars[i] == ';' {
                return tuples;
            }
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        i += 1; // skip '('
        let mut row = Vec::new();
        let mut cur = String::new();
        let mut depth = 0usize;
        let mut in_str = false;
        while i < chars.len() {
            let c = chars[i];
            if in_str {
                if c == '\'' {
                    if i + 1 < chars.len() && chars[i + 1] == '\'' {
                        cur.push('\'');
                        i += 2;
                        continue;
                    }
                    in_str = false;
                } else {
                    cur.push(c);
                }
                i += 1;
                continue;
            }
            match c {
                '\'' => {
                    // Strip leading `N` from SQL Server `N'...'` Unicode literal
                    if cur.trim() == "N" || cur.trim() == "n" {
                        cur.clear();
                    }
                    in_str = true;
                    i += 1;
                }
                '(' => {
                    depth += 1;
                    cur.push(c);
                    i += 1;
                }
                ')' if depth > 0 => {
                    depth -= 1;
                    cur.push(c);
                    i += 1;
                }
                ',' if depth == 0 => {
                    row.push(normalize_sql_value(&cur));
                    cur.clear();
                    i += 1;
                }
                ')' if depth == 0 => {
                    row.push(normalize_sql_value(&cur));
                    i += 1;
                    break;
                }
                _ => {
                    cur.push(c);
                    i += 1;
                }
            }
        }
        if !row.is_empty() {
            tuples.push(row);
        }
    }
    tuples
}

fn normalize_sql_value(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("NULL") {
        return String::new();
    }
    // Unwrap simple `CAST(N'123' AS Decimal(...))` or `CAST(123 AS ...)`
    if let Some(rest) = trimmed.strip_prefix("CAST(").or_else(|| trimmed.strip_prefix("cast("))
        && let Some((val, _)) = rest.split_once(" AS ").or_else(|| rest.split_once(" as "))
    {
        return val.trim().trim_start_matches(['N', 'n']).trim_matches('\'').to_string();
    }
    trimmed.to_string()
}

fn find_col<'a>(cols: &[String], row: &'a [String], names: &[&str]) -> &'a str {
    for target in names {
        if let Some(pos) = cols.iter().position(|c| c.eq_ignore_ascii_case(target))
            && let Some(val) = row.get(pos)
            && !val.trim().is_empty()
        {
            return val.trim();
        }
    }
    ""
}

fn map_contact_row(cols: &[String], row: &[String], table: &str) -> Option<ImportedContact> {
    let name = clean_field(
        find_col(
            cols,
            row,
            &[
                "c_name",
                "name",
                "title",
                "fullname",
                "full_name",
                "partyname",
                "customername",
                "dltitle",
                "نام",
                "نام طرف حساب",
            ],
        ),
        160,
    );
    if name.is_empty() {
        return None;
    }
    let code = clean_field(find_col(cols, row, &["c_code", "code", "id", "partyid", "number", "کد"]), 60);
    let phone = clean_field(find_col(cols, row, &["tel", "phone", "telephone", "c_tel", "تلفن"]), 80);
    let mobile = clean_field(find_col(cols, row, &["mobile", "cell", "c_mobile", "موبایل", "همراه"]), 80);
    let address = clean_field(find_col(cols, row, &["address", "c_address", "آدرس", "نشانی"]), 500);
    let national_id = clean_field(
        find_col(cols, row, &["nationalid", "national_id", "mellicode", "economiccode", "کد ملی"]),
        40,
    );
    let is_cust_col = find_col(cols, row, &["iscustomer", "is_customer", "customer", "kharidar"]);
    let is_supp_col = find_col(
        cols,
        row,
        &["issupplier", "is_supplier", "isvendor", "supplier", "vendor", "forooshandeh"],
    );
    let (customer, supplier) = if !is_cust_col.is_empty() || !is_supp_col.is_empty() {
        let c = matches!(is_cust_col.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "بله");
        let s = matches!(is_supp_col.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "بله");
        if !c && !s { (true, false) } else { (c, s) }
    } else if matches!(table, "supplier" | "suppliers" | "vendor" | "vendors") {
        (false, true)
    } else {
        (true, false)
    };
    Some(ImportedContact {
        code,
        name,
        customer,
        supplier,
        phone,
        mobile,
        address,
        national_id,
    })
}

fn map_item_row(cols: &[String], row: &[String], amount_unit: MoneyUnit) -> Option<ImportedItem> {
    let name = clean_field(
        find_col(
            cols,
            row,
            &["a_name", "name", "title", "itemname", "productname", "kalaname", "نام", "نام کالا"],
        ),
        160,
    );
    if name.is_empty() {
        return None;
    }
    let code = clean_field(find_col(cols, row, &["a_code", "code", "sku", "id", "itemid", "کد", "کد کالا"]), 60);
    let unit_name = clean_field(
        find_col(cols, row, &["unit", "unitname", "unit_name", "vahed", "a_unit", "واحد"]),
        60,
    );
    let sale_raw = find_col(
        cols,
        row,
        &[
            "sel_price",
            "sellprice",
            "sell_price",
            "saleprice",
            "sale_price",
            "price",
            "fee",
            "قیمت فروش",
            "قیمت",
        ],
    );
    let buy_raw = find_col(
        cols,
        row,
        &["buy_price", "buyprice", "purchase_price", "cost", "lastprice", "قیمت خرید"],
    );
    let barcode = clean_field(find_col(cols, row, &["barcode", "a_barcode", "بارکد"]), 80);
    Some(ImportedItem {
        code,
        name,
        unit_name,
        sale_price_rials: parse_optional_money(sale_raw, amount_unit),
        purchase_price_rials: parse_optional_money(buy_raw, amount_unit),
        barcode,
    })
}

fn parse_tsql_script(raw: &str, amount_unit: MoneyUnit) -> Result<SqlImportPlan> {
    let cleaned = strip_sql_comments(raw);
    let mut contacts = Vec::new();
    let mut items = Vec::new();
    let mut units_set = HashSet::new();
    let mut units = Vec::new();
    let mut tables_seen = HashSet::new();
    let mut warnings = Vec::new();

    // Scan for `INSERT` statements case-insensitively
    let lower = cleaned.to_ascii_lowercase();
    let mut search_from = 0usize;
    while let Some(rel) = lower[search_from..].find("insert ") {
        let start = search_from + rel;
        let rest = &cleaned[start + 7..];
        let rest_trimmed = rest.trim_start();
        let after_into = if rest_trimmed.len() >= 5 && rest_trimmed[..5].eq_ignore_ascii_case("into ") {
            rest_trimmed[5..].trim_start()
        } else {
            rest_trimmed
        };
        let Some(open_cols) = after_into.find('(') else {
            search_from = start + 7;
            continue;
        };
        let table_raw = after_into[..open_cols].trim();
        let table = normalize_table_name(table_raw);
        let after_open = &after_into[open_cols + 1..];
        let Some(close_cols) = after_open.find(')') else {
            search_from = start + 7;
            continue;
        };
        let cols: Vec<String> = after_open[..close_cols].split(',').map(strip_ident).collect();
        let after_cols = &after_open[close_cols + 1..];
        let after_cols_lower = after_cols.to_ascii_lowercase();
        let Some(val_pos) = after_cols_lower.find("values") else {
            search_from = start + 7;
            continue;
        };
        let values_slice = &after_cols[val_pos + 6..];
        // Bound statement at next `\nGO` or next `\nINSERT ` or `;`
        let stmt_end = values_slice
            .find("\nINSERT ")
            .or_else(|| values_slice.find("\ninsert "))
            .or_else(|| values_slice.find("\nGO"))
            .unwrap_or(values_slice.len());
        let tuples = parse_value_tuples(&values_slice[..stmt_end]);

        tables_seen.insert(table.clone());
        if is_contact_table(&table) {
            for row in &tuples {
                if let Some(c) = map_contact_row(&cols, row, &table) {
                    contacts.push(c);
                }
            }
        } else if is_item_table(&table) {
            for row in &tuples {
                if let Some(it) = map_item_row(&cols, row, amount_unit) {
                    if !it.unit_name.is_empty() && units_set.insert(it.unit_name.clone()) {
                        units.push(it.unit_name.clone());
                    }
                    items.push(it);
                }
            }
        }
        search_from = start + 7;
    }

    if contacts.is_empty() && items.is_empty() {
        push_warning(
            &mut warnings,
            format!(
                "No recognized contact or product tables found (saw {} table(s)). Supported tables include CUSTOMER/ARTICLE (Holoo), GNR.Party/INV.Item (Sepidar), CONTACTS/PRODUCTS.",
                tables_seen.len()
            ),
        );
    }

    Ok(SqlImportPlan {
        detected_format: detect_format(&tables_seen),
        contacts,
        items,
        units,
        warnings,
    })
}

fn parse_csv_text(raw: &str, amount_unit: MoneyUnit) -> Result<SqlImportPlan> {
    let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(raw.as_bytes());
    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| anyhow::anyhow!("Invalid CSV header: {e}"))?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();
    ensure!(!headers.is_empty(), "CSV file has no headers");

    let lower_headers: Vec<String> = headers.iter().map(|h| h.to_ascii_lowercase()).collect();
    let looks_like_item = lower_headers.iter().any(|h| {
        matches!(
            h.as_str(),
            "sku"
                | "a_code"
                | "a_name"
                | "sellprice"
                | "sell_price"
                | "sale_price"
                | "price"
                | "barcode"
                | "unit"
                | "قیمت"
                | "قیمت فروش"
                | "نام کالا"
        )
    });

    let mut contacts = Vec::new();
    let mut items = Vec::new();
    let mut units_set = HashSet::new();
    let mut units = Vec::new();
    let mut warnings = Vec::new();

    for (idx, rec) in rdr.records().enumerate() {
        let rec = match rec {
            Ok(r) => r,
            Err(e) => {
                push_warning(&mut warnings, format!("Skipped CSV row {}: {e}", idx + 2));
                continue;
            }
        };
        let row: Vec<String> = rec.iter().map(|s| s.to_string()).collect();
        if looks_like_item {
            if let Some(it) = map_item_row(&headers, &row, amount_unit) {
                if !it.unit_name.is_empty() && units_set.insert(it.unit_name.clone()) {
                    units.push(it.unit_name.clone());
                }
                items.push(it);
            }
        } else if let Some(c) = map_contact_row(&headers, &row, "contacts") {
            contacts.push(c);
        }
    }

    if contacts.is_empty() && items.is_empty() {
        push_warning(&mut warnings, "No contacts or products could be mapped from the CSV columns.");
    }

    Ok(SqlImportPlan {
        detected_format: "csv".to_string(),
        contacts,
        items,
        units,
        warnings,
    })
}

/// Preview what a `SqlImportPlan` would add to `company_id` without mutating state.
pub fn preview_plan(state: &State, company_id: &str, plan: &SqlImportPlan) -> Result<SqlImportPreview> {
    let resources = state.accounting.procurement_resources(company_id)?;
    let existing_parties: HashSet<String> = resources.parties.iter().map(|p| p.name.trim().to_lowercase()).collect();
    let existing_items: HashSet<String> = resources.items.iter().map(|i| i.name.trim().to_lowercase()).collect();

    let mut existing_contacts_skipped = 0usize;
    let mut new_contacts = Vec::new();
    for c in &plan.contacts {
        if existing_parties.contains(&c.name.trim().to_lowercase()) {
            existing_contacts_skipped += 1;
        } else {
            new_contacts.push(c.name.clone());
        }
    }

    let mut existing_items_skipped = 0usize;
    let mut new_items = Vec::new();
    for it in &plan.items {
        if existing_items.contains(&it.name.trim().to_lowercase()) {
            existing_items_skipped += 1;
        } else {
            new_items.push(it.name.clone());
        }
    }

    Ok(SqlImportPreview {
        detected_format: plan.detected_format.clone(),
        contacts_count: new_contacts.len(),
        items_count: new_items.len(),
        units_count: plan.units.len(),
        existing_contacts_skipped,
        existing_items_skipped,
        sample_contacts: new_contacts.into_iter().take(5).collect(),
        sample_items: new_items.into_iter().take(5).collect(),
        warnings: plan.warnings.clone(),
    })
}

/// Apply a `SqlImportPlan` atomically inside `store.change(...)`, skipping existing names for idempotency.
pub fn apply_plan(state: &mut State, ctx: ProcurementContext, plan: &SqlImportPlan) -> Result<SqlImportResult> {
    let company_id = ctx.company_id.clone();
    let resources = state.accounting.procurement_resources(&company_id)?;

    let mut existing_parties: HashSet<String> = resources.parties.iter().map(|p| p.name.trim().to_lowercase()).collect();
    let mut existing_items: HashSet<String> = resources.items.iter().map(|i| i.name.trim().to_lowercase()).collect();
    let mut unit_by_name: HashMap<String, String> = resources
        .units
        .iter()
        .map(|u| (u.name.trim().to_lowercase(), u.id.clone()))
        .collect();

    let mut cmd = RegisterResources {
        context: ctx,
        parties: Vec::new(),
        units: Vec::new(),
        items: Vec::new(),
        warehouses: Vec::new(),
    };

    let mut new_party_profiles: Vec<(String, PartyProfile)> = Vec::new();
    let mut new_item_profiles: Vec<(String, ProductProfile)> = Vec::new();
    let mut skipped_duplicates = 0usize;

    // 1. Ensure default unit + any imported unit names
    for uname in &plan.units {
        let key = uname.trim().to_lowercase();
        if !key.is_empty() && !unit_by_name.contains_key(&key) {
            let uid = format!("sql-unit-{}", uuid::Uuid::new_v4().simple());
            unit_by_name.insert(key, uid.clone());
            cmd.units.push(CoreUnit {
                id: uid,
                company_id: company_id.clone(),
                name: uname.clone(),
            });
        }
    }
    let default_unit_id = if let Some(u) = resources.units.first() {
        u.id.clone()
    } else if let Some(u) = cmd.units.first() {
        u.id.clone()
    } else {
        let uid = format!("sql-unit-{}", uuid::Uuid::new_v4().simple());
        cmd.units.push(CoreUnit {
            id: uid.clone(),
            company_id: company_id.clone(),
            name: "عدد".to_string(),
        });
        uid
    };

    // 2. Contacts / Parties
    for c in &plan.contacts {
        let key = c.name.trim().to_lowercase();
        if !existing_parties.insert(key) {
            skipped_duplicates += 1;
            continue;
        }
        let pid = format!("sql-party-{}", uuid::Uuid::new_v4().simple());
        cmd.parties.push(CoreParty {
            id: pid.clone(),
            company_id: company_id.clone(),
            name: c.name.clone(),
            customer: c.customer,
            supplier: c.supplier,
        });
        let profile = PartyProfile {
            company_id: company_id.clone(),
            display_name: c.name.clone(),
            phone: c.phone.clone(),
            mobile: c.mobile.clone(),
            address: c.address.clone(),
            national_id: c.national_id.clone(),
            ..Default::default()
        };
        if profile.validate().is_ok() {
            new_party_profiles.push((pid, profile));
        }
    }

    // 3. Products / Items
    for it in &plan.items {
        let key = it.name.trim().to_lowercase();
        if !existing_items.insert(key) {
            skipped_duplicates += 1;
            continue;
        }
        let iid = format!("sql-item-{}", uuid::Uuid::new_v4().simple());
        let unit_id = unit_by_name
            .get(&it.unit_name.trim().to_lowercase())
            .cloned()
            .unwrap_or_else(|| default_unit_id.clone());
        cmd.items.push(CoreItem {
            id: iid.clone(),
            company_id: company_id.clone(),
            sku: if it.code.is_empty() { format!("P-{iid}") } else { it.code.clone() },
            name: it.name.clone(),
            base_unit_id: unit_id,
            packs: vec![],
            indivisible: false,
        });
        let profile = ProductProfile {
            company_id: company_id.clone(),
            sale_price: it.sale_price_rials.map(Money::from_rials),
            purchase_price: it.purchase_price_rials.map(Money::from_rials),
            barcode: if it.barcode.is_empty() {
                it.code.clone()
            } else {
                it.barcode.clone()
            },
            ..Default::default()
        };
        if profile.validate().is_ok() {
            new_item_profiles.push((iid, profile));
        }
    }

    let imported_contacts = cmd.parties.len();
    let imported_items = cmd.items.len();
    let imported_units = cmd.units.len();

    if imported_contacts > 0 || imported_items > 0 || imported_units > 0 {
        state.accounting.post_procurement_command(ProcurementCommand::Resources(cmd))?;
        for (pid, prof) in new_party_profiles {
            state.party_profiles.insert(pid, prof);
        }
        for (iid, prof) in new_item_profiles {
            state.product_profiles.insert(iid, prof);
        }
    }

    Ok(SqlImportResult {
        detected_format: plan.detected_format.clone(),
        imported_contacts,
        imported_items,
        imported_units,
        skipped_duplicates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_holoo_utf16le_tsql_script() {
        let sql = "\
-- Holoo SQL Server data script
INSERT INTO [dbo].[CUSTOMER] ([C_Code], [C_Name], [C_Mobile], [C_Address]) VALUES (N'101', N'شرکت آفتاب', N'09121112233', N'تهران');
GO
INSERT INTO [dbo].[ARTICLE] ([A_Code], [A_Name], [A_Unit], [Sel_Price], [Buy_Price]) VALUES (N'A-1', N'کابل شبکه', N'حلقه', 2500000, 1800000), (N'A-2', N'سوئیچ ۸ پورت', N'عدد', CAST(4500000 AS Decimal(18,0)), NULL);
";
        let mut utf16_bytes = vec![0xFFu8, 0xFEu8];
        for u in sql.encode_utf16() {
            utf16_bytes.extend_from_slice(&u.to_le_bytes());
        }
        let decoded = decode_text_bytes(&utf16_bytes).unwrap();
        let plan = parse_sql_or_csv(&decoded, MoneyUnit::Rial).unwrap();
        assert_eq!(plan.detected_format, "holoo");
        assert_eq!(plan.contacts.len(), 1);
        assert_eq!(plan.contacts[0].name, "شرکت آفتاب");
        assert_eq!(plan.contacts[0].mobile, "09121112233");
        assert_eq!(plan.items.len(), 2);
        assert_eq!(plan.items[0].sale_price_rials, Some(2_500_000));
        assert_eq!(plan.items[1].sale_price_rials, Some(4_500_000));
        assert_eq!(plan.units, vec!["حلقه", "عدد"]);
    }

    #[test]
    fn parses_sepidar_tsql_and_csv() {
        let sql = "INSERT INTO [GNR].[Party] ([Code], [Title], [IsCustomer], [IsVendor]) VALUES (N'P1', N'بازرگانی پارس', 1, 1);";
        let plan = parse_sql_or_csv(sql, MoneyUnit::Rial).unwrap();
        assert_eq!(plan.detected_format, "sepidar");
        assert_eq!(plan.contacts.len(), 1);
        assert!(plan.contacts[0].customer && plan.contacts[0].supplier);

        let csv = "SKU,Name,Unit,Price\nSKU-1,Mouse,Piece,120000\n";
        let csv_plan = parse_sql_or_csv(csv, MoneyUnit::Toman).unwrap();
        assert_eq!(csv_plan.detected_format, "csv");
        assert_eq!(csv_plan.items.len(), 1);
        assert_eq!(csv_plan.items[0].sale_price_rials, Some(1_200_000));
    }

    #[test]
    fn rejects_raw_bak_with_helpful_message() {
        let err = decode_text_bytes(b"TAPE\x00\x01\x02\x03").unwrap_err().to_string();
        assert!(err.contains(".bak") && err.contains("SSMS"));
    }
}
