//! Exact, read-only sales documents and end-of-day control snapshots.
//! Source records remain immutable; these reports make no statutory/tax claim.
use crate::{
    accounting::{AccountingState, Money, MoneyUnit},
    domain::{DateDisplay, DisplayDigits, ltr},
    export::safe_cell,
    store::write_new,
};
use anyhow::{Result, ensure};
use pango::{self, prelude::FontMapExt};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug)]
pub struct DocumentPreview {
    pub title: String,
    pub company: String,
    pub as_of: String,
    pub policy_version: String,
    pub status: String,
    pub lines: Vec<String>,
}
struct Report {
    company_id: String,
    company: String,
    as_of: String,
    policy_version: String,
    policy_label: String,
    status: String,
    status_label: String,
    basis: String,
    basis_label: String,
    kind: String,
    source_id: String,
    title: String,
    rows: Vec<Row>,
    snapshot: String,
}
#[derive(Default)]
struct Row {
    kind: String,
    id: String,
    event_id: String,
    source_id: String,
    reference: String,
    label: String,
    quantity: Option<i64>,
    base_quantity: Option<i64>,
    remaining_quantity: Option<i64>,
    amount: Option<Money>,
    price: Option<Money>,
    ledger: Option<Money>,
    detail: Option<Money>,
    difference: Option<Money>,
    fields: BTreeMap<&'static str, String>,
    snapshot: String,
}
#[derive(Clone, Copy)]
struct Preferences {
    unit: MoneyUnit,
    dates: DateDisplay,
    digits: DisplayDigits,
}
fn unit_key(unit: MoneyUnit) -> &'static str {
    if unit == MoneyUnit::Rial {
        "rial"
    } else {
        "toman"
    }
}
fn date(value: &str, display: DateDisplay) -> Result<String> {
    crate::core::civil_date(value)?;
    if display == DateDisplay::Jalali {
        match crate::calendar::parse_date(value, crate::calendar::Calendar::Gregorian)
            .and_then(|date| crate::calendar::format_date(date, crate::calendar::Calendar::Jalali))
        {
            Ok(jalali) => Ok(jalali),
            Err(_) => Ok(format!("{value} (Gregorian; Jalali unavailable)")),
        }
    } else {
        Ok(display.format(value))
    }
}
fn qty(value: i64) -> String {
    let n = i128::from(value);
    let sign = if n < 0 { "-" } else { "" };
    format!("{sign}{}.{:03}", n.abs() / 1000, n.abs() % 1000)
}
fn company_name(state: &AccountingState, company_id: &str) -> Result<String> {
    Ok(state
        .companies
        .iter()
        .find(|c| c.id == company_id)
        .ok_or_else(|| anyhow::anyhow!("شرکت گزارش پیدا نشد"))?
        .name
        .clone())
}
fn render_csv(report: &Report, prefs: Preferences, path: &Path) -> Result<()> {
    let fields = [
        "role",
        "account_id",
        "account_code",
        "account_name",
        "party_id",
        "party_name",
        "item_id",
        "item_sku",
        "item_name",
        "warehouse_id",
        "warehouse_name",
        "target_warehouse_id",
        "target_warehouse_name",
        "unit_id",
        "unit_name",
        "base_unit_id",
        "base_unit_name",
        "document_id",
        "source_line_id",
        "document_date",
        "availability_date",
        "due_date",
        "sequence",
        "direction",
        "gross_irr",
        "line_discount_irr",
        "document_discount_irr",
        "original_quantity_thousandths",
        "original_value_irr",
        "financial_effect_id",
        "financial_effect_kind",
        "price_reference",
        "price_date",
        "override_reason",
        "allocation_method",
        "price_unit_id",
        "price_unit_name",
        "cost_irr",
        "net_irr",
        "delivery_id",
        "invoice_id",
        "return_id",
        "credit_id",
        "delivery_reference",
        "invoice_reference",
        "return_reference",
        "credit_reference",
        "delivery_state",
        "invoice_state",
        "return_state",
        "credit_state",
        "quote_id",
        "quote_series",
        "quote_revision",
        "quote_number",
        "quote_sha256",
        "quote_source_line_index",
        "credit_ids_json",
    ];
    let mut writer = csv::Writer::from_writer(Vec::new());
    let mut headers = vec![
        "format_version",
        "company_id",
        "company",
        "as_of_gregorian",
        "display_as_of",
        "display_calendar",
        "base_currency",
        "display_unit",
        "policy_version",
        "included_states",
        "basis",
        "document_kind",
        "report_source_id",
        "row_kind",
        "row_id",
        "event_id",
        "source_id",
        "source_reference",
        "label",
        "quantity_thousandths",
        "base_quantity_thousandths",
        "remaining_quantity_thousandths",
        "amount_irr",
        "unit_price_irr",
        "display_amount",
        "ledger_irr",
        "detail_irr",
        "difference_irr",
        "display_ledger",
        "display_detail",
        "display_difference",
    ];
    headers.extend(fields);
    headers.push("snapshot_json");
    writer.write_record(headers)?;
    let common = vec![
        "sarv-sales-report-v1".into(),
        safe_cell(&report.company_id),
        safe_cell(&report.company),
        report.as_of.clone(),
        prefs.digits.format(date(&report.as_of, prefs.dates)?),
        if prefs.dates == DateDisplay::Jalali {
            if date(&report.as_of, prefs.dates)?.contains("Gregorian;") {
                "gregorian_fallback".into()
            } else {
                "jalali".into()
            }
        } else {
            "gregorian".into()
        },
        "IRR".into(),
        unit_key(prefs.unit).into(),
        safe_cell(&report.policy_version),
        safe_cell(&report.status),
        safe_cell(&report.basis),
        safe_cell(&report.kind),
        safe_cell(&report.source_id),
    ];
    let mut rows: Vec<&Row> = report.rows.iter().collect();
    let full = Row {
        kind: "complete_snapshot".into(),
        id: report.source_id.clone(),
        source_id: report.source_id.clone(),
        snapshot: report.snapshot.clone(),
        ..Row::default()
    };
    rows.push(&full);
    for row in rows {
        let money = |m: Option<Money>| m.map(|m| m.rials().to_string()).unwrap_or_default();
        let display = |m: Option<Money>| {
            m.map(|m| prefs.digits.format(m.display(prefs.unit)))
                .unwrap_or_default()
        };
        let mut cells = common.clone();
        cells.extend([
            safe_cell(&row.kind),
            safe_cell(&row.id),
            safe_cell(&row.event_id),
            safe_cell(&row.source_id),
            safe_cell(&row.reference),
            safe_cell(&row.label.replace(['\u{2066}', '\u{2069}'], "")),
            row.quantity.map(|q| q.to_string()).unwrap_or_default(),
            row.base_quantity.map(|q| q.to_string()).unwrap_or_default(),
            row.remaining_quantity
                .map(|q| q.to_string())
                .unwrap_or_default(),
            money(row.amount),
            money(row.price),
            display(row.amount),
            money(row.ledger),
            money(row.detail),
            money(row.difference),
            display(row.ledger),
            display(row.detail),
            display(row.difference),
        ]);
        cells.extend(
            fields
                .iter()
                .map(|key| safe_cell(row.fields.get(key).map_or("", String::as_str))),
        );
        cells.push(safe_cell(&row.snapshot));
        writer.write_record(cells)?;
    }
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend(writer.into_inner()?);
    write_new(path, &bytes)
}
fn layout(cr: &cairo::Context, map: &pango::FontMap, value: &str, size: u32) -> pango::Layout {
    let context = map.create_context();
    pangocairo::functions::update_context(cr, &context);
    context.set_base_dir(pango::Direction::Rtl);
    pangocairo::functions::context_set_resolution(&context, 72.0);
    let layout = pango::Layout::new(&context);
    layout.set_auto_dir(false);
    layout.set_font_description(Some(&pango::FontDescription::from_string(&format!(
        "Vazirmatn, Noto Sans Arabic {size}"
    ))));
    layout.set_auto_dir(false);
    layout.set_width(523 * pango::SCALE);
    layout.set_wrap(pango::WrapMode::WordChar);
    layout.set_alignment(pango::Alignment::Right);
    layout.set_text(value);
    layout
}
fn draw(cr: &cairo::Context, map: &pango::FontMap, value: &str, y: f64, size: u32) -> Result<f64> {
    let layout = layout(cr, map, value, size);
    cr.move_to(36.0, y);
    pangocairo::functions::show_layout(cr, &layout);
    cr.status()?;
    Ok(f64::from(layout.pixel_size().1))
}
fn human_line(row: &Row, prefs: Preferences) -> String {
    let mut line = if matches!(
        row.kind.as_str(),
        "order_line" | "fulfillment_line" | "return_line" | "receivable_remaining"
    ) {
        format!("{} • {}", ltr(&row.id), prefs.digits.format(&row.label))
    } else {
        prefs.digits.format(&row.label)
    };
    if let Some(q) = row.quantity {
        line.push_str(&format!(
            "\nمقدار انتخابی {} • مقدار پایه {}",
            ltr(&prefs.digits.format(qty(q))),
            ltr(&prefs.digits.format(qty(row.base_quantity.unwrap_or(q))))
        ));
        if let Some(remaining) = row.remaining_quantity {
            line.push_str(&format!(
                " • باقی‌مانده {}",
                ltr(&prefs.digits.format(qty(remaining)))
            ));
        }
    }
    if let Some(price) = row.price {
        line.push_str(&format!(
            "\n{} {} {} • دقیق IRR {}",
            if matches!(row.kind.as_str(), "fulfillment_line" | "return_line") {
                format!(
                    "قیمت اصلی سفارش • واحد قیمت {}",
                    row.fields.get("price_unit_name").map_or("", String::as_str)
                )
            } else {
                "قیمت هر واحد".into()
            },
            ltr(&prefs.digits.format(price.display(prefs.unit))),
            unit_key(prefs.unit),
            ltr(&prefs.digits.format(price.rials()))
        ));
    }
    if let Some(amount) = row.amount {
        line.push_str(&format!(
            "\nمبلغ {} {} • دقیق IRR {}",
            ltr(&prefs.digits.format(amount.display(prefs.unit))),
            unit_key(prefs.unit),
            ltr(&prefs.digits.format(amount.rials()))
        ));
    }
    if let (Some(ledger), Some(detail), Some(difference)) = (row.ledger, row.detail, row.difference)
    {
        line.push_str(&format!(
            "\nدفتر کل {} • جزئیات {} • اختلاف {} {}",
            ltr(&prefs.digits.format(ledger.display(prefs.unit))),
            ltr(&prefs.digits.format(detail.display(prefs.unit))),
            ltr(&prefs.digits.format(difference.display(prefs.unit))),
            unit_key(prefs.unit)
        ));
        line.push_str(&format!(
            "\nدقیق IRR • دفتر کل {} • جزئیات {} • اختلاف {}",
            ltr(&prefs.digits.format(ledger.rials())),
            ltr(&prefs.digits.format(detail.rials())),
            ltr(&prefs.digits.format(difference.rials()))
        ));
    }
    if !row.reference.is_empty() {
        line.push_str(&format!("\nمرجع {}", ltr(&row.reference)));
    }
    if row.kind == "document_metadata" || row.kind == "source_event" {
        line.push_str(&format!("\nشناسهٔ رویداد {}", ltr(&row.event_id)));
    }
    line
}
fn render_pdf(report: &Report, prefs: Preferences, path: &Path) -> Result<()> {
    let displayed = date(&report.as_of, prefs.dates)?;
    let jalali = date(&report.as_of, DateDisplay::Jalali)
        .unwrap_or_else(|_| "خارج از دامنهٔ شمسی؛ میلادی معتبر است".into());
    let surface = cairo::PdfSurface::for_stream(595.0, 842.0, Vec::<u8>::new())?;
    let cr = cairo::Context::new(&surface)?;
    let map = crate::pdf_fonts::font_map();
    let _fonts = crate::pdf_fonts::private(&map, "sales")?;
    let heading = |page: usize| -> Result<f64> {
        cr.set_source_rgb(0.08, 0.12, 0.16);
        let mut y = 32.0;
        y += draw(
            &cr,
            &map,
            &format!("{} • {}", report.title, report.company),
            y,
            15,
        )? + 6.0;
        let header_lines = [
            format!("شناسهٔ شرکت {}", ltr(&report.company_id)),
            format!(
                "مرجع منبع {}",
                if report.kind == "sales_reconciliation" {
                    "تطبیق در پایان روز".into()
                } else {
                    ltr(&report.source_id)
                }
            ),
            format!(
                "نمایش {} • شمسی {}",
                ltr(&prefs.digits.format(&displayed)),
                ltr(&prefs.digits.format(&jalali))
            ),
            format!(
                "تاریخ مالی میلادی {}",
                ltr(&prefs.digits.format(&report.as_of))
            ),
            format!(
                "پایه IRR • واحد {} • سیاست {} • صفحه {}",
                unit_key(prefs.unit),
                ltr(&report.policy_version),
                prefs.digits.format(page)
            ),
            report.status_label.clone(),
            report.basis_label.clone(),
            report.policy_label.clone(),
        ];
        for header in header_lines {
            y += draw(&cr, &map, &header, y, 9)? + 3.0;
        }
        y += 7.0;
        y += draw(
            &cr,
            &map,
            "کالا / حساب / منبع • مقدار و ارزش دقیق • جزئیات و اختلاف کنترل",
            y,
            10,
        )? + 12.0;
        ensure!(y < 620.0, "سربرگ در صفحه جا نمی‌شود؛ CSV را انتخاب کنید");
        Ok(y)
    };
    let measure_surface = cairo::RecordingSurface::create(cairo::Content::ColorAlpha, None)?;
    let measure = cairo::Context::new(&measure_surface)?;
    let mut page = 1;
    let mut y = heading(page)?;
    for row in &report.rows {
        let value = human_line(row, prefs);
        let height = f64::from(layout(&measure, &map, &value, 9).pixel_size().1) + 12.0;
        if y + height > 778.0 {
            cr.show_page()?;
            page += 1;
            y = heading(page)?;
        }
        ensure!(
            y + height <= 778.0,
            "ردیف در یک صفحه جا نمی‌شود؛ CSV را انتخاب کنید"
        );
        cr.set_source_rgb(0.08, 0.12, 0.16);
        draw(&cr, &map, &value, y, 9)?;
        y += height;
        cr.set_source_rgb(0.8, 0.84, 0.87);
        cr.set_line_width(0.5);
        cr.move_to(36.0, y - 5.0);
        cr.line_to(559.0, y - 5.0);
        cr.stroke()?;
    }
    cr.show_page()?;
    surface.finish();
    surface.status()?;
    let bytes = surface
        .finish_output_stream()
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .downcast::<Vec<u8>>()
        .map_err(|_| anyhow::anyhow!("جریان PDF نامعتبر است"))?;
    write_new(path, &bytes)
}

use crate::{
    core::CoreItem,
    procurement::{RegisterResources, StockDirection},
    sales::*,
};
fn field(row: &mut Row, key: &'static str, value: impl ToString) {
    row.fields.insert(key, value.to_string());
}
fn item_row(row: &mut Row, resources: &RegisterResources, item: &CoreItem) -> Result<()> {
    field(row, "item_id", &item.id);
    field(row, "item_sku", &item.sku);
    field(row, "item_name", &item.name);
    field(row, "base_unit_id", &item.base_unit_id);
    let unit = resources
        .units
        .iter()
        .find(|u| u.id == item.base_unit_id)
        .ok_or_else(|| anyhow::anyhow!("واحد پایهٔ تاریخی گزارش پیدا نشد"))?;
    field(row, "base_unit_name", &unit.name);
    Ok(())
}
fn pack_label(
    resources: &RegisterResources,
    item: &CoreItem,
    selected_unit: &str,
) -> Result<String> {
    if selected_unit == item.base_unit_id {
        return Ok(String::new());
    }
    let name = |id: &str| {
        resources
            .units
            .iter()
            .find(|u| u.id == id)
            .map(|u| u.name.as_str())
            .ok_or_else(|| anyhow::anyhow!("واحد تاریخی نسبت بسته پیدا نشد"))
    };
    let ratio = item
        .packs
        .iter()
        .find(|p| p.unit_id == selected_unit)
        .ok_or_else(|| anyhow::anyhow!("نسبت منجمد بسته پیدا نشد"))?;
    Ok(format!(
        " • نسبت بسته {}",
        ltr(&format!(
            "1 {} = {} / {} {}",
            name(selected_unit)?,
            ratio.numerator,
            ratio.denominator,
            name(&item.base_unit_id)?
        ))
    ))
}
fn item<'a>(resources: &'a RegisterResources, id: &str) -> Result<&'a CoreItem> {
    resources
        .items
        .iter()
        .find(|x| x.id == id)
        .ok_or_else(|| anyhow::anyhow!("کالای تاریخی گزارش پیدا نشد"))
}
fn warehouse(
    row: &mut Row,
    resources: &RegisterResources,
    id: &str,
    target: bool,
) -> Result<String> {
    let w = resources
        .warehouses
        .iter()
        .find(|x| x.id == id)
        .ok_or_else(|| anyhow::anyhow!("انبار تاریخی گزارش پیدا نشد"))?;
    field(
        row,
        if target {
            "target_warehouse_id"
        } else {
            "warehouse_id"
        },
        id,
    );
    field(
        row,
        if target {
            "target_warehouse_name"
        } else {
            "warehouse_name"
        },
        &w.name,
    );
    Ok(w.name.clone())
}
fn unit(row: &mut Row, resources: &RegisterResources, id: &str) -> Result<String> {
    let u = resources
        .units
        .iter()
        .find(|x| x.id == id)
        .ok_or_else(|| anyhow::anyhow!("واحد تاریخی گزارش پیدا نشد"))?;
    field(row, "unit_id", id);
    field(row, "unit_name", &u.name);
    Ok(u.name.clone())
}
fn party(row: &mut Row, resources: &RegisterResources, id: &str) -> Result<String> {
    let p = resources
        .parties
        .iter()
        .find(|x| x.id == id)
        .ok_or_else(|| anyhow::anyhow!("شخص تاریخی گزارش پیدا نشد"))?;
    field(row, "party_id", id);
    field(row, "party_name", &p.name);
    Ok(p.name.clone())
}
fn account_name(state: &AccountingState, company_id: &str, id: &str) -> Result<(String, String)> {
    let a = state
        .accounts
        .iter()
        .find(|a| a.id == id && a.company_id == company_id)
        .ok_or_else(|| anyhow::anyhow!("حساب تاریخی گزارش پیدا نشد؛ نام قابل حدس نیست"))?;
    Ok((a.code.clone(), a.name.clone()))
}
fn source_row(record: &SalesRecord, kind: &str, id: &str) -> Row {
    Row {
        kind: kind.into(),
        id: id.into(),
        event_id: record.id.clone(),
        source_id: record.command.context().source_id.clone(),
        ..Row::default()
    }
}
fn selected_master_snapshot(resources: &RegisterResources, rows: &[Row]) -> serde_json::Value {
    let ids = |key: &str| {
        rows.iter()
            .filter_map(|r| r.fields.get(key))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
    };
    let parties = ids("party_id");
    let items = ids("item_id");
    let warehouses = ids("warehouse_id");
    let mut units = ids("unit_id");
    units.extend(ids("base_unit_id"));
    for i in resources.items.iter().filter(|i| items.contains(&i.id)) {
        units.extend(i.packs.iter().map(|p| p.unit_id.clone()));
    }
    serde_json::json!({"parties":resources.parties.iter().filter(|x|parties.contains(&x.id)).collect::<Vec<_>>(),"items":resources.items.iter().filter(|x|items.contains(&x.id)).collect::<Vec<_>>(),"warehouses":resources.warehouses.iter().filter(|x|warehouses.contains(&x.id)).collect::<Vec<_>>(),"units":resources.units.iter().filter(|x|units.contains(&x.id)).collect::<Vec<_>>()})
}
fn record_for<'a>(state: &'a AccountingState, id: &str) -> Result<&'a SalesRecord> {
    if let Some(record) = state.sales.records.iter().find(|r| {
        r.id == id
            || match &r.data {
                SalesData::Fulfill(v) => v.delivery_id == id || v.invoice_id == id,
                SalesData::Return(v) => v.return_id == id || v.credit_id == id,
                _ => false,
            }
    }) {
        return Ok(record);
    }
    let mut sources = state
        .sales
        .records
        .iter()
        .filter(|r| r.command.context().source_id == id);
    let record = sources
        .next()
        .ok_or_else(|| anyhow::anyhow!("سند فروش پیدا نشد"))?;
    ensure!(
        sources.next().is_none(),
        "مرجع منبع چند سند دارد؛ شناسهٔ یکتای رویداد یا سند را انتخاب کنید"
    );
    Ok(record)
}

fn price_unit(row: &mut Row, resources: &RegisterResources, id: &str) -> Result<()> {
    let unit = resources
        .units
        .iter()
        .find(|u| u.id == id)
        .ok_or_else(|| anyhow::anyhow!("واحد قیمت اصلی سفارش پیدا نشد"))?;
    field(row, "price_unit_id", id);
    field(row, "price_unit_name", &unit.name);
    Ok(())
}
const ALLOCATION_NOTE: &str = "خالص و تخفیف‌های اصلی جداگانه و تجمعی گرد می‌شوند؛ آخرین بخش باقی‌ماندهٔ دقیق را می‌گیرد. ناخالص تخصیص‌یافته برابر خالص به‌علاوهٔ تخفیف‌هاست و ممکن است با قیمت اصلی سفارش ضرب‌در مقدار این بخش فرق کند؛ اختلاف الزاماً یک ریال نیست.";
fn amount_breakdown(
    row: &mut Row,
    gross: Money,
    line_discount: Money,
    document_discount: Money,
    net: Money,
    cost: Option<Money>,
) {
    field(row, "gross_irr", gross.rials());
    field(row, "line_discount_irr", line_discount.rials());
    field(row, "document_discount_irr", document_discount.rials());
    field(row, "net_irr", net.rials());
    row.amount = Some(net);
    let allocated = matches!(row.kind.as_str(), "fulfillment_line" | "return_line");
    if allocated {
        field(
            row,
            "allocation_method",
            "cumulative_original_net_and_discounts_half_up_final_residual_gross_sum",
        );
    }
    row.label.push_str(&format!(
        "\n{} {} IRR • تخفیف ردیف {} IRR • تخفیف سند {} IRR • خالص {} IRR",
        if allocated {
            "ناخالص تخصیص‌یافته از سفارش"
        } else {
            "ناخالص"
        },
        ltr(&gross.rials().to_string()),
        ltr(&line_discount.rials().to_string()),
        ltr(&document_discount.rials().to_string()),
        ltr(&net.rials().to_string())
    ));
    if let Some(cost) = cost {
        field(row, "cost_irr", cost.rials());
        row.label.push_str(&format!(
            " • بهای اصلی خروج {} IRR",
            ltr(&cost.rials().to_string())
        ));
    }
}
fn quotation(row: &mut Row, order: &SalesOrder, index: Option<usize>) -> Result<()> {
    if let Some(q) = &order.quotation {
        for (key, value) in [
            ("quote_id", q.quote_id.clone()),
            ("quote_series", q.quote_series.clone()),
            ("quote_revision", q.quote_revision.to_string()),
            ("quote_number", q.quote_number.clone()),
            ("quote_sha256", q.quote_sha256.clone()),
        ] {
            field(row, key, value);
        }
        if let Some(index) = index {
            field(
                row,
                "quote_source_line_index",
                *q.source_line_indices
                    .get(index)
                    .ok_or_else(|| anyhow::anyhow!("پیوند ردیف پیش‌فاکتور نامعتبر است"))?,
            );
        }
        row.label.push_str(&format!(
            "\nپیش‌فاکتور پیوندشده {} • شناسه {} • سری {} • نسخه {} • SHA256 {}",
            ltr(&q.quote_number),
            ltr(&q.quote_id),
            ltr(&q.quote_series),
            ltr(&q.quote_revision.to_string()),
            ltr(&q.quote_sha256)
        ));
    } else {
        row.label.push_str("\nسفارش مستقیم؛ بدون پیوند پیش‌فاکتور");
    }
    Ok(())
}
fn document_report(state: &AccountingState, company_id: &str, document_id: &str) -> Result<Report> {
    let record = record_for(state, document_id)?;
    let context = record.command.context();
    ensure!(
        context.company_id == company_id,
        "سند متعلق به شرکت انتخاب‌شده نیست"
    );
    let resources = state.procurement_resources(company_id)?;
    let (kind, title) = match &record.data {
        SalesData::Order(_) => ("sales_order", "سفارش فروش"),
        SalesData::Fulfill(v) if document_id == v.delivery_id => ("delivery_note", "حواله خروج"),
        SalesData::Fulfill(_) => ("sales_invoice", "فاکتور فروش"),
        SalesData::Return(v) if document_id == v.return_id => {
            ("sales_return", "برگهٔ برگشت و ورود به انبار")
        }
        SalesData::Return(_) => ("sales_credit", "برگهٔ اعتبار و برگشت فروش"),
    };
    let mut report = Report {
        company_id: company_id.into(),
        company: company_name(state, company_id)?,
        as_of: context.date.clone(),
        policy_version: record.frozen_setup.command.policy.version.clone(),
        policy_label: record.frozen_setup.command.policy.summary(),
        status: if record.journal_id.is_some() {
            "posted_financial_document"
        } else {
            "recorded_nonfinancial_event"
        }
        .into(),
        status_label: if record.journal_id.is_some() {
            "رویداد مالی ثبت‌شده؛ وصول وجه انجام نشده"
        } else {
            "رویداد عملیاتی ثبت‌شده؛ بدون سند مالی"
        }
        .into(),
        basis: "immutable_source_snapshot_at_recording".into(),
        basis_label: "تصویر ثابت زمان ثبت • تعهدی / موجودی دائمی • بدون ادعای مالیاتی".into(),
        kind: kind.into(),
        source_id: context.source_id.clone(),
        title: title.into(),
        rows: vec![],
        snapshot: String::new(),
    };
    let mut metadata = source_row(record, "document_metadata", &record.id);
    metadata.label = format!(
        "شناسهٔ منبع {} • شناسهٔ رویداد {} • ترتیب {}\nتاریخ مالی میلادی {} • دلیل {}",
        ltr(&context.source_id),
        ltr(&record.id),
        ltr(&record.sequence.to_string()),
        ltr(&context.date),
        context.reason
    );
    field(&mut metadata, "sequence", record.sequence);
    field(
        &mut metadata,
        "financial_effect_kind",
        if record.journal_id.is_some() {
            "journal"
        } else {
            "nonfinancial"
        },
    );
    field(
        &mut metadata,
        "financial_effect_id",
        record.journal_id.as_deref().unwrap_or(""),
    );
    metadata.snapshot = serde_json::to_string(record)?;
    let mut related = Vec::new();
    match (&record.command, &record.data) {
        (SalesCommand::Order(input), SalesData::Order(posted)) => {
            metadata.reference = input.reference.clone();
            field(&mut metadata, "document_date", &input.document_date);
            metadata.label.push_str(&format!(
                "\nتاریخ سند میلادی {} • شمسی {} • مشتری {} • سفارش ثبت‌شده؛ بدون رزرو و خروج کالا",
                ltr(&input.document_date),
                ltr(&date(&input.document_date, DateDisplay::Jalali)?),
                posted.customer.name
            ));
            field(&mut metadata, "party_id", &posted.customer.id);
            field(&mut metadata, "party_name", &posted.customer.name);
            quotation(&mut metadata, input, None)?;
            for (index, line) in posted.lines.iter().enumerate() {
                let mut row = source_row(record, "order_line", &line.input.id);
                item_row(&mut row, &resources, &line.item)?;
                let wh = warehouse(&mut row, &resources, &line.input.warehouse_id, false)?;
                let u = unit(&mut row, &resources, &line.input.unit_id)?;
                row.label = format!(
                    "{} • کد {} • انبار {} • واحد {}{}",
                    line.item.name,
                    ltr(&line.item.sku),
                    wh,
                    u,
                    pack_label(&resources, &line.item, &line.input.unit_id)?
                );
                row.quantity = Some(line.input.quantity_thousandths);
                row.base_quantity = Some(line.base_quantity_thousandths);
                row.price = Some(line.input.unit_price);
                price_unit(&mut row, &resources, &line.input.unit_id)?;
                field(&mut row, "source_line_id", &line.input.id);
                field(&mut row, "price_reference", &line.input.price_reference);
                field(&mut row, "price_date", &line.input.price_date);
                field(
                    &mut row,
                    "override_reason",
                    line.input.override_reason.as_deref().unwrap_or(""),
                );
                row.label.push_str(&format!(
                    "\nمنبع قیمت {} • تاریخ قیمت {} • دلیل تغییر {}",
                    ltr(&line.input.price_reference),
                    ltr(&line.input.price_date),
                    line.input
                        .override_reason
                        .as_deref()
                        .unwrap_or("بدون تغییر")
                ));
                amount_breakdown(
                    &mut row,
                    line.gross,
                    line.input.line_discount,
                    line.document_discount,
                    line.net,
                    None,
                );
                quotation(&mut row, input, Some(index))?;
                row.snapshot = serde_json::to_string(line)?;
                report.rows.push(row);
            }
            report.rows.push(Row {
                kind: "document_total".into(),
                id: record.id.clone(),
                label: "جمع خالص سفارش؛ بدون اثر دفتر کل و موجودی".into(),
                amount: Some(posted.total),
                ..Row::default()
            });
        }
        (SalesCommand::Fulfill(input), SalesData::Fulfill(posted)) => {
            let order = record_for(state, &posted.order_id)?;
            let (SalesCommand::Order(order_input), SalesData::Order(order_data)) =
                (&order.command, &order.data)
            else {
                anyhow::bail!("منبع سفارش ناسازگار است")
            };
            related.push(order);
            metadata.reference = input.invoice_reference.clone();
            field(&mut metadata, "document_date", &input.document_date);
            field(&mut metadata, "due_date", &input.due_date);
            field(&mut metadata, "delivery_id", &posted.delivery_id);
            field(&mut metadata, "invoice_id", &posted.invoice_id);
            field(
                &mut metadata,
                "delivery_reference",
                &input.delivery_reference,
            );
            field(&mut metadata, "invoice_reference", &input.invoice_reference);
            field(&mut metadata, "delivery_state", "delivered");
            field(&mut metadata, "invoice_state", "posted");
            field(&mut metadata, "party_id", &posted.customer.id);
            field(&mut metadata, "party_name", &posted.customer.name);
            metadata.label.push_str(&format!("\nتحویل {} • مرجع {} • وضعیت تحویل‌شده\nفاکتور {} • مرجع {} • وضعیت ثبت‌شده\nمشتری {} • تاریخ سند میلادی {} • شمسی {} • سررسید میلادی {} • شمسی {}\nسفارش منبع {} • ثبت همزمان تحویل و فاکتور؛ پرداخت ثبت نشده",ltr(&posted.delivery_id),ltr(&input.delivery_reference),ltr(&posted.invoice_id),ltr(&input.invoice_reference),posted.customer.name,ltr(&input.document_date),ltr(&date(&input.document_date,DateDisplay::Jalali)?),ltr(&input.due_date),ltr(&date(&input.due_date,DateDisplay::Jalali)?),ltr(&posted.order_id)));
            quotation(&mut metadata, order_input, None)?;
            metadata.label.push_str(&format!("\n{ALLOCATION_NOTE}"));
            field(
                &mut metadata,
                "allocation_method",
                "cumulative_original_net_and_discounts_half_up_final_residual_gross_sum",
            );
            for line in &posted.lines {
                let original = order_data
                    .lines
                    .iter()
                    .find(|l| l.input.id == line.order_line_id)
                    .ok_or_else(|| anyhow::anyhow!("ردیف سفارش منبع پیدا نشد"))?;
                let selected = input
                    .lines
                    .iter()
                    .find(|l| l.id == line.id)
                    .ok_or_else(|| anyhow::anyhow!("ردیف تحویل پیدا نشد"))?;
                let mut row = source_row(record, "fulfillment_line", &line.id);
                item_row(&mut row, &resources, &line.item)?;
                let wh = warehouse(&mut row, &resources, &line.warehouse_id, false)?;
                let u = unit(&mut row, &resources, &selected.unit_id)?;
                row.label = format!(
                    "{} • کد {} • انبار {} • واحد {}{}\nردیف سفارش {}",
                    line.item.name,
                    ltr(&line.item.sku),
                    wh,
                    u,
                    pack_label(&resources, &line.item, &selected.unit_id)?,
                    ltr(&line.order_line_id)
                );
                row.quantity = Some(selected.quantity_thousandths);
                row.base_quantity = Some(line.quantity_thousandths);
                row.price = Some(original.input.unit_price);
                price_unit(&mut row, &resources, &original.input.unit_id)?;
                field(&mut row, "source_line_id", &line.order_line_id);
                field(&mut row, "price_reference", &original.input.price_reference);
                field(&mut row, "price_date", &original.input.price_date);
                field(
                    &mut row,
                    "override_reason",
                    original.input.override_reason.as_deref().unwrap_or(""),
                );
                row.label.push_str(&format!(
                    "\nمنبع قیمت {} • تاریخ قیمت {} • دلیل تغییر {}",
                    ltr(&original.input.price_reference),
                    ltr(&original.input.price_date),
                    original
                        .input
                        .override_reason
                        .as_deref()
                        .unwrap_or("بدون تغییر")
                ));
                let source_index = order_data
                    .lines
                    .iter()
                    .position(|l| l.input.id == line.order_line_id)
                    .ok_or_else(|| anyhow::anyhow!("ردیف سفارش منبع پیدا نشد"))?;
                quotation(&mut row, order_input, Some(source_index))?;
                amount_breakdown(
                    &mut row,
                    line.gross,
                    line.line_discount,
                    line.document_discount,
                    line.net,
                    Some(line.cost),
                );
                row.snapshot = serde_json::to_string(line)?;
                report.rows.push(row);
            }
            report.rows.push(Row {
                kind: "document_total".into(),
                id: posted.invoice_id.clone(),
                label: format!(
                    "جمع خالص فاکتور • بهای خروج {} IRR",
                    ltr(&posted.cost.rials().to_string())
                ),
                amount: Some(posted.total),
                ..Row::default()
            });
        }
        (SalesCommand::Return(input), SalesData::Return(posted)) => {
            let fulfillment = record_for(state, &posted.fulfillment_id)?;
            let (SalesCommand::Fulfill(fulfill_input), SalesData::Fulfill(fulfill_data)) =
                (&fulfillment.command, &fulfillment.data)
            else {
                anyhow::bail!("منبع فاکتور ناسازگار است")
            };
            let order = record_for(state, &fulfill_data.order_id)?;
            related.extend([fulfillment, order]);
            let (SalesCommand::Order(order_input), SalesData::Order(order_data)) =
                (&order.command, &order.data)
            else {
                anyhow::bail!("منبع سفارش ناسازگار است")
            };
            metadata.reference = input.credit_reference.clone();
            field(&mut metadata, "document_date", &input.document_date);
            field(&mut metadata, "return_id", &posted.return_id);
            field(&mut metadata, "credit_id", &posted.credit_id);
            field(&mut metadata, "return_reference", &input.return_reference);
            field(&mut metadata, "credit_reference", &input.credit_reference);
            field(&mut metadata, "return_state", "restocked");
            field(&mut metadata, "credit_state", "posted");
            field(&mut metadata, "invoice_id", &fulfill_data.invoice_id);
            field(&mut metadata, "delivery_id", &fulfill_data.delivery_id);
            field(&mut metadata, "party_id", &fulfill_data.customer.id);
            field(&mut metadata, "party_name", &fulfill_data.customer.name);
            metadata.label.push_str(&format!("\nبرگشت {} • مرجع {} • ورود به انبار اصلی\nاعتبار {} • مرجع {} • ثبت‌شده\nفاکتور اصلی {} • مرجع {} • مشتری {}\nتاریخ سند میلادی {} • شمسی {} • بازگردانی بهای اصلی؛ سفارش دوباره باز نمی‌شود",ltr(&posted.return_id),ltr(&input.return_reference),ltr(&posted.credit_id),ltr(&input.credit_reference),ltr(&fulfill_data.invoice_id),ltr(&fulfill_input.invoice_reference),fulfill_data.customer.name,ltr(&input.document_date),ltr(&date(&input.document_date,DateDisplay::Jalali)?)));
            quotation(&mut metadata, order_input, None)?;
            metadata.label.push_str(&format!("\n{ALLOCATION_NOTE}"));
            field(
                &mut metadata,
                "allocation_method",
                "cumulative_original_net_and_discounts_half_up_final_residual_gross_sum",
            );
            for line in &posted.lines {
                let original = fulfill_data
                    .lines
                    .iter()
                    .find(|l| l.id == line.fulfillment_line_id)
                    .ok_or_else(|| anyhow::anyhow!("ردیف خروج اصلی پیدا نشد"))?;
                let mut row = source_row(record, "return_line", &line.id);
                item_row(&mut row, &resources, &original.item)?;
                let wh = warehouse(&mut row, &resources, &line.warehouse_id, false)?;
                let u = unit(&mut row, &resources, &original.item.base_unit_id)?;
                row.label = format!(
                    "{} • کد {} • انبار اصلی {} • واحد پایه {}\nردیف خروج اصلی {} • مقدار اولیه {} • خالص اولیه {} IRR • بهای اولیه {} IRR",
                    original.item.name,
                    ltr(&original.item.sku),
                    wh,
                    u,
                    ltr(&line.fulfillment_line_id),
                    ltr(&qty(original.quantity_thousandths)),
                    ltr(&original.net.rials().to_string()),
                    ltr(&original.cost.rials().to_string())
                );
                let source_index = order_data
                    .lines
                    .iter()
                    .position(|l| l.input.id == original.order_line_id)
                    .ok_or_else(|| anyhow::anyhow!("ردیف سفارش اصلی پیدا نشد"))?;
                let source = &order_data.lines[source_index].input;
                row.price = Some(source.unit_price);
                price_unit(&mut row, &resources, &source.unit_id)?;
                field(&mut row, "price_reference", &source.price_reference);
                field(&mut row, "price_date", &source.price_date);
                field(
                    &mut row,
                    "override_reason",
                    source.override_reason.as_deref().unwrap_or(""),
                );
                quotation(&mut row, order_input, Some(source_index))?;
                row.quantity = Some(line.quantity_thousandths);
                row.base_quantity = Some(line.quantity_thousandths);
                field(&mut row, "source_line_id", &line.fulfillment_line_id);
                field(
                    &mut row,
                    "original_quantity_thousandths",
                    original.quantity_thousandths,
                );
                field(&mut row, "original_value_irr", original.cost.rials());
                amount_breakdown(
                    &mut row,
                    line.gross,
                    line.line_discount,
                    line.document_discount,
                    line.net,
                    Some(line.cost),
                );
                row.snapshot = serde_json::to_string(line)?;
                report.rows.push(row);
            }
            report.rows.push(Row {
                kind: "document_total".into(),
                id: posted.credit_id.clone(),
                label: format!(
                    "جمع اعتبار فروش • بهای بازگشتی اصلی {} IRR",
                    ltr(&posted.cost.rials().to_string())
                ),
                amount: Some(posted.total),
                ..Row::default()
            });
        }
        _ => anyhow::bail!("نوع سند و تصویر ثبت‌شده سازگار نیست"),
    }
    report.rows.insert(0, metadata);
    for movement in &record.movements {
        let i = item(&resources, &movement.item_id)?;
        let mut row = source_row(record, "stock_movement", &movement.id);
        item_row(&mut row, &resources, i)?;
        let wh = warehouse(&mut row, &resources, &movement.warehouse_id, false)?;
        let u = unit(&mut row, &resources, &i.base_unit_id)?;
        row.label = format!(
            "{} • کد {} • انبار {} • واحد پایه {} • {}\nپیش از رویداد {} / {} IRR • پس از رویداد {} / {} IRR • ردیف منبع {}",
            i.name,
            ltr(&i.sku),
            wh,
            u,
            if movement.direction == StockDirection::In {
                "ورود"
            } else {
                "خروج"
            },
            ltr(&qty(movement.before.quantity_thousandths)),
            ltr(&movement.before.value.rials().to_string()),
            ltr(&qty(movement.after.quantity_thousandths)),
            ltr(&movement.after.value.rials().to_string()),
            ltr(&movement.source_line_id)
        );
        row.quantity = Some(movement.quantity_thousandths);
        row.base_quantity = Some(movement.quantity_thousandths);
        row.amount = Some(movement.value);
        field(
            &mut row,
            "direction",
            if movement.direction == StockDirection::In {
                "in"
            } else {
                "out"
            },
        );
        field(&mut row, "source_line_id", &movement.source_line_id);
        row.snapshot = serde_json::to_string(movement)?;
        report.rows.push(row);
    }
    report.snapshot = serde_json::to_string(
        &serde_json::json!({"document":record,"related_sources":related,"masters":selected_master_snapshot(&resources,&report.rows)}),
    )?;
    Ok(report)
}
fn reconciliation_report(state: &AccountingState, company_id: &str, as_of: &str) -> Result<Report> {
    let result = state.sales_reconcile(company_id, as_of)?;
    let resources = state.procurement_resources(company_id)?;
    let opening = state
        .core
        .openings
        .iter()
        .find(|o| o.command.company_id == company_id)
        .ok_or_else(|| anyhow::anyhow!("افتتاحیهٔ تطبیق‌شده وجود ندارد"))?;
    let mut report = Report {
        company_id: company_id.into(),
        company: company_name(state, company_id)?,
        as_of: as_of.into(),
        policy_version: opening.frozen_setup.command.policy.version.clone(),
        policy_label: opening.frozen_setup.command.policy.summary(),
        status: "posted_sales_procurement_including_opening".into(),
        status_label: if result.reconciled {
            "افتتاحیه و رویدادهای ثبت‌شده • کنترل‌ها تطبیق‌شده"
        } else {
            "اختلاف کنترل‌ها؛ نیازمند بررسی"
        }
        .into(),
        basis: "accrual_perpetual_end_of_day".into(),
        basis_label: "تعهدی / موجودی دائمی • مانده در پایان روز مالی • بدون تسویه و پرداخت".into(),
        kind: "sales_reconciliation".into(),
        source_id: format!("reconciliation:{company_id}:{as_of}"),
        title: "تطبیق فروش، دریافتنی و موجودی تا پایان روز".into(),
        rows: vec![],
        snapshot: String::new(),
    };
    for control in &result.controls {
        let (code, name) = account_name(state, company_id, &control.account_id)?;
        let mut row = Row {
            kind: "control".into(),
            id: control.role.key().into(),
            label: format!("کنترل {} • کد {}", name, ltr(&code)),
            ledger: Some(control.ledger),
            detail: Some(control.detail),
            difference: Some(control.difference),
            snapshot: serde_json::to_string(control)?,
            ..Row::default()
        };
        field(&mut row, "role", control.role.key());
        field(&mut row, "account_id", &control.account_id);
        field(&mut row, "account_code", code);
        field(&mut row, "account_name", name);
        report.rows.push(row);
    }
    let mut stock_total = Money::ZERO;
    for pool in &result.stock {
        let i = item(&resources, &pool.item_id)?;
        let mut row = Row {
            kind: "stock_pool".into(),
            id: format!("{}:{}", pool.item_id, pool.warehouse_id),
            quantity: Some(pool.quantity_thousandths),
            base_quantity: Some(pool.quantity_thousandths),
            amount: Some(pool.value),
            snapshot: serde_json::to_string(pool)?,
            ..Row::default()
        };
        item_row(&mut row, &resources, i)?;
        let wh = warehouse(&mut row, &resources, &pool.warehouse_id, false)?;
        let u = unit(&mut row, &resources, &i.base_unit_id)?;
        row.label = format!(
            "موجودی {} • کد {} • انبار {} • واحد پایه {} • نسبت دقیق {} IRR / {} هزارم واحد پایه",
            i.name,
            ltr(&i.sku),
            wh,
            u,
            ltr(&pool.value.rials().to_string()),
            ltr(&pool.quantity_thousandths.to_string())
        );
        stock_total = stock_total.checked_add(pool.value)?;
        report.rows.push(row);
    }
    report.rows.push(Row {
        kind: "stock_total".into(),
        id: "TOTAL-STOCK".into(),
        label: "جمع ارزش موجودی؛ مقدارهای واحدهای مختلف جمع نمی‌شوند".into(),
        amount: Some(stock_total),
        ..Row::default()
    });
    let mut receivables_total = Money::ZERO;
    for receivable in &result.receivables {
        let source = state.sales.records.iter().find(|r| {
            r.command.context().company_id == company_id
                && matches!(&r.data,SalesData::Fulfill(v) if v.invoice_id==receivable.id)
        });
        let party_name = if let Some(r) = source {
            let SalesData::Fulfill(v) = &r.data else {
                unreachable!()
            };
            v.customer.name.clone()
        } else {
            party(&mut Row::default(), &resources, &receivable.party_id)?
        };
        let mut row = Row {
            kind: "receivable_remaining".into(),
            id: receivable.id.clone(),
            source_id: receivable.source_id.clone(),
            event_id: source
                .map(|r| r.id.clone())
                .unwrap_or_else(|| opening.journal_id.clone()),
            reference: receivable.reference.clone(),
            amount: Some(receivable.remaining),
            snapshot: serde_json::to_string(receivable)?,
            ..Row::default()
        };
        field(&mut row, "party_id", &receivable.party_id);
        field(&mut row, "party_name", &party_name);
        field(&mut row, "document_date", &receivable.document_date);
        field(&mut row, "due_date", &receivable.due_date);
        field(&mut row, "original_value_irr", receivable.original.rials());
        field(&mut row, "invoice_id", &receivable.id);
        field(
            &mut row,
            "credit_ids_json",
            serde_json::to_string(&receivable.credit_ids)?,
        );
        row.label = format!(
            "ماندهٔ دریافتنی • مشتری {} • فاکتور {}\nتاریخ سند میلادی {} • شمسی {} • سررسید میلادی {} • شمسی {}\nمبلغ اصلی {} IRR • اعتبارهای پیوندشده {} • مبلغ زیر مانده است؛ بدون ثبت وصول وجه",
            party_name,
            ltr(&receivable.id),
            ltr(&receivable.document_date),
            ltr(&date(&receivable.document_date, DateDisplay::Jalali)?),
            ltr(&receivable.due_date),
            ltr(&date(&receivable.due_date, DateDisplay::Jalali)?),
            ltr(&receivable.original.rials().to_string()),
            ltr(&receivable.credit_ids.join(", "))
        );
        receivables_total = receivables_total.checked_add(receivable.remaining)?;
        report.rows.push(row);
    }
    for (kind, label, value) in [
        ("receivables_total", "جمع ماندهٔ دریافتنی", receivables_total),
        ("sales_total", "جمع فروش ثبت‌شده", result.sales),
        (
            "sales_returns_total",
            "جمع برگشت فروش ثبت‌شده",
            result.sales_returns,
        ),
        (
            "cogs_total",
            "بهای تمام‌شدهٔ فروش پس از برگشت بهای اصلی",
            result.cogs,
        ),
        (
            "profit_total",
            "حاشیهٔ فروش منهای برگشت و بهای فروش؛ سود کل شرکت نیست",
            result.profit,
        ),
        (
            "trial_total_debit",
            "جمع بدهکار تراز آزمایشی در همین پایان روز",
            result.total_debit,
        ),
        (
            "trial_total_credit",
            "جمع بستانکار تراز آزمایشی در همین پایان روز",
            result.total_credit,
        ),
    ] {
        report.rows.push(Row {
            kind: kind.into(),
            id: kind.into(),
            label: label.into(),
            amount: Some(value),
            ..Row::default()
        });
    }
    for (kind, label, value) in [
        ("typed_sales_total", "فروش ثبت‌شده", result.typed_sales),
        (
            "typed_sales_returns_total",
            "برگشت ناشی از اعتبارهای پیوندشده",
            result.typed_sales_returns,
        ),
        (
            "typed_cogs_total",
            "بهای ناشی از خروج و برگشت اصلی",
            result.typed_cogs,
        ),
    ] {
        report.rows.push(Row {
            kind: kind.into(),
            id: kind.into(),
            label: label.into(),
            amount: Some(value),
            ..Row::default()
        });
    }
    for adjustment in &result.adjustments {
        let mut row = Row {
            kind: "ordinary_journal_adjustment".into(),
            id: format!("{}:{}", adjustment.journal_id, adjustment.role.key()),
            event_id: adjustment.journal_id.clone(),
            amount: Some(adjustment.amount),
            snapshot: serde_json::to_string(adjustment)?,
            ..Row::default()
        };
        row.label = format!(
            "تعدیل سند عادی دفتر کل • سند {} • نقش {} • تاریخ میلادی {}؛ فاکتور یا برگشت خودکار نیست",
            ltr(&adjustment.journal_id),
            ltr(adjustment.role.key()),
            ltr(&adjustment.date)
        );
        field(&mut row, "role", adjustment.role.key());
        field(&mut row, "document_date", &adjustment.date);
        field(&mut row, "financial_effect_id", &adjustment.journal_id);
        field(&mut row, "financial_effect_kind", "ordinary_journal");
        report.rows.push(row);
    }
    let records = state
        .sales
        .records
        .iter()
        .filter(|r| {
            r.command.context().company_id == company_id
                && r.command.context().date.as_str() <= as_of
        })
        .collect::<Vec<_>>();
    for record in &records {
        let mut row = source_row(record, "source_event", &record.id);
        row.label = format!(
            "رویداد منبع {} • ترتیب {} • تاریخ مالی میلادی {} • {}",
            ltr(&record.command.context().source_id),
            ltr(&record.sequence.to_string()),
            ltr(&record.command.context().date),
            if record.journal_id.is_some() {
                "سند مالی ثبت‌شده"
            } else {
                "بدون سند مالی"
            }
        );
        field(&mut row, "sequence", record.sequence);
        field(&mut row, "document_date", &record.command.context().date);
        field(
            &mut row,
            "financial_effect_id",
            record.journal_id.as_deref().unwrap_or(""),
        );
        field(
            &mut row,
            "financial_effect_kind",
            if record.journal_id.is_some() {
                "journal"
            } else {
                "nonfinancial"
            },
        );
        row.snapshot = serde_json::to_string(record)?;
        report.rows.push(row);
    }
    let procurement = state
        .procurement
        .records
        .iter()
        .filter(|r| {
            r.command.context().company_id == company_id
                && r.command.context().date.as_str() <= as_of
        })
        .collect::<Vec<_>>();
    report.snapshot = serde_json::to_string(
        &serde_json::json!({"reconciliation":result,"opening":opening,"records":records,"procurement_sources":procurement,"masters":selected_master_snapshot(&resources,&report.rows)}),
    )?;
    Ok(report)
}
pub fn document_csv(
    state: &AccountingState,
    company_id: &str,
    document_id: &str,
    unit: MoneyUnit,
    dates: DateDisplay,
    digits: DisplayDigits,
    path: &Path,
) -> Result<()> {
    render_csv(
        &document_report(state, company_id, document_id)?,
        Preferences {
            unit,
            dates,
            digits,
        },
        path,
    )
}
pub fn document_pdf(
    state: &AccountingState,
    company_id: &str,
    document_id: &str,
    unit: MoneyUnit,
    dates: DateDisplay,
    digits: DisplayDigits,
    path: &Path,
) -> Result<()> {
    render_pdf(
        &document_report(state, company_id, document_id)?,
        Preferences {
            unit,
            dates,
            digits,
        },
        path,
    )
}
pub fn reconciliation_csv(
    state: &AccountingState,
    company_id: &str,
    as_of: &str,
    unit: MoneyUnit,
    dates: DateDisplay,
    digits: DisplayDigits,
    path: &Path,
) -> Result<()> {
    render_csv(
        &reconciliation_report(state, company_id, as_of)?,
        Preferences {
            unit,
            dates,
            digits,
        },
        path,
    )
}
pub fn reconciliation_pdf(
    state: &AccountingState,
    company_id: &str,
    as_of: &str,
    unit: MoneyUnit,
    dates: DateDisplay,
    digits: DisplayDigits,
    path: &Path,
) -> Result<()> {
    render_pdf(
        &reconciliation_report(state, company_id, as_of)?,
        Preferences {
            unit,
            dates,
            digits,
        },
        path,
    )
}
pub fn document_preview(
    state: &AccountingState,
    company_id: &str,
    document_id: &str,
    unit: MoneyUnit,
    dates: DateDisplay,
    digits: DisplayDigits,
) -> Result<DocumentPreview> {
    let report = document_report(state, company_id, document_id)?;
    let preferences = Preferences {
        unit,
        dates,
        digits,
    };
    Ok(DocumentPreview {
        title: report.title,
        company: report.company,
        as_of: digits.format(date(&report.as_of, dates)?),
        policy_version: report.policy_version,
        status: report.status_label,
        lines: report
            .rows
            .iter()
            .map(|r| human_line(r, preferences))
            .collect(),
    })
}

#[path = "sales_print.rs"]
mod printable;
pub use printable::printable_document_pdf;
pub use printable::printable_quotation_pdf;
