use crate::{domain::*, store::write_new};
use anyhow::{Context, Result, ensure};
use std::path::Path;

/// Neutralize formulas after spreadsheet whitespace/control trimming. The quote
/// identifier, dates and every other textual field receive the same treatment.
pub fn safe_cell(s: &str) -> String {
    let first = s
        .trim_start_matches(|c: char| c.is_whitespace() || c.is_control() || c == '\u{feff}')
        .chars()
        .next();
    if matches!(first, Some('=' | '+' | '-' | '@')) {
        format!("'{s}")
    } else {
        s.to_string()
    }
}
pub fn csv_bytes(q: &Quote) -> Result<Vec<u8>> {
    validate_amounts(q)?;
    let mut w = csv::Writer::from_writer(vec![]);
    w.write_record([
        "quote_id",
        "version",
        "customer",
        "phone",
        "catalog",
        "requested_item",
        "code",
        "name",
        "quantity",
        "measure",
        "unit_price_rial",
        "line_total_rial",
        "source_unit",
        "source_at",
        "discount_rial",
        "quote_total_rial",
        "approved_at",
        "reference",
        "quote_date",
        "valid_until",
        "notes",
        "quote_number",
        "issuer_name",
        "issuer_phone",
        "issuer_address",
        "header",
        "footer",
        "terms",
        "tax_basis_points",
        "tax_rial",
        "display_digits",
        "date_display",
        "customer_id",
        "customer_contact",
        "customer_email",
        "customer_address",
    ])?;
    for l in &q.lines {
        w.write_record([
            safe_cell(&q.id),
            q.version.to_string(),
            safe_cell(&q.customer),
            safe_cell(&q.phone),
            safe_cell(&q.catalog_name),
            safe_cell(&l.query),
            safe_cell(&l.code),
            safe_cell(&l.name),
            format!("{}.{:03}", l.quantity_milli / 1000, l.quantity_milli % 1000),
            safe_cell(&l.measure),
            l.price_rial.to_string(),
            l.total_rial.to_string(),
            l.source_unit.label().into(),
            safe_cell(&l.source_at),
            q.discount_rial.to_string(),
            q.total_rial.to_string(),
            safe_cell(&q.approved_at),
            safe_cell(&q.reference),
            safe_cell(&q.quote_date),
            safe_cell(&q.valid_until),
            safe_cell(&q.notes),
            safe_cell(&q.number),
            safe_cell(&q.issuer_name),
            safe_cell(&q.issuer_phone),
            safe_cell(&q.issuer_address),
            safe_cell(&q.header),
            safe_cell(&q.footer),
            safe_cell(&q.terms),
            q.tax_basis_points.to_string(),
            q.tax_rial.to_string(),
            if q.display_digits == DisplayDigits::Latin {
                "latin".into()
            } else {
                "persian".into()
            },
            match q.date_display {
                DateDisplay::Iso => "iso".into(),
                DateDisplay::DayMonthYear => "day_month_year".into(),
                DateDisplay::Jalali => "jalali".into(),
            },
            safe_cell(&q.customer_id),
            safe_cell(&q.customer_contact),
            safe_cell(&q.customer_email),
            safe_cell(&q.customer_address),
        ])?;
    }
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend(w.into_inner()?);
    Ok(bytes)
}
pub fn csv(q: &Quote, path: &Path) -> Result<()> {
    write_new(path, &csv_bytes(q)?)
}
pub fn summary(q: &Quote) -> String {
    format!(
        "پیش‌فاکتور • نسخهٔ {}\n{}\nمشتری: {}  |  {}\nفهرست: {}\nتأیید: {}\nجمع نهایی: {}\nتخفیف: {}\nفقط پیش‌فاکتور — بدون ثبت حسابداری یا تعهد موجودی",
        document_digits(q, q.version),
        number(q),
        q.customer,
        ltr(&q.phone),
        q.catalog_name,
        ltr(&display_date(q, &q.approved_at)),
        display_amount(q, q.total_rial),
        display_amount(q, q.discount_rial)
    )
}

#[cfg(feature = "pdf-export")]
fn display_timestamp(q: &Quote, value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date| {
            let utc = date.with_timezone(&chrono::Utc);
            let date = q.date_display.format(&utc.format("%Y-%m-%d").to_string());
            document_digits(q, format!("{date} {}", utc.format("%H:%M UTC")))
        })
        .unwrap_or_else(|_| display_date(q, value))
}
fn document_digits(q: &Quote, value: impl ToString) -> String {
    let value = value.to_string();
    if q.display_digits == DisplayDigits::Latin {
        digits(&value).replace('٬', ",")
    } else {
        fa(value)
    }
}
fn display_date(q: &Quote, value: &str) -> String {
    document_digits(q, q.date_display.format(value))
}
fn display_amount(q: &Quote, rial: i64) -> String {
    document_digits(q, q.display_unit.display(rial))
}
#[cfg(feature = "pdf-export")]
fn amount(q: &Quote, rial: i64) -> String {
    display_amount(q, rial)
        .trim_end_matches(q.display_unit.label())
        .trim_end()
        .to_string()
}
fn number(q: &Quote) -> String {
    if q.number.is_empty() {
        if q.id.is_empty() {
            "پس از تأیید ایجاد می‌شود".into()
        } else {
            ltr(&q.id)
        }
    } else {
        ltr(&document_digits(q, &q.number))
    }
}
#[cfg(feature = "pdf-export")]
fn tax_label(q: &Quote) -> String {
    let whole = q.tax_basis_points / 100;
    let fractional = q.tax_basis_points % 100;
    let rate = if fractional == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fractional:02}")
            .trim_end_matches('0')
            .to_string()
    };
    format!(
        "مالیات تنظیم‌شده ({})",
        ltr(&format!("{}٪", document_digits(q, rate)))
    )
}
fn validate_amounts(q: &Quote) -> Result<i64> {
    ensure!(
        !q.lines.is_empty() && q.lines.len() <= MAX_QUOTE_LINES,
        "تعداد ردیف‌های خروجی نامعتبر است"
    );
    let mut subtotal = 0i64;
    for line in &q.lines {
        ensure!(
            line_total(line.price_rial, line.quantity_milli)? == line.total_rial,
            "جمع ردیف خروجی با قیمت تأییدشده سازگار نیست"
        );
        subtotal = subtotal
            .checked_add(line.total_rial)
            .filter(|n| *n <= MAX_MONEY)
            .context("سرریز جمع خروجی")?;
    }
    ensure!(
        q.discount_rial >= 0 && q.discount_rial <= subtotal && q.tax_basis_points <= 10_000,
        "تخفیف یا نرخ مالیات خروجی نامعتبر است"
    );
    let taxable = subtotal - q.discount_rial;
    let expected_tax = i128::from(tax_amount(taxable, q.tax_basis_points)?);
    let expected_total = i128::from(taxable) + expected_tax;
    ensure!(
        expected_total <= i128::from(MAX_MONEY)
            && i128::from(q.tax_rial) == expected_tax
            && i128::from(q.total_rial) == expected_total,
        "جمع نهایی خروجی ناسازگار است"
    );
    Ok(subtotal)
}

#[cfg(feature = "pdf-export")]
#[path = "export_pdf.rs"]
mod pdf_renderer;
#[cfg(feature = "pdf-export")]
pub use pdf_renderer::{pdf, png_pages};
