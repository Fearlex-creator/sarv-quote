//! Real `.xlsx` templates, preview, commit, and export.
//!
//! Calamine reads cached cell values and does not calculate formulas or run
//! macros. This module refuses macro parts and sheets that still contain
//! formulas. Amounts are whole rials. A toman display in the app never changes
//! these columns. Commit is all-or-nothing for the workbook.
//!
//! Opening stock posts a purchase order and a full goods receipt. Inventory is
//! debited and GRNI is credited. It is not an equity journal and it is not a
//! supplier bill until someone bills that receipt.
use crate::accounting::{Money, MoneyUnit};
use crate::contact_profile::PartyProfile;
use crate::core::{CoreItem, CoreParty, CoreUnit, CoreWarehouse, positive_half_up};
use crate::domain::{State, digits, normalize, quantity, text};
use crate::procurement::{
    OrderLine, ProcurementCommand, ProcurementContext, PurchaseOrder, ReceiveLine, ReceiveOrder,
    RegisterResources,
};
use crate::product_profile::ProductProfile;
use crate::store::Store;
use anyhow::{Result, anyhow, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};
use rust_xlsxwriter::{Format, Workbook};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Cursor, Read};

pub const SCHEMA: &str = "sarv-excel-1";
const MAX_BYTES: usize = 1_000_000;
const MAX_UNCOMPRESSED: u64 = 8_000_000;
const MAX_ROWS: usize = 5_000;
const MAX_ENTRIES: usize = 64;

const PRODUCTS: &[&str] = &[
    "sku",
    "name",
    "unit",
    "barcode",
    "category",
    "sale_price_rial",
    "purchase_price_rial",
    "minimum_stock",
];
const PARTIES: &[&str] = &[
    "name",
    "phone",
    "mobile",
    "email",
    "address",
    "city",
    "postal_code",
    "national_id",
    "tax_id",
    "notes",
];
const OPENING: &[&str] = &[
    "sku",
    "name",
    "unit",
    "warehouse",
    "quantity",
    "unit_cost_rial",
];

pub fn command(store: &mut Store, action: &str, p: &Value) -> Result<Value> {
    match action {
        "excel_template" => {
            let kind = kind(p)?;
            let (filename, bytes) = template(kind)?;
            Ok(file_json(&filename, &bytes))
        }
        "excel_export" => export(store, p),
        "excel_preview" => {
            let kind = kind(p)?;
            let bytes = decode(p)?;
            Ok(wire_preview(kind, parse(&bytes, kind)?))
        }
        "excel_commit" => commit(store, p),
        "excel_graph_status" => Ok(crate::excel_graph::status(store)),
        "excel_graph_begin" => crate::excel_graph::begin(store),
        "excel_graph_complete" => crate::excel_graph::complete(store, p),
        "excel_graph_revoke" => crate::excel_graph::revoke(store),
        "excel_graph_preview" => graph_preview(store, p),
        "excel_graph_commit" => graph_commit(store, p),
        "excel_graph_push" => crate::excel_graph::push(store, p),
        "excel_graph_sync" => Err(anyhow!(
            "Name the workbook item and use excel_graph_preview, then excel_graph_commit. There is no background sync."
        )),
        _ => Err(anyhow!("Unsupported Excel command")),
    }
}

fn graph_preview(store: &Store, p: &Value) -> Result<Value> {
    let bytes = crate::excel_graph::download(store, p)?;
    let mut payload = p.clone();
    payload["content_base64"] = json!(STANDARD.encode(&bytes));
    command_preview(store, &payload)
}

fn graph_commit(store: &mut Store, p: &Value) -> Result<Value> {
    let bytes = crate::excel_graph::download(store, p)?;
    let mut payload = p.clone();
    payload["content_base64"] = json!(STANDARD.encode(&bytes));
    commit(store, &payload)
}

fn command_preview(store: &Store, p: &Value) -> Result<Value> {
    let _ = store;
    let kind = kind(p)?;
    let bytes = decode(p)?;
    let parsed = parse(&bytes, kind)?;
    Ok(wire_preview(kind, parsed))
}

fn wire_preview(kind: &str, parsed: Parsed) -> Value {
    json!({
        "dry_run": true,
        "committed": false,
        "kind": kind,
        "schema": SCHEMA,
        "sheet": parsed.sheet,
        "sheets": parsed.sheets,
        "accepted": parsed.rows.len(),
        "errors": parsed.errors,
        "warnings": parsed.warnings,
        "currency": "IRR",
        "opening_stock_effect": if kind == "opening_stock" {
            "This sheet is stock received and not yet invoiced: inventory debit, GRNI credit. It is not an opening capital balance. Owner equity is a separate entry. It does not create a supplier bill. Clear GRNI later with a supplier bill or a reviewed journal."
        } else {
            ""
        },
        "live_verified": false,
    })
}

fn commit(store: &mut Store, p: &Value) -> Result<Value> {
    ensure!(p["confirm"] == true, "Commit needs explicit confirmation");
    let kind = kind(p)?;
    let bytes = decode(p)?;
    let digest = sha(&bytes);
    let key = required(p, "request_key")?;
    text(&key)?;
    if let Some(prior) = store
        .state
        .retail
        .imports
        .iter()
        .find(|m| m.request_key == key)
    {
        ensure!(
            prior.sha256 == digest && prior.kind == kind,
            "This request key was already used for a different workbook"
        );
        let mut summary: Value = serde_json::from_str(&prior.summary)?;
        if let Some(object) = summary.as_object_mut() {
            object.insert("replayed".into(), json!(true));
        }
        return Ok(summary);
    }
    let parsed = parse(&bytes, kind)?;
    ensure!(
        parsed.errors.is_empty(),
        "Workbook has row errors. Preview it and fix them before committing."
    );
    ensure!(!parsed.rows.is_empty(), "Workbook has no data rows");
    let company = required(p, "company_id")?;
    let period = required(p, "period_id")?;
    let date = required(p, "date")?;
    crate::core::civil_date(&date)?;
    let actor = p["_actor"].as_str().unwrap_or("local-desktop").to_owned();
    let policy = p["policy"].as_str().unwrap_or("skip");
    ensure!(
        matches!(policy, "skip" | "update"),
        "Policy must be skip or update"
    );
    ensure!(key.chars().count() <= 80, "request_key is too long");
    match kind {
        "products" => commit_products(
            store, &parsed, &company, &period, &date, &actor, &key, &digest, policy,
        ),
        "customers" | "suppliers" => commit_parties(
            store, &parsed, kind, &company, &period, &date, &actor, &key, &digest, policy,
        ),
        "opening_stock" => commit_opening(
            store, &parsed, &company, &period, &date, &actor, &key, &digest,
        ),
        _ => Err(anyhow!("This sheet is export-only")),
    }
}

struct Parsed {
    sheet: String,
    sheets: Vec<String>,
    rows: Vec<BTreeMap<String, String>>,
    errors: Vec<Value>,
    warnings: Vec<String>,
}

fn parse(bytes: &[u8], kind: &str) -> Result<Parsed> {
    guard_zip(bytes)?;
    let mut book: Xlsx<_> = open_workbook_from_rs(Cursor::new(bytes.to_vec()))
        .map_err(|e| anyhow!("Could not read this workbook: {e}"))?;
    let sheets = book.sheet_names();
    let expected = headers(kind)?;
    let sheet = sheets
        .iter()
        .find(|n| n.eq_ignore_ascii_case("Data"))
        .cloned()
        .unwrap_or_else(|| sheets.first().cloned().unwrap_or_default());
    ensure!(!sheet.is_empty(), "Workbook has no sheet");
    let formulas = book
        .worksheet_formula(&sheet)
        .map_err(|e| anyhow!("Could not inspect formulas: {e}"))?;
    ensure!(
        formulas
            .rows()
            .all(|row| row.iter().all(|cell| cell.is_empty())),
        "This sheet contains formulas. Sarv does not calculate them. Paste values and import again."
    );
    let range = book
        .worksheet_range(&sheet)
        .map_err(|e| anyhow!("Could not read {sheet}: {e}"))?;
    let mut rows_iter = range.rows();
    let header = rows_iter
        .next()
        .ok_or_else(|| anyhow!("The sheet is empty"))?;
    let names: Vec<String> = header.iter().map(cell_label).collect::<Result<_>>()?;
    let mut errors = vec![];
    if names.len() != expected.len() || names.iter().zip(expected).any(|(g, w)| g != w) {
        errors.push(json!({
            "row": 1,
            "message": format!("Expected columns: {}", expected.join(", ")),
        }));
    }
    let mut rows = vec![];
    let mut warnings: Vec<String> = vec![];
    for (offset, row) in rows_iter.enumerate() {
        let number = offset + 2;
        if row.iter().all(|c| matches!(c, Data::Empty)) {
            continue;
        }
        if rows.len() >= MAX_ROWS {
            errors.push(json!({"row": number, "message": "Too many rows"}));
            break;
        }
        if names.len() != expected.len() {
            continue;
        }
        match read_row(kind, number, &names, row) {
            Ok((map, row_warnings)) => {
                warnings.extend(row_warnings);
                rows.push(map);
            }
            Err(message) => errors.push(json!({"row": number, "message": message})),
        }
    }
    let mut seen = BTreeMap::<String, usize>::new();
    let key_name = if kind == "customers" || kind == "suppliers" {
        "name"
    } else {
        "sku"
    };
    for (index, row) in rows.iter().enumerate() {
        let key = normalize(row.get(key_name).map(String::as_str).unwrap_or(""));
        if let Some(previous) = seen.insert(key, index + 2) {
            errors.push(json!({
                "row": index + 2,
                "message": format!("Duplicate {key_name} also appears on row {previous}"),
            }));
        }
    }
    Ok(Parsed {
        sheet,
        sheets,
        rows,
        errors,
        warnings,
    })
}

fn read_row(
    kind: &str,
    _number: usize,
    names: &[String],
    row: &[Data],
) -> std::result::Result<(BTreeMap<String, String>, Vec<String>), String> {
    let mut map = BTreeMap::new();
    for (index, name) in names.iter().enumerate() {
        let cell = row.get(index).unwrap_or(&Data::Empty);
        let text = match name.as_str() {
            "sku" | "barcode" | "phone" | "mobile" | "national_id" | "tax_id" | "postal_code" => {
                text_identifier(cell)?
            }
            "sale_price_rial" | "purchase_price_rial" | "unit_cost_rial" => money_text(cell)?,
            "quantity" | "minimum_stock" => quantity_text(cell)?,
            _ => plain_text(cell)?,
        };
        map.insert(name.clone(), text);
    }
    match kind {
        "products" => {
            if map["sku"].is_empty() || map["name"].is_empty() || map["unit"].is_empty() {
                return Err("sku, name, and unit are required".into());
            }
            check_label(&map["sku"])?;
            check_label(&map["name"])?;
            check_label(&map["unit"])?;
        }
        "customers" | "suppliers" => {
            if map["name"].is_empty() {
                return Err("name is required".into());
            }
            check_label(&map["name"])?;
        }
        "opening_stock" => {
            if map["sku"].is_empty()
                || map["name"].is_empty()
                || map["unit"].is_empty()
                || map["warehouse"].is_empty()
                || map["quantity"].is_empty()
                || map["unit_cost_rial"].is_empty()
            {
                return Err(
                    "sku, name, unit, warehouse, quantity, and unit_cost_rial are required".into(),
                );
            }
            check_label(&map["sku"])?;
            check_label(&map["warehouse"])?;
            if quantity(&map["quantity"]).is_err() {
                return Err(
                    "quantity must be a positive number with at most 3 decimal places".into(),
                );
            }
        }
        _ => return Err("This workbook cannot be imported".into()),
    }
    Ok((map, vec![]))
}

fn commit_products(
    store: &mut Store,
    parsed: &Parsed,
    company: &str,
    period: &str,
    date: &str,
    actor: &str,
    key: &str,
    digest: &str,
    policy: &str,
) -> Result<Value> {
    let mut created = 0u32;
    let mut updated = 0u32;
    let mut skipped = 0u32;
    store.change(|state| {
        let parties = vec![];
        let mut units = vec![];
        let mut items = vec![];
        let mut profiles: Vec<(String, ProductProfile)> = vec![];
        let existing = state.accounting.procurement_resources(company)?;
        for row in &parsed.rows {
            let sku = row["sku"].clone();
            if let Some(item) = existing
                .items
                .iter()
                .find(|i| normalize(&i.sku) == normalize(&sku))
            {
                if policy == "skip" {
                    skipped += 1;
                    continue;
                }
                let mut profile =
                    state
                        .product_profiles
                        .get(&item.id)
                        .cloned()
                        .unwrap_or(ProductProfile {
                            company_id: company.into(),
                            ..ProductProfile::default()
                        });
                fill_product(&mut profile, row)?;
                profile.validate()?;
                profiles.push((item.id.clone(), profile));
                updated += 1;
                continue;
            }
            let unit_id = ensure_unit(&existing, &mut units, company, &row["unit"])?;
            let id = stable("excel-item", &[company, &normalize(&sku)]);
            items.push(CoreItem {
                id: id.clone(),
                company_id: company.into(),
                sku,
                name: row["name"].clone(),
                base_unit_id: unit_id,
                indivisible: false,
                packs: vec![],
            });
            let mut profile = ProductProfile {
                company_id: company.into(),
                ..ProductProfile::default()
            };
            fill_product(&mut profile, row)?;
            profile.validate()?;
            profiles.push((id, profile));
            created += 1;
        }
        if !units.is_empty() || !items.is_empty() {
            state
                .accounting
                .post_procurement_command(ProcurementCommand::Resources(RegisterResources {
                    context: context(
                        state,
                        company,
                        period,
                        &format!("{key}:resources"),
                        &format!("{key}:resources"),
                        date,
                        actor,
                        "Excel product import",
                    )?,
                    parties,
                    units,
                    items,
                    warehouses: vec![],
                }))?;
        }
        for (id, profile) in profiles {
            state.product_profiles.insert(id, profile);
        }
        Ok(mark(
            state,
            key,
            digest,
            "products",
            summary("products", created, updated, skipped, &parsed.warnings),
        )?)
    })
}

fn commit_parties(
    store: &mut Store,
    parsed: &Parsed,
    kind: &str,
    company: &str,
    period: &str,
    date: &str,
    actor: &str,
    key: &str,
    digest: &str,
    policy: &str,
) -> Result<Value> {
    let customer = kind == "customers";
    let mut created = 0u32;
    let mut updated = 0u32;
    let mut skipped = 0u32;
    store.change(|state| {
        let existing = state.accounting.procurement_resources(company)?;
        let mut parties = vec![];
        let mut profiles = vec![];
        for row in &parsed.rows {
            let name = row["name"].clone();
            let matches: Vec<_> = existing
                .parties
                .iter()
                .filter(|p| {
                    normalize(&p.name) == normalize(&name)
                        && (if customer { p.customer } else { p.supplier })
                })
                .collect();
            if matches.len() > 1 {
                return Err(anyhow!("More than one contact is named {name}"));
            }
            if let Some(party) = matches.first() {
                if policy == "skip" {
                    skipped += 1;
                    continue;
                }
                let mut profile =
                    state
                        .party_profiles
                        .get(&party.id)
                        .cloned()
                        .unwrap_or(PartyProfile {
                            company_id: company.into(),
                            display_name: party.name.clone(),
                            ..PartyProfile::default()
                        });
                fill_party(&mut profile, row)?;
                profile.validate()?;
                profiles.push((party.id.clone(), profile));
                updated += 1;
                continue;
            }
            let id = stable(
                if customer {
                    "excel-customer"
                } else {
                    "excel-supplier"
                },
                &[company, &normalize(&name)],
            );
            parties.push(CoreParty {
                id: id.clone(),
                company_id: company.into(),
                name: name.clone(),
                customer,
                supplier: !customer,
            });
            let mut profile = PartyProfile {
                company_id: company.into(),
                display_name: name,
                ..PartyProfile::default()
            };
            fill_party(&mut profile, row)?;
            profile.validate()?;
            profiles.push((id, profile));
            created += 1;
        }
        if !parties.is_empty() {
            state
                .accounting
                .post_procurement_command(ProcurementCommand::Resources(RegisterResources {
                    context: context(
                        state,
                        company,
                        period,
                        &format!("{key}:parties"),
                        &format!("{key}:parties"),
                        date,
                        actor,
                        "Excel contact import",
                    )?,
                    parties,
                    units: vec![],
                    items: vec![],
                    warehouses: vec![],
                }))?;
        }
        for (id, profile) in profiles {
            state.party_profiles.insert(id, profile);
        }
        Ok(mark(
            state,
            key,
            digest,
            kind,
            summary(kind, created, updated, skipped, &parsed.warnings),
        )?)
    })
}

fn commit_opening(
    store: &mut Store,
    parsed: &Parsed,
    company: &str,
    period: &str,
    date: &str,
    actor: &str,
    key: &str,
    digest: &str,
) -> Result<Value> {
    let supplier_id = stable("excel-opening-supplier", &[company]);
    store.change(|state| {
        let existing = state.accounting.procurement_resources(company)?;
        if !existing.parties.iter().any(|p| p.id == supplier_id) {
            state
                .accounting
                .post_procurement_command(ProcurementCommand::Resources(RegisterResources {
                    context: context(
                        state,
                        company,
                        period,
                        &supplier_id,
                        &format!("excel-opening-supplier:{company}"),
                        date,
                        actor,
                        "Excel opening-stock supplier",
                    )?,
                    parties: vec![CoreParty {
                        id: supplier_id.clone(),
                        company_id: company.into(),
                        name: "Opening stock".into(),
                        customer: false,
                        supplier: true,
                    }],
                    units: vec![],
                    items: vec![],
                    warehouses: vec![],
                }))?;
        }
        let existing = state.accounting.procurement_resources(company)?;
        let mut units = vec![];
        let mut items = vec![];
        let mut warehouses = vec![];
        let mut lines = vec![];
        let mut total = Money::ZERO;
        for (index, row) in parsed.rows.iter().enumerate() {
            let unit_id = ensure_unit(&existing, &mut units, company, &row["unit"])?;
            let warehouse_id =
                ensure_warehouse(&existing, &mut warehouses, company, &row["warehouse"])?;
            let item_id = if let Some(item) = existing
                .items
                .iter()
                .chain(items.iter())
                .find(|i| normalize(&i.sku) == normalize(&row["sku"]))
            {
                item.id.clone()
            } else {
                let id = stable("excel-item", &[company, &normalize(&row["sku"])]);
                items.push(CoreItem {
                    id: id.clone(),
                    company_id: company.into(),
                    sku: row["sku"].clone(),
                    name: row["name"].clone(),
                    base_unit_id: unit_id.clone(),
                    indivisible: false,
                    packs: vec![],
                });
                let profile = ProductProfile {
                    company_id: company.into(),
                    ..ProductProfile::default()
                };
                profile.validate()?;
                state.product_profiles.entry(id.clone()).or_insert(profile);
                id
            };
            let qty = quantity(&row["quantity"])?;
            let price = Money::parse(&row["unit_cost_rial"], MoneyUnit::Rial)?;
            ensure!(price.rials() >= 0, "Cost cannot be negative");
            let gross = Money::from_rials(positive_half_up(
                i128::from(price.rials()) * i128::from(qty),
                1000,
            )?);
            total = total.checked_add(gross)?;
            lines.push(OrderLine {
                id: format!("ln-{key}-{index}"),
                item_id,
                warehouse_id,
                unit_id,
                quantity_thousandths: qty,
                unit_price: price,
                line_discount: Money::ZERO,
                price_reference: format!("excel:{key}"),
                price_date: date.into(),
                override_reason: None,
            });
        }
        if !units.is_empty() || !items.is_empty() || !warehouses.is_empty() {
            state
                .accounting
                .post_procurement_command(ProcurementCommand::Resources(RegisterResources {
                    context: context(
                        state,
                        company,
                        period,
                        &format!("{key}:resources"),
                        &format!("{key}:resources"),
                        date,
                        actor,
                        "Excel opening stock resources",
                    )?,
                    parties: vec![],
                    units,
                    items,
                    warehouses,
                }))?;
        }
        let order_id = state
            .accounting
            .post_procurement_command(ProcurementCommand::Order(PurchaseOrder {
                context: context(
                    state,
                    company,
                    period,
                    &format!("{key}:order"),
                    &format!("{key}:order"),
                    date,
                    actor,
                    "Excel opening stock order",
                )?,
                supplier_id: supplier_id.clone(),
                reference: format!("OPEN-{key}"),
                document_date: date.into(),
                lines: lines.clone(),
                document_discount: Money::ZERO,
                supplied_total: total,
            }))?;
        let receive_lines = lines
            .iter()
            .enumerate()
            .map(|(index, line)| ReceiveLine {
                id: format!("rc-{key}-{index}"),
                order_line_id: line.id.clone(),
                warehouse_id: line.warehouse_id.clone(),
                unit_id: line.unit_id.clone(),
                quantity_thousandths: line.quantity_thousandths,
            })
            .collect();
        state
            .accounting
            .post_procurement_command(ProcurementCommand::Receive(ReceiveOrder {
                context: context(
                    state,
                    company,
                    period,
                    &format!("{key}:receive"),
                    &format!("{key}:receive"),
                    date,
                    actor,
                    "Excel opening stock receipt",
                )?,
                order_id: order_id.record_id,
                reference: format!("RCV-{key}"),
                document_date: date.into(),
                lines: receive_lines,
                supplied_total: total,
            }))?;
        let mut done = summary(
            "opening_stock",
            parsed.rows.len() as u32,
            0,
            0,
            &parsed.warnings,
        );
        done["effect"] = json!("inventory_debited_grni_credited");
        done["supplier_bill"] = json!(false);
        done["equity_journal"] = json!(false);
        Ok(mark(state, key, digest, "opening_stock", done)?)
    })
}

fn mark(state: &mut State, key: &str, digest: &str, kind: &str, summary: Value) -> Result<Value> {
    state.retail.imports.push(crate::retail::ImportMark {
        request_key: key.to_owned(),
        sha256: digest.to_owned(),
        kind: kind.to_owned(),
        summary: serde_json::to_string(&summary)?,
    });
    Ok(summary)
}

fn ensure_unit(
    existing: &RegisterResources,
    units: &mut Vec<CoreUnit>,
    company: &str,
    name: &str,
) -> Result<String> {
    if let Some(unit) = existing
        .units
        .iter()
        .chain(units.iter())
        .find(|u| normalize(&u.name) == normalize(name))
    {
        return Ok(unit.id.clone());
    }
    let id = stable("excel-unit", &[company, &normalize(name)]);
    units.push(CoreUnit {
        id: id.clone(),
        company_id: company.into(),
        name: name.into(),
    });
    Ok(id)
}
fn ensure_warehouse(
    existing: &RegisterResources,
    warehouses: &mut Vec<CoreWarehouse>,
    company: &str,
    name: &str,
) -> Result<String> {
    if let Some(warehouse) = existing
        .warehouses
        .iter()
        .chain(warehouses.iter())
        .find(|w| normalize(&w.name) == normalize(name))
    {
        return Ok(warehouse.id.clone());
    }
    let id = stable("excel-warehouse", &[company, &normalize(name)]);
    warehouses.push(CoreWarehouse {
        id: id.clone(),
        company_id: company.into(),
        name: name.into(),
    });
    Ok(id)
}

fn fill_product(profile: &mut ProductProfile, row: &BTreeMap<String, String>) -> Result<()> {
    if !row["barcode"].is_empty() {
        profile.barcode = row["barcode"].clone();
    }
    if !row["category"].is_empty() {
        profile.category = row["category"].clone();
    }
    if !row["sale_price_rial"].is_empty() {
        profile.sale_price = Some(Money::parse(&row["sale_price_rial"], MoneyUnit::Rial)?);
    }
    if !row["purchase_price_rial"].is_empty() {
        profile.purchase_price = Some(Money::parse(&row["purchase_price_rial"], MoneyUnit::Rial)?);
    }
    if !row["minimum_stock"].is_empty() {
        profile.minimum_stock_thousandths = if row["minimum_stock"] == "0" {
            0
        } else {
            quantity(&row["minimum_stock"])?
        };
    }
    Ok(())
}
fn fill_party(profile: &mut PartyProfile, row: &BTreeMap<String, String>) -> Result<()> {
    profile.display_name = row["name"].clone();
    for (field, slot) in [
        ("phone", &mut profile.phone),
        ("mobile", &mut profile.mobile),
        ("email", &mut profile.email),
        ("address", &mut profile.address),
        ("city", &mut profile.city),
        ("postal_code", &mut profile.postal_code),
        ("national_id", &mut profile.national_id),
        ("tax_id", &mut profile.tax_id),
        ("notes", &mut profile.notes),
    ] {
        if !row[field].is_empty() {
            *slot = row[field].clone();
        }
    }
    Ok(())
}

fn context(
    state: &State,
    company: &str,
    period: &str,
    source: &str,
    key: &str,
    date: &str,
    actor: &str,
    reason: &str,
) -> Result<ProcurementContext> {
    ensure!(
        state.accounting.companies.iter().any(|c| c.id == company),
        "Company not found"
    );
    ensure!(
        state
            .accounting
            .periods
            .iter()
            .any(|p| p.id == period && p.company_id == company),
        "Period does not belong to company"
    );
    let setup = state
        .accounting
        .core
        .setups
        .iter()
        .find(|s| s.command.company_id == company)
        .ok_or_else(|| anyhow!("Configure the chart and opening balances first"))?;
    text(source)?;
    text(key)?;
    text(reason)?;
    Ok(ProcurementContext {
        company_id: company.into(),
        period_id: period.into(),
        setup_id: setup.id.clone(),
        source_id: source.into(),
        source_revision: 1,
        date: date.into(),
        request_key: key.into(),
        actor: actor.into(),
        reason: reason.into(),
    })
}

pub(crate) fn exported_bytes(store: &Store, p: &Value) -> Result<Vec<u8>> {
    Ok(export_pair(store, p)?.1)
}

fn export(store: &Store, p: &Value) -> Result<Value> {
    let (filename, bytes) = export_pair(store, p)?;
    Ok(file_json(&filename, &bytes))
}

fn export_pair(store: &Store, p: &Value) -> Result<(String, Vec<u8>)> {
    let kind = p["kind"].as_str().unwrap_or("products");
    let company = required(p, "company_id")?;
    ensure!(
        store
            .state
            .accounting
            .companies
            .iter()
            .any(|c| c.id == company),
        "Company not found"
    );
    let (filename, bytes) = match kind {
        "products" => export_products(&store.state, &company)?,
        "customers" | "suppliers" => export_parties(&store.state, &company, kind)?,
        "trial_balance" => export_trial(&store.state, &company, &required(p, "to")?)?,
        "sales_register" => export_sales(&store.state, &company)?,
        "opening_stock" => template("opening_stock")?,
        _ => return Err(anyhow!("Unknown export")),
    };
    Ok((filename, bytes))
}

fn export_products(state: &State, company: &str) -> Result<(String, Vec<u8>)> {
    let resources = state.accounting.procurement_resources(company)?;
    let mut rows = vec![];
    for item in &resources.items {
        let profile = state.product_profiles.get(&item.id);
        let unit = resources
            .units
            .iter()
            .find(|u| u.id == item.base_unit_id)
            .map(|u| u.name.clone())
            .unwrap_or_default();
        rows.push(vec![
            item.sku.clone(),
            item.name.clone(),
            unit,
            profile.map(|p| p.barcode.clone()).unwrap_or_default(),
            profile.map(|p| p.category.clone()).unwrap_or_default(),
            profile
                .and_then(|p| p.sale_price)
                .map(|m| m.rials().to_string())
                .unwrap_or_default(),
            profile
                .and_then(|p| p.purchase_price)
                .map(|m| m.rials().to_string())
                .unwrap_or_default(),
            profile
                .map(|p| format_qty(p.minimum_stock_thousandths))
                .unwrap_or_else(|| "0".into()),
        ]);
    }
    workbook("products", PRODUCTS, &rows, "products.xlsx")
}
fn export_parties(state: &State, company: &str, kind: &str) -> Result<(String, Vec<u8>)> {
    let resources = state.accounting.procurement_resources(company)?;
    let mut rows = vec![];
    for party in resources.parties.iter().filter(|p| {
        if kind == "customers" {
            p.customer
        } else {
            p.supplier
        }
    }) {
        let profile = state.party_profiles.get(&party.id);
        rows.push(vec![
            party.name.clone(),
            profile.map(|p| p.phone.clone()).unwrap_or_default(),
            profile.map(|p| p.mobile.clone()).unwrap_or_default(),
            profile.map(|p| p.email.clone()).unwrap_or_default(),
            profile.map(|p| p.address.clone()).unwrap_or_default(),
            profile.map(|p| p.city.clone()).unwrap_or_default(),
            profile.map(|p| p.postal_code.clone()).unwrap_or_default(),
            profile.map(|p| p.national_id.clone()).unwrap_or_default(),
            profile.map(|p| p.tax_id.clone()).unwrap_or_default(),
            profile.map(|p| p.notes.clone()).unwrap_or_default(),
        ]);
    }
    workbook(kind, PARTIES, &rows, &format!("{kind}.xlsx"))
}
fn export_trial(state: &State, company: &str, to: &str) -> Result<(String, Vec<u8>)> {
    crate::core::civil_date(to)?;
    let trial = state.accounting.trial_balance(company, to)?;
    let headers = ["code", "name", "debit_rial", "credit_rial"];
    let rows: Vec<Vec<String>> = trial
        .rows
        .iter()
        .map(|r| {
            vec![
                r.code.clone(),
                r.name.clone(),
                r.debit.rials().to_string(),
                r.credit.rials().to_string(),
            ]
        })
        .collect();
    let mut extra = rows;
    extra.push(vec![
        "".into(),
        "total".into(),
        trial.total_debit.rials().to_string(),
        trial.total_credit.rials().to_string(),
    ]);
    workbook(
        "trial",
        &headers,
        &extra,
        &format!("trial-balance-{to}.xlsx"),
    )
}
fn export_sales(state: &State, company: &str) -> Result<(String, Vec<u8>)> {
    let headers = [
        "date",
        "reference",
        "customer",
        "invoice_id",
        "total_rial",
        "cost_rial",
    ];
    let mut rows = vec![];
    for record in &state.accounting.sales.records {
        if record.command.context().company_id != company {
            continue;
        }
        if let crate::sales::SalesData::Fulfill(data) = &record.data {
            rows.push(vec![
                record.command.context().date.clone(),
                record.command.context().source_id.clone(),
                data.customer.name.clone(),
                data.invoice_id.clone(),
                data.total.rials().to_string(),
                data.cost.rials().to_string(),
            ]);
        }
    }
    workbook("sales", &headers, &rows, "sales-register.xlsx")
}

fn template(kind: &str) -> Result<(String, Vec<u8>)> {
    let headers = headers(kind)?;
    let sample: Vec<Vec<String>> = match kind {
        "products" => vec![vec![
            "00123".into(),
            "Sample tea".into(),
            "Piece".into(),
            "6260000000012".into(),
            "Grocery".into(),
            "250000".into(),
            "180000".into(),
            "2".into(),
        ]],
        "customers" | "suppliers" => vec![vec![
            "Sample contact".into(),
            "02100000000".into(),
            "09120000000".into(),
            "sample@example.com".into(),
            "Example street".into(),
            "Tehran".into(),
            "1234567890".into(),
            "".into(),
            "".into(),
            "Synthetic sample. Delete this row.".into(),
        ]],
        "opening_stock" => vec![vec![
            "00123".into(),
            "Sample tea".into(),
            "Piece".into(),
            "Main".into(),
            "2".into(),
            "180000".into(),
        ]],
        _ => vec![],
    };
    workbook(
        kind,
        headers,
        &sample,
        &format!("sarv-{kind}-template.xlsx"),
    )
}

fn workbook(
    kind: &str,
    headers: &[&str],
    rows: &[Vec<String>],
    filename: &str,
) -> Result<(String, Vec<u8>)> {
    let mut book = Workbook::new();
    let text = Format::new().set_num_format("@");
    let notes = book.add_worksheet();
    notes.set_name("Instructions")?;
    notes.write_string(0, 0, "schema")?;
    notes.write_string(0, 1, SCHEMA)?;
    notes.write_string(1, 0, "kind")?;
    notes.write_string(1, 1, kind)?;
    notes.write_string(2, 0, "currency")?;
    notes.write_string(2, 1, "IRR whole rials. Do not enter tomans.")?;
    notes.write_string(3, 0, "formulas")?;
    notes.write_string(3, 1, "Not allowed. Values only. Macros are refused.")?;
    notes.write_string(4, 0, "dates")?;
    notes.write_string(
        4,
        1,
        "The import date is chosen in Sarv, not as an Excel serial.",
    )?;
    let data = book.add_worksheet();
    data.set_name("Data")?;
    for (column, header) in headers.iter().enumerate() {
        data.write_string_with_format(0, column as u16, *header, &text)?;
        data.set_column_width(column as u16, 22)?;
    }
    for (row_index, row) in rows.iter().enumerate() {
        for (column, value) in row.iter().enumerate() {
            data.write_string_with_format(
                (row_index + 1) as u32,
                column as u16,
                &safe_text(value),
                &text,
            )?;
        }
    }
    let bytes = book.save_to_buffer()?;
    Ok((filename.into(), bytes))
}

fn safe_text(value: &str) -> String {
    let trimmed = value.trim_start();
    if trimmed.starts_with(['=', '+', '-', '@']) {
        format!("'{value}")
    } else {
        value.to_owned()
    }
}

fn guard_zip(bytes: &[u8]) -> Result<()> {
    ensure!(bytes.len() <= MAX_BYTES, "Workbook is larger than 1 MB");
    ensure!(bytes.len() >= 4 && &bytes[..2] == b"PK", "Not an xlsx file");
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| anyhow!("Workbook archive is not readable"))?;
    ensure!(archive.len() <= MAX_ENTRIES, "Workbook has too many parts");
    let mut declared = 0u64;
    let mut actual = 0u64;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|_| anyhow!("Workbook part is not readable"))?;
        let name = file
            .name()
            .map_err(|_| anyhow!("Workbook part name is not readable"))?
            .replace('\\', "/");
        let lower = name.to_ascii_lowercase();
        ensure!(
            !lower.contains("..")
                && !lower.starts_with('/')
                && !lower.starts_with('\\')
                && !lower.contains("vbaproject")
                && !lower.contains("externallink")
                && !lower.contains("embeddings"),
            "Workbook contains a macro, an external link, or an unsafe path"
        );
        let declared_size = file.size();
        let compressed = file.compressed_size();
        ensure!(
            compressed <= MAX_BYTES as u64 && declared_size <= MAX_UNCOMPRESSED,
            "Workbook part is too large"
        );
        declared = declared.saturating_add(declared_size);
        ensure!(declared <= MAX_UNCOMPRESSED, "Workbook expands too far");
        let mut buf = [0u8; 8192];
        loop {
            let read = file
                .read(&mut buf)
                .map_err(|_| anyhow!("Workbook part could not be expanded"))?;
            if read == 0 {
                break;
            }
            actual = actual.saturating_add(read as u64);
            ensure!(actual <= MAX_UNCOMPRESSED, "Workbook expands too far");
        }
    }
    Ok(())
}

fn file_json(filename: &str, bytes: &[u8]) -> Value {
    json!({
        "filename": filename,
        "content_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "content_base64": STANDARD.encode(bytes),
        "bytes": bytes.len(),
        "schema": SCHEMA,
    })
}
fn summary(kind: &str, created: u32, updated: u32, skipped: u32, warnings: &[String]) -> Value {
    json!({
        "committed": true,
        "replayed": false,
        "kind": kind,
        "schema": SCHEMA,
        "created": created,
        "updated": updated,
        "skipped": skipped,
        "warnings": warnings,
        "currency": "IRR",
        "strategy": "all_or_nothing",
    })
}
fn headers(kind: &str) -> Result<&'static [&'static str]> {
    match kind {
        "products" => Ok(PRODUCTS),
        "customers" | "suppliers" => Ok(PARTIES),
        "opening_stock" => Ok(OPENING),
        _ => Err(anyhow!("Unknown Excel sheet kind")),
    }
}
fn kind(p: &Value) -> Result<&'static str> {
    match p["kind"].as_str().unwrap_or("") {
        "products" => Ok("products"),
        "customers" => Ok("customers"),
        "suppliers" => Ok("suppliers"),
        "opening_stock" => Ok("opening_stock"),
        other => Err(anyhow!("Unknown Excel sheet kind: {other}")),
    }
}
fn decode(p: &Value) -> Result<Vec<u8>> {
    let raw = required(p, "content_base64")?;
    let bytes = STANDARD
        .decode(raw.trim().as_bytes())
        .map_err(|_| anyhow!("Workbook is not valid base64"))?;
    ensure!(!bytes.is_empty(), "Workbook is empty");
    Ok(bytes)
}
fn required(p: &Value, key: &str) -> Result<String> {
    p.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("Missing {key}"))
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn stable(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let hex = format!("{:x}", hasher.finalize());
    format!("{prefix}-{}", &hex[..16])
}
fn cell_label(cell: &Data) -> Result<String> {
    match cell {
        Data::String(value) => Ok(value.trim().to_ascii_lowercase()),
        Data::Empty => Ok(String::new()),
        _ => Err(anyhow!("Column titles must be text")),
    }
}
fn plain_text(cell: &Data) -> std::result::Result<String, String> {
    match cell {
        Data::Empty => Ok(String::new()),
        Data::String(value) => {
            let trimmed = value.trim();
            if trimmed.starts_with('=') {
                return Err("Formula text is not accepted".into());
            }
            if trimmed.chars().any(char::is_control) {
                return Err("Control characters are not accepted".into());
            }
            Ok(trimmed.to_owned())
        }
        Data::Int(n) => Ok(n.to_string()),
        Data::Float(_) => Err("Write this value as text, not a decimal number".into()),
        Data::DateTime(_) | Data::DateTimeIso(_) | Data::DurationIso(_) => {
            Err("Excel date serials are rejected. Choose the date in Sarv.".into())
        }
        Data::Bool(_) => Err("Boolean cells are not used".into()),
        Data::Error(_) => Err("The cell contains an Excel error".into()),
    }
}
fn text_identifier(cell: &Data) -> std::result::Result<String, String> {
    match cell {
        Data::Int(_) | Data::Float(_) => {
            Err("Write SKUs, barcodes and IDs as text so leading zeros are kept".into())
        }
        other => plain_text(other),
    }
}
fn money_text(cell: &Data) -> std::result::Result<String, String> {
    match cell {
        Data::Empty => Ok(String::new()),
        Data::Int(n) if *n >= 0 => Ok(n.to_string()),
        Data::Float(_) => {
            Err("Write rials as a whole number. Fractional rials are rejected".into())
        }
        Data::String(value) => {
            let cleaned = digits(value).replace([' ', '٬', '،', ','], "");
            if cleaned.is_empty() {
                return Ok(String::new());
            }
            if cleaned.contains('.') || cleaned.starts_with('-') {
                return Err("Rials must be a whole nonnegative number".into());
            }
            if Money::parse(&cleaned, MoneyUnit::Rial).is_err() {
                return Err("Rial amount is invalid".into());
            }
            Ok(cleaned)
        }
        _ => plain_text(cell),
    }
}
fn quantity_text(cell: &Data) -> std::result::Result<String, String> {
    match cell {
        Data::Empty => Ok(String::new()),
        Data::Int(n) if *n > 0 => Ok(n.to_string()),
        Data::Int(0) => Ok("0".into()),
        Data::Float(n) => {
            if !n.is_finite() || *n < 0.0 {
                return Err("Quantity is invalid".into());
            }
            let thousandths = (n * 1000.0).round();
            if (n * 1000.0 - thousandths).abs() > 1e-4 {
                return Err("Quantity has more than 3 decimal places".into());
            }
            let whole = thousandths as i64;
            Ok(format_qty(whole))
        }
        Data::String(value) => {
            let cleaned = digits(value).replace([' ', '٬', '،'], "").replace('٫', ".");
            if cleaned.is_empty() {
                return Ok(String::new());
            }
            if cleaned.contains(',') {
                return Err("Use a dot for the decimal mark".into());
            }
            Ok(cleaned)
        }
        _ => plain_text(cell),
    }
}
fn format_qty(thousandths: i64) -> String {
    let whole = thousandths / 1000;
    let fraction = thousandths % 1000;
    if fraction == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fraction:03}")
            .trim_end_matches('0')
            .to_owned()
    }
}
fn check_label(value: &str) -> std::result::Result<(), String> {
    if value.starts_with(['=', '+', '@']) || text(value).is_err() {
        Err("Value is empty, too long, or looks like a formula".into())
    } else {
        Ok(())
    }
}
