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
pub fn catalog_csv(input: &str, name: &str) -> Result<Catalog> {
    ensure!(input.len() <= MAX_IMPORT, "فایل بیش از ۱۰ مگابایت است");
    text(name)?;
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(input.trim_start_matches('\u{feff}').as_bytes());
    ensure!(
        rdr.headers()?.iter().eq([
            "code",
            "name",
            "measure",
            "price",
            "currency",
            "source_at",
            "aliases"
        ]),
        "سرستون‌ها باید code,name,measure,price,currency,source_at,aliases باشند"
    );
    let mut products = vec![];
    let mut codes = HashSet::new();
    for (i, row) in rdr.records().enumerate() {
        let row = row.map_err(|e| anyhow::anyhow!("ردیف {}: {}", i + 2, e))?;
        let mut parse = || -> Result<Product> {
            text(&row[0])?;
            text(&row[1])?;
            text(&row[2])?;
            ensure!(codes.insert(normalize(&row[0])), "کد تکراری");
            let unit = Unit::parse(&row[4])?;
            timestamp(&row[5])?;
            let mut aliases = row[6]
                .split('|')
                .filter(|a| !a.trim().is_empty())
                .map(|a| {
                    text(a)?;
                    Ok(normalize(a))
                })
                .collect::<Result<Vec<_>>>()?;
            aliases.sort();
            aliases.dedup();
            Ok(Product {
                code: digits(&row[0]).to_uppercase(),
                name: row[1].to_string(),
                measure: row[2].to_string(),
                price_rial: unit.amount(&row[3])?,
                source_unit: unit,
                source_at: row[5].to_string(),
                aliases,
            })
        };
        products.push(parse().map_err(|e| anyhow::anyhow!("ردیف {}: {}", i + 2, e))?);
        ensure!(products.len() <= 50_000, "حداکثر ۵۰ هزار کالا");
    }
    ensure!(!products.is_empty(), "فهرست خالی است");
    let fingerprint = product_fingerprint(&products)?;
    Ok(Catalog {
        id: id(),
        name: name.trim().to_string(),
        imported_at: now(),
        fingerprint,
        products,
    })
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
/// CSV code,quantity; repeated codes are intentionally separate customer lines.
pub fn order_csv(input: &str) -> Result<Vec<OrderLine>> {
    ensure!(input.len() <= MAX_IMPORT, "فایل بیش از ۱۰ مگابایت است");
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(input.trim_start_matches('\u{feff}').as_bytes());
    ensure!(
        rdr.headers()?.iter().eq(["item", "quantity"]),
        "سرستون سفارش باید item,quantity باشد"
    );
    let mut lines = vec![];
    for (i, r) in rdr.records().enumerate() {
        let r = r.map_err(|e| anyhow::anyhow!("ردیف {}: {}", i + 2, e))?;
        text(&r[0]).map_err(|e| anyhow::anyhow!("ردیف {}: {}", i + 2, e))?;
        lines.push(OrderLine {
            query: r[0].to_string(),
            quantity_milli: quantity(&r[1])
                .map_err(|e| anyhow::anyhow!("ردیف {}: {}", i + 2, e))?,
            chosen_code: None,
        });
        ensure!(lines.len() <= 10_000, "حداکثر ۱۰ هزار ردیف");
    }
    ensure!(!lines.is_empty(), "سفارش خالی است");
    Ok(lines)
}
