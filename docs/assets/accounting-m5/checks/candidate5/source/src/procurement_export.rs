//! Exact, read-only procurement documents and end-of-day control snapshots.
//! Source records remain immutable; these reports make no statutory/tax claim.
use crate::{
    accounting::{AccountingState, Money, MoneyUnit},
    domain::{DateDisplay, DisplayDigits, ltr},
    export::safe_cell,
    store::write_new,
};
use anyhow::{Result, ensure};
use gtk::pango::{self, prelude::FontMapExt};
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
        "sarv-procurement-report-v1".into(),
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
        "order_line" | "receipt_line" | "bill_match" | "receipt_remaining"
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
            "\nقیمت هر واحد {} {} • دقیق IRR {}",
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
    let map = pangocairo::FontMap::new();
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
                if report.kind == "procurement_reconciliation" {
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

use crate::{core::CoreItem, procurement::*};
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
fn source_row(record: &ProcurementRecord, kind: &str, id: &str) -> Row {
    Row {
        kind: kind.into(),
        id: id.into(),
        event_id: record.id.clone(),
        source_id: record.command.context().source_id.clone(),
        ..Row::default()
    }
}
fn source_receipt<'a>(
    state: &'a AccountingState,
    company_id: &str,
    line_id: &str,
) -> Result<(&'a CoreItem, String, String, String)> {
    for record in &state.procurement.records {
        if record.command.context().company_id != company_id {
            continue;
        }
        if let ProcurementData::Receive(receipt) = &record.data
            && let Some(line) = receipt.lines.iter().find(|l| l.id == line_id)
        {
            let order = state.procurement_document(&receipt.order_id)?;
            if let ProcurementData::Order(order_data) = &order.data
                && let Some(order_line) = order_data
                    .lines
                    .iter()
                    .find(|l| l.input.id == line.order_line_id)
            {
                return Ok((
                    &order_line.item,
                    line.warehouse_id.clone(),
                    receipt.supplier_id.clone(),
                    record.id.clone(),
                ));
            }
        }
    }
    let opening = state
        .core
        .openings
        .iter()
        .find(|x| x.command.company_id == company_id)
        .ok_or_else(|| anyhow::anyhow!("افتتاحیهٔ منبع رسید پیدا نشد"))?;
    let receipt = opening
        .command
        .grni
        .iter()
        .find(|r| r.id == line_id)
        .ok_or_else(|| anyhow::anyhow!("رسید منبع گزارش پیدا نشد"))?;
    let item = opening
        .command
        .items
        .iter()
        .find(|i| i.id == receipt.item_id)
        .ok_or_else(|| anyhow::anyhow!("کالای تاریخی رسید پیدا نشد"))?;
    Ok((
        item,
        receipt.warehouse_id.clone(),
        receipt.party_id.clone(),
        opening.journal_id.clone(),
    ))
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
    let mut warehouses = ids("warehouse_id");
    warehouses.extend(ids("target_warehouse_id"));
    let mut units = ids("unit_id");
    units.extend(ids("base_unit_id"));
    for i in resources.items.iter().filter(|i| items.contains(&i.id)) {
        units.extend(i.packs.iter().map(|p| p.unit_id.clone()));
    }
    serde_json::json!({"parties":resources.parties.iter().filter(|x|parties.contains(&x.id)).collect::<Vec<_>>(),"items":resources.items.iter().filter(|x|items.contains(&x.id)).collect::<Vec<_>>(),"warehouses":resources.warehouses.iter().filter(|x|warehouses.contains(&x.id)).collect::<Vec<_>>(),"units":resources.units.iter().filter(|x|units.contains(&x.id)).collect::<Vec<_>>()})
}
fn document_report(state: &AccountingState, company_id: &str, document_id: &str) -> Result<Report> {
    let record = state.procurement_document(document_id)?;
    let context = record.command.context();
    ensure!(
        context.company_id == company_id,
        "سند متعلق به شرکت انتخاب‌شده نیست"
    );
    let resources = state.procurement_resources(company_id)?;
    let (kind, title) = match &record.data {
        ProcurementData::Resources(_) => ("resources", "تعریف منابع خرید و انبار"),
        ProcurementData::Order(_) => ("purchase_order", "سفارش خرید"),
        ProcurementData::Receive(_) => ("goods_receipt", "رسید کالا"),
        ProcurementData::Bill(_) => ("supplier_bill", "صورتحساب تأمین‌کننده"),
        ProcurementData::Transfer(_) => ("warehouse_transfer", "انتقال داخلی انبار"),
    };
    let mut report = Report {
        company_id: company_id.into(),
        company: company_name(state, company_id)?,
        as_of: context.date.clone(),
        policy_version: record.frozen_setup.command.policy.version.clone(),
        policy_label: record.frozen_setup.command.policy.summary(),
        status: if record.journal_id.is_some() {
            "posted_financial_document".into()
        } else {
            "recorded_nonfinancial_event".into()
        },
        status_label: if record.journal_id.is_some() {
            "سند مالی ثبت‌شده".into()
        } else {
            "رویداد غیرمالی ثبت‌شده؛ بدون سند مالی".into()
        },
        basis: "immutable_source_snapshot_at_recording".into(),
        basis_label: "تصویر ثابت سند در زمان ثبت • تعهدی / موجودی دائمی".into(),
        kind: kind.into(),
        source_id: context.source_id.clone(),
        title: title.into(),
        rows: vec![],
        snapshot: String::new(),
    };
    let mut metadata = source_row(record, "document_metadata", &record.id);
    metadata.label = format!(
        "ترتیب ثبت {} • {} • دلیل {}",
        ltr(&record.sequence.to_string()),
        record
            .journal_id
            .as_ref()
            .map(|j| format!("سند مالی ثبت‌شده\nشناسهٔ سند حسابداری {}\n", ltr(j)))
            .unwrap_or_else(|| "بدون اثر مالی دفتر کل".into()),
        context.reason
    );
    metadata.snapshot = serde_json::to_string(record)?;
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
        record.journal_id.as_deref().unwrap_or(&record.id),
    );
    let mut related = Vec::<&ProcurementRecord>::new();
    match (&record.command, &record.data) {
        (ProcurementCommand::Resources(input), ProcurementData::Resources(_)) => {
            for p in &input.parties {
                let mut row = source_row(record, "party", &p.id);
                row.label = format!(
                    "شخص {} • مشتری {} • تأمین‌کننده {}",
                    p.name,
                    if p.customer { "بله" } else { "خیر" },
                    if p.supplier { "بله" } else { "خیر" }
                );
                row.snapshot = serde_json::to_string(p)?;
                field(&mut row, "party_id", &p.id);
                field(&mut row, "party_name", &p.name);
                report.rows.push(row);
            }
            for i in &input.items {
                let mut row = source_row(record, "item", &i.id);
                row.label = format!("کالا {} • کد {}", i.name, ltr(&i.sku));
                item_row(&mut row, &resources, i)?;
                row.snapshot = serde_json::to_string(i)?;
                report.rows.push(row);
            }
            for u in &input.units {
                let mut row = source_row(record, "unit", &u.id);
                row.label = u.name.clone();
                field(&mut row, "unit_id", &u.id);
                field(&mut row, "unit_name", &u.name);
                row.snapshot = serde_json::to_string(u)?;
                report.rows.push(row);
            }
            for w in &input.warehouses {
                let mut row = source_row(record, "warehouse", &w.id);
                row.label = w.name.clone();
                warehouse(&mut row, &resources, &w.id, false)?;
                row.snapshot = serde_json::to_string(w)?;
                report.rows.push(row);
            }
        }
        (ProcurementCommand::Order(input), ProcurementData::Order(posted)) => {
            metadata.reference = input.reference.clone();
            field(&mut metadata, "document_date", &input.document_date);
            let name = party(&mut metadata, &resources, &input.supplier_id)?;
            metadata.label.push_str(&format!(
                " • تأمین‌کننده {} • تاریخ سند میلادی {}",
                name,
                ltr(&input.document_date)
            ));
            for line in &posted.lines {
                let mut row = source_row(record, "order_line", &line.input.id);
                item_row(&mut row, &resources, &line.item)?;
                let wh = warehouse(&mut row, &resources, &line.input.warehouse_id, false)?;
                let u = unit(&mut row, &resources, &line.input.unit_id)?;
                row.label = format!(
                    "{} • کد {} • انبار {} • واحد {} • ناخالص {} IRR • تخفیف ردیف {} IRR • سهم تخفیف سند {} IRR",
                    line.item.name,
                    ltr(&line.item.sku),
                    wh,
                    u,
                    ltr(&line.gross.rials().to_string()),
                    ltr(&line.input.line_discount.rials().to_string()),
                    ltr(&line.document_discount.rials().to_string())
                );
                row.label.push_str(&format!(
                    " • تاریخ مرجع قیمت میلادی {}{}",
                    ltr(&line.input.price_date),
                    pack_label(&resources, &line.item, &line.input.unit_id)?
                ));
                if let Some(reason) = &line.input.override_reason {
                    row.label.push_str(&format!(" • دلیل تغییر قیمت {reason}"));
                }
                row.quantity = Some(line.input.quantity_thousandths);
                row.base_quantity = Some(line.base_quantity_thousandths);
                row.price = Some(line.input.unit_price);
                row.amount = Some(line.net);
                row.reference = line.input.price_reference.clone();
                field(&mut row, "document_id", &record.id);
                field(&mut row, "source_line_id", &line.input.id);
                field(&mut row, "gross_irr", line.gross.rials());
                field(
                    &mut row,
                    "line_discount_irr",
                    line.input.line_discount.rials(),
                );
                field(
                    &mut row,
                    "document_discount_irr",
                    line.document_discount.rials(),
                );
                field(&mut row, "price_reference", &line.input.price_reference);
                field(&mut row, "price_date", &line.input.price_date);
                if let Some(reason) = &line.input.override_reason {
                    field(&mut row, "override_reason", reason);
                }
                row.snapshot = serde_json::to_string(line)?;
                report.rows.push(row);
            }
            report.rows.push(Row {
                kind: "document_total".into(),
                id: "TOTAL".into(),
                label: "جمع ارزش سفارش؛ تعهد خرید بدون ایجاد بدهی یا موجودی".into(),
                amount: Some(posted.total),
                ..Row::default()
            });
        }
        (ProcurementCommand::Receive(input), ProcurementData::Receive(posted)) => {
            metadata.reference = input.reference.clone();
            field(&mut metadata, "document_date", &input.document_date);
            let name = party(&mut metadata, &resources, &posted.supplier_id)?;
            metadata.label.push_str(&format!(
                " • تأمین‌کننده {} • سفارش {} • تاریخ سند میلادی {}",
                name,
                ltr(&posted.order_id),
                ltr(&input.document_date)
            ));
            let order_record = state.procurement_document(&posted.order_id)?;
            ensure!(
                order_record.command.context().company_id == company_id,
                "سفارش منبع متعلق به شرکت دیگری است"
            );
            related.push(order_record);
            let ProcurementData::Order(order) = &order_record.data else {
                anyhow::bail!("منبع رسید سفارش خرید نیست")
            };
            for line in &posted.lines {
                let selected = input
                    .lines
                    .iter()
                    .find(|l| l.id == line.id)
                    .ok_or_else(|| anyhow::anyhow!("ردیف اصلی رسید پیدا نشد"))?;
                let original = order
                    .lines
                    .iter()
                    .find(|l| l.input.id == line.order_line_id)
                    .ok_or_else(|| anyhow::anyhow!("ردیف سفارش منبع پیدا نشد"))?;
                let mut row = source_row(record, "receipt_line", &line.id);
                item_row(&mut row, &resources, &original.item)?;
                let wh = warehouse(&mut row, &resources, &line.warehouse_id, false)?;
                let u = unit(&mut row, &resources, &selected.unit_id)?;
                row.label = format!(
                    "{} • کد {} • انبار {} • واحد {} • ردیف سفارش {} • بهای منجمد دریافت‌شده",
                    original.item.name,
                    ltr(&original.item.sku),
                    wh,
                    u,
                    ltr(&line.order_line_id)
                );
                row.label
                    .push_str(&pack_label(&resources, &original.item, &selected.unit_id)?);
                row.quantity = Some(selected.quantity_thousandths);
                row.base_quantity = Some(line.quantity_thousandths);
                row.amount = Some(line.value);
                row.reference = input.reference.clone();
                field(&mut row, "document_id", &posted.order_id);
                field(&mut row, "source_line_id", &line.order_line_id);
                row.snapshot = serde_json::to_string(line)?;
                report.rows.push(row);
            }
            report.rows.push(Row {
                kind: "document_total".into(),
                id: "TOTAL".into(),
                label: "جمع ارزش رسید؛ موجودی در برابر کالای دریافت‌شده صورتحساب‌نشده".into(),
                amount: Some(posted.total),
                ..Row::default()
            });
        }
        (ProcurementCommand::Bill(input), ProcurementData::Bill(posted)) => {
            metadata.reference = input.reference.clone();
            field(&mut metadata, "document_date", &input.document_date);
            field(&mut metadata, "due_date", &input.due_date);
            let name = party(&mut metadata, &resources, &input.supplier_id)?;
            metadata.label.push_str(&format!(" • تأمین‌کننده {} • تاریخ سند میلادی {} • سررسید میلادی {} • کالا پیش‌تر دریافت شده؛ بدون دریافت دوباره",name,ltr(&input.document_date),ltr(&input.due_date)));
            for line in &posted.lines {
                let (i, wh_id, _, source) =
                    source_receipt(state, company_id, &line.receipt_line_id)?;
                let mut row = source_row(record, "bill_match", &line.id);
                item_row(&mut row, &resources, i)?;
                let wh = warehouse(&mut row, &resources, &wh_id, false)?;
                let u = unit(&mut row, &resources, &i.base_unit_id)?;
                row.label = format!(
                    "{} • کد {} • انبار منبع {} • واحد پایه {} • رسید {} • تطبیق هزینهٔ اصلی؛ بدون ورود دوبارهٔ کالا",
                    i.name,
                    ltr(&i.sku),
                    wh,
                    u,
                    ltr(&line.receipt_line_id)
                );
                row.quantity = Some(line.quantity_thousandths);
                row.base_quantity = Some(line.quantity_thousandths);
                row.amount = Some(line.value);
                row.reference = line.receipt_line_id.clone();
                field(&mut row, "source_line_id", &line.receipt_line_id);
                field(&mut row, "document_id", &source);
                row.snapshot = serde_json::to_string(line)?;
                report.rows.push(row);
                if let Ok(r) = state.procurement_document(&source) {
                    related.push(r);
                    if let ProcurementData::Receive(receipt) = &r.data {
                        related.push(state.procurement_document(&receipt.order_id)?);
                    }
                }
            }
            report.rows.push(Row {
                kind: "document_total".into(),
                id: "TOTAL".into(),
                label: "جمع صورتحساب؛ تسویهٔ کالای دریافت‌شده صورتحساب‌نشده در برابر حساب پرداختنی"
                    .into(),
                amount: Some(posted.total),
                ..Row::default()
            });
        }
        (ProcurementCommand::Transfer(input), ProcurementData::Transfer(posted)) => {
            let i = item(&resources, &input.item_id)?;
            let mut row = source_row(record, "transfer_summary", &record.id);
            item_row(&mut row, &resources, i)?;
            let from = warehouse(&mut row, &resources, &input.source_warehouse_id, false)?;
            let to = warehouse(&mut row, &resources, &input.target_warehouse_id, true)?;
            let u = unit(&mut row, &resources, &input.unit_id)?;
            row.label = format!(
                "{} • کد {} • از {} به {} • واحد {} • بهای حمل‌شدهٔ یکسان؛ دفتر کل شرکت تغییر نمی‌کند",
                i.name,
                ltr(&i.sku),
                from,
                to,
                u
            );
            row.label
                .push_str(&pack_label(&resources, i, &input.unit_id)?);
            row.quantity = Some(input.quantity_thousandths);
            row.base_quantity = Some(posted.quantity_thousandths);
            row.amount = Some(posted.value);
            row.snapshot = serde_json::to_string(posted)?;
            report.rows.push(row);
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
            "{} • کد {} • انبار {} • {} • واحد پایه {}\nپیش از رویداد {} / {} IRR • پس از رویداد {} / {} IRR • نسبت دقیق پیشین {}",
            i.name,
            ltr(&i.sku),
            wh,
            if movement.direction == StockDirection::In {
                "ورود"
            } else {
                "خروج"
            },
            u,
            ltr(&qty(movement.before.quantity_thousandths)),
            ltr(&movement.before.value.rials().to_string()),
            ltr(&qty(movement.after.quantity_thousandths)),
            ltr(&movement.after.value.rials().to_string()),
            ltr(&format!(
                "{} IRR / {} base-thousandths",
                movement.before.value.rials(),
                movement.before.quantity_thousandths
            ))
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
        row.reference = movement.source_line_id.clone();
        row.snapshot = serde_json::to_string(movement)?;
        report.rows.push(row);
    }
    related.sort_by(|a, b| a.id.cmp(&b.id));
    related.dedup_by(|a, b| a.id == b.id);
    report.snapshot = serde_json::to_string(
        &serde_json::json!({"document":record,"related_sources":related,"masters":selected_master_snapshot(&resources,&report.rows),"opening_source":if matches!(record.data,ProcurementData::Bill(_)){state.core.openings.iter().find(|o|o.command.company_id==company_id)}else{None}}),
    )?;
    Ok(report)
}
fn source_event(
    state: &AccountingState,
    company_id: &str,
    source: &str,
    row_id: &str,
    receipt: bool,
) -> Result<String> {
    if let Some(record) = state.procurement.records.iter().find(|r| {
        r.command.context().company_id == company_id
            && r.command.context().source_id == source
            && match &r.data {
                ProcurementData::Receive(data) if receipt => {
                    data.lines.iter().any(|line| line.id == row_id)
                }
                ProcurementData::Bill(_) if !receipt => r.id == row_id,
                _ => false,
            }
    }) {
        return Ok(record.id.clone());
    }
    let opening = state
        .core
        .openings
        .iter()
        .find(|o| {
            o.command.company_id == company_id
                && o.command.source_id == source
                && if receipt {
                    o.command.grni.iter().any(|line| line.id == row_id)
                } else {
                    o.command.open_items.iter().any(|line| line.id == row_id)
                }
        })
        .ok_or_else(|| anyhow::anyhow!("شناسهٔ رویداد منبع گزارش پیدا نشد؛ مرجع قابل حدس نیست"))?;
    Ok(opening.journal_id.clone())
}
fn receipt_document_date<'a>(
    state: &'a AccountingState,
    company_id: &str,
    line_id: &str,
) -> Result<&'a str> {
    for record in &state.procurement.records {
        if record.command.context().company_id != company_id {
            continue;
        }
        if let (ProcurementCommand::Receive(input), ProcurementData::Receive(posted)) =
            (&record.command, &record.data)
            && posted.lines.iter().any(|l| l.id == line_id)
        {
            return Ok(&input.document_date);
        }
    }
    state
        .core
        .openings
        .iter()
        .find(|o| o.command.company_id == company_id)
        .and_then(|o| o.command.grni.iter().find(|l| l.id == line_id))
        .map(|l| l.received_date.as_str())
        .ok_or_else(|| anyhow::anyhow!("تاریخ اصلی رسید منبع پیدا نشد"))
}
fn reconciliation_report(state: &AccountingState, company_id: &str, as_of: &str) -> Result<Report> {
    let result = state.procurement_reconcile(company_id, as_of)?;
    let resources = state.procurement_resources(company_id)?;
    let opening = state
        .core
        .openings
        .iter()
        .find(|o| o.command.company_id == company_id)
        .ok_or_else(|| anyhow::anyhow!("افتتاحیهٔ تطبیق‌شده وجود ندارد؛ صفر تأییدشده نیست"))?;
    let mut report = Report {
        company_id: company_id.into(),
        company: company_name(state, company_id)?,
        as_of: as_of.into(),
        policy_version: opening.frozen_setup.command.policy.version.clone(),
        policy_label: opening.frozen_setup.command.policy.summary(),
        status: "posted_procurement_including_opening".into(),
        status_label: if result.reconciled {
            "افتتاحیه و رویدادهای ثبت‌شده • کنترل‌ها تطبیق‌شده".into()
        } else {
            "اختلاف کنترل‌ها؛ نیازمند بررسی".into()
        },
        basis: "accrual_perpetual_end_of_day".into(),
        basis_label: "تعهدی / موجودی دائمی • مانده در پایان روز مالی".into(),
        kind: "procurement_reconciliation".into(),
        source_id: format!("reconciliation:{company_id}:{as_of}"),
        title: "تطبیق خرید و موجودی تا پایان روز".into(),
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
    let mut quantities: BTreeMap<String, i64> = BTreeMap::new();
    let mut total_value = Money::ZERO;
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
            "موجودی {} • کد {} • انبار {} • واحد پایه {} • نسبت دقیق {}",
            i.name,
            ltr(&i.sku),
            wh,
            u,
            ltr(&format!(
                "{} IRR / {} base-thousandths",
                pool.value.rials(),
                pool.quantity_thousandths
            ))
        );
        let total = quantities.entry(i.base_unit_id.clone()).or_default();
        *total = total
            .checked_add(pool.quantity_thousandths)
            .ok_or_else(|| anyhow::anyhow!("سرریز جمع مقدار در یک واحد پایه"))?;
        total_value = total_value.checked_add(pool.value)?;
        report.rows.push(row);
    }
    let total_quantity = if quantities.len() == 1 {
        quantities.values().next().copied()
    } else {
        None
    };
    let total_unit = if quantities.len() == 1 {
        resources
            .units
            .iter()
            .find(|u| Some(&u.id) == quantities.keys().next())
            .map(|u| u.name.as_str())
            .unwrap_or("")
    } else {
        ""
    };
    report.rows.push(Row {
        kind: "stock_total".into(),
        id: "TOTAL-STOCK".into(),
        label: if total_unit.is_empty() {
            "جمع ارزش موجودی شرکت؛ مقدارها با واحدهای پایه متفاوت جمع نمی‌شوند".into()
        } else {
            format!("جمع مقدار پایه و ارزش موجودی شرکت • واحد پایه {total_unit}")
        },
        quantity: total_quantity,
        base_quantity: total_quantity,
        amount: Some(total_value),
        ..Row::default()
    });
    for receipt in &result.receipts {
        let i = item(&resources, &receipt.item_id)?;
        let mut row = Row {
            kind: "receipt_remaining".into(),
            id: receipt.id.clone(),
            source_id: receipt.source_id.clone(),
            event_id: source_event(state, company_id, &receipt.source_id, &receipt.id, true)?,
            reference: receipt.reference.clone(),
            quantity: Some(receipt.original_quantity_thousandths),
            base_quantity: Some(receipt.original_quantity_thousandths),
            remaining_quantity: Some(receipt.quantity_thousandths),
            amount: Some(receipt.value),
            snapshot: serde_json::to_string(receipt)?,
            ..Row::default()
        };
        item_row(&mut row, &resources, i)?;
        let name = party(&mut row, &resources, &receipt.supplier_id)?;
        let wh = warehouse(&mut row, &resources, &receipt.warehouse_id, false)?;
        let u = unit(&mut row, &resources, &i.base_unit_id)?;
        let document_date = receipt_document_date(state, company_id, &receipt.id)?;
        field(&mut row, "document_date", document_date);
        field(&mut row, "availability_date", &receipt.date);
        field(&mut row, "source_line_id", &receipt.id);
        field(
            &mut row,
            "original_quantity_thousandths",
            receipt.original_quantity_thousandths,
        );
        field(
            &mut row,
            "original_value_irr",
            receipt.original_value.rials(),
        );
        row.label = format!(
            "دریافت صورتحساب‌نشده • {} • کد {} • تأمین‌کننده {} • انبار منبع {} • واحد پایه {}\nارزش اولیه {} IRR • تاریخ سند منبع میلادی {} • تاریخ دسترسی مالی میلادی {} • مبلغ نمایش‌شده ارزش باقی‌مانده است",
            i.name,
            ltr(&i.sku),
            name,
            wh,
            u,
            ltr(&receipt.original_value.rials().to_string()),
            ltr(document_date),
            ltr(&receipt.date)
        );
        report.rows.push(row);
    }
    for payable in &result.payables {
        let mut row = Row {
            kind: "payable_remaining".into(),
            id: payable.id.clone(),
            source_id: payable.source_id.clone(),
            event_id: source_event(state, company_id, &payable.source_id, &payable.id, false)?,
            reference: payable.reference.clone(),
            amount: Some(payable.remaining),
            snapshot: serde_json::to_string(payable)?,
            ..Row::default()
        };
        let name = party(&mut row, &resources, &payable.party_id)?;
        field(&mut row, "document_date", &payable.document_date);
        field(&mut row, "due_date", &payable.due_date);
        row.label = format!(
            "ماندهٔ پرداختنی • {} • تاریخ سند میلادی {} • سررسید میلادی {}",
            name,
            ltr(&payable.document_date),
            ltr(&payable.due_date)
        );
        report.rows.push(row);
    }
    for (kind, value) in [
        ("trial_total_debit", result.total_debit),
        ("trial_total_credit", result.total_credit),
    ] {
        report.rows.push(Row {
            kind: kind.into(),
            id: kind.into(),
            label: if kind == "trial_total_debit" {
                "جمع بدهکار تراز آزمایشی در همین پایان روز".into()
            } else {
                "جمع بستانکار تراز آزمایشی در همین پایان روز".into()
            },
            amount: Some(value),
            ..Row::default()
        });
    }
    let records = state
        .procurement
        .records
        .iter()
        .filter(|r| {
            r.command.context().company_id == company_id
                && r.command.context().date.as_str() <= as_of
        })
        .collect::<Vec<_>>();
    for record in &records {
        let context = record.command.context();
        let mut row = source_row(record, "source_event", &record.id);
        row.label = format!(
            "رویداد منبع • ترتیب {} • تاریخ مالی میلادی {} • {}",
            ltr(&record.sequence.to_string()),
            ltr(&context.date),
            if record.journal_id.is_some() {
                "سند مالی ثبت‌شده"
            } else {
                "غیرمالی؛ بدون سند مالی"
            }
        );
        field(&mut row, "sequence", record.sequence);
        field(&mut row, "document_date", &context.date);
        field(
            &mut row,
            "financial_effect_kind",
            if record.journal_id.is_some() {
                "journal"
            } else {
                "nonfinancial"
            },
        );
        field(
            &mut row,
            "financial_effect_id",
            record.journal_id.as_deref().unwrap_or(&record.id),
        );
        row.snapshot = serde_json::to_string(record)?;
        report.rows.push(row);
        for movement in &record.movements {
            let i = item(&resources, &movement.item_id)?;
            let mut row = source_row(record, "source_stock_movement", &movement.id);
            item_row(&mut row, &resources, i)?;
            let wh = warehouse(&mut row, &resources, &movement.warehouse_id, false)?;
            unit(&mut row, &resources, &i.base_unit_id)?;
            row.label = format!(
                "{} • کد {} • انبار {} • {} • قبل {} / {} IRR • بعد {} / {} IRR",
                i.name,
                ltr(&i.sku),
                wh,
                if movement.direction == StockDirection::In {
                    "ورود"
                } else {
                    "خروج"
                },
                ltr(&qty(movement.before.quantity_thousandths)),
                ltr(&movement.before.value.rials().to_string()),
                ltr(&qty(movement.after.quantity_thousandths)),
                ltr(&movement.after.value.rials().to_string())
            );
            row.quantity = Some(movement.quantity_thousandths);
            row.base_quantity = Some(movement.quantity_thousandths);
            row.amount = Some(movement.value);
            row.reference = movement.source_line_id.clone();
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
    }
    report.snapshot = serde_json::to_string(
        &serde_json::json!({"reconciliation":result,"opening":opening,"records":records,"masters":selected_master_snapshot(&resources,&report.rows)}),
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
