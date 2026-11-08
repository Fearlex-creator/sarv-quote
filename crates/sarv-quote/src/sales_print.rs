//! Customer-facing A4 sales documents. Financial values come from frozen records.
use super::*;
use crate::domain::{BusinessProfile, State};

fn text_layout(map: &pango::FontMap, cr: &cairo::Context, text: &str, width: f64, size: u32, bold: bool, rtl: bool) -> pango::Layout {
    let context = map.create_context();
    pangocairo::functions::update_context(cr, &context);
    pangocairo::functions::context_set_resolution(&context, 72.0);
    context.set_base_dir(if rtl { pango::Direction::Rtl } else { pango::Direction::Ltr });
    let layout = pango::Layout::new(&context);
    layout.set_auto_dir(false);
    layout.set_font_description(Some(&pango::FontDescription::from_string(&format!("Vazirmatn, sans-serif {} {size}", if bold { "Bold" } else { "" }))));
    layout.set_width((width * f64::from(pango::SCALE)) as i32);
    layout.set_wrap(pango::WrapMode::WordChar);
    layout.set_alignment(if rtl { pango::Alignment::Right } else { pango::Alignment::Left });
    layout.set_text(text);
    layout
}
fn paint(cr: &cairo::Context, layout: &pango::Layout, x: f64, y: f64) -> Result<f64> {
    cr.set_source_rgb(0.10, 0.12, 0.14);
    cr.move_to(x, y);
    pangocairo::functions::show_layout(cr, layout);
    cr.status()?;
    Ok(f64::from(layout.pixel_size().1))
}
fn label<'a>(rtl: bool, fa: &'a str, en: &'a str) -> &'a str { if rtl { fa } else { en } }

pub fn printable_document_pdf(state: &State, company_id: &str, document_id: &str, language: &str, path: &Path) -> Result<()> {
    let report = document_report(&state.accounting, company_id, document_id)?;
    let rtl = language == "fa";
    let prefs = Preferences { unit: if state.settings.default_currency == crate::domain::Unit::Rial { MoneyUnit::Rial } else { MoneyUnit::Toman }, dates: state.settings.date_display, digits: state.settings.display_digits };
    let profile = state.business_profiles.get(company_id).cloned().unwrap_or_else(|| BusinessProfile { phone: state.settings.business_phone.clone(), address: state.settings.business_address.clone(), terms: state.settings.default_terms.clone(), footer: state.settings.footer.clone(), ..BusinessProfile::default() });
    let metadata = report.rows.iter().find(|row| row.kind == "document_metadata").ok_or_else(|| anyhow::anyhow!("Document metadata missing"))?;
    let rows: Vec<_> = report.rows.iter().filter(|row| matches!(row.kind.as_str(), "order_line" | "fulfillment_line" | "return_line")).collect();
    let total = report.rows.iter().find(|row| row.kind == "document_total").and_then(|row| row.amount).ok_or_else(|| anyhow::anyhow!("Document total missing"))?;
    let mut discount = 0_i64;
    for row in &rows {
        for key in ["line_discount_irr", "document_discount_irr"] {
            discount = discount.checked_add(row.fields.get(key).map(|v| v.parse::<i64>()).transpose()?.unwrap_or(0)).ok_or_else(|| anyhow::anyhow!("Discount overflow"))?;
        }
    }
    let title = match report.kind.as_str() {
        "sales_order" => label(rtl, "سفارش فروش", "Sales order"),
        "delivery_note" => label(rtl, "حواله خروج", "Delivery note"),
        "sales_invoice" => label(rtl, "فاکتور فروش", "Sales invoice"),
        "sales_return" => label(rtl, "برگشت فروش", "Sales return"),
        _ => label(rtl, "برگه اعتبار فروش", "Sales credit note"),
    };
    let unit = label(rtl, if prefs.unit == MoneyUnit::Rial { "ریال" } else { "تومان" }, if prefs.unit == MoneyUnit::Rial { "IRR" } else { "toman" });
    let money = |value: Money| ltr(&prefs.digits.format(value.display(prefs.unit)));
    let surface = cairo::PdfSurface::for_stream(595.0, 842.0, Vec::<u8>::new())?;
    let cr = cairo::Context::new(&surface)?;
    let map = crate::pdf_fonts::font_map();
    let _fonts = crate::pdf_fonts::private(&map, "sales")?;
    let widths = [24.0, 174.0, 65.0, 85.0, 80.0, 95.0];
    let x_for = |index: usize| -> f64 { if rtl { 559.0 - widths[..=index].iter().sum::<f64>() } else { 36.0 + widths[..index].iter().sum::<f64>() } };
    let draw = |text: &str, y: f64, size: u32, bold: bool| -> Result<f64> { paint(&cr, &text_layout(&map, &cr, text, 523.0, size, bold, rtl), 36.0, y) };
    let table_header = |y: f64| -> Result<f64> {
        cr.set_source_rgb(0.93, 0.95, 0.95); cr.rectangle(36.0, y, 523.0, 34.0); cr.fill()?;
        let headers = ["#", label(rtl, "شرح کالا", "Item / description"), label(rtl, "مقدار", "Quantity"), label(rtl, "قیمت واحد", "Unit price"), label(rtl, "تخفیف", "Discount"), label(rtl, "مبلغ خالص", "Net amount")];
        for (index, header) in headers.iter().enumerate() { paint(&cr, &text_layout(&map, &cr, header, widths[index] - 10.0, 8, true, rtl), x_for(index) + 5.0, y + 8.0)?; }
        Ok(y + 34.0)
    };
    let page_header = |page: usize| -> Result<f64> {
        let mut y = 36.0;
        y += draw(title, y, 22, true)? + 8.0;
        y += draw(&report.company, y, 14, true)? + 6.0;
        let mut contacts = Vec::new();
        if !profile.phone.is_empty() { contacts.push(format!("{}: {}", label(rtl, "تلفن", "Phone"), ltr(&profile.phone))); }
        if !profile.registration_id.is_empty() { contacts.push(format!("{}: {}", label(rtl, "شناسه ملی / ثبت", "Registration ID"), ltr(&profile.registration_id))); }
        if !profile.tax_id.is_empty() { contacts.push(format!("{}: {}", label(rtl, "شناسه اقتصادی", "Tax ID"), ltr(&profile.tax_id))); }
        if !contacts.is_empty() { y += draw(&contacts.join("  ·  "), y, 9, false)? + 5.0; }
        if !profile.address.is_empty() { y += draw(&profile.address, y, 9, false)? + 5.0; }
        y += 8.0;
        let doc_date = metadata.fields.get("document_date").unwrap_or(&report.as_of);
        y += draw(&format!("{}: {}   ·   {}: {}", label(rtl, "شماره", "Reference"), ltr(if metadata.reference.is_empty() { document_id } else { &metadata.reference }), label(rtl, "تاریخ", "Date"), ltr(&prefs.digits.format(date(doc_date, prefs.dates)?))), y, 10, true)? + 5.0;
        y += draw(&format!("{}: {}", label(rtl, "خریدار", "Customer"), metadata.fields.get("party_name").map_or("", String::as_str)), y, 10, false)? + 5.0;
        if let Some(due) = metadata.fields.get("due_date") { y += draw(&format!("{}: {}", label(rtl, "سررسید", "Due date"), ltr(&prefs.digits.format(date(due, prefs.dates)?))), y, 9, false)? + 5.0; }
        y += draw(&format!("{}: {unit} · {} {}", label(rtl, "واحد مبلغ", "Amounts in"), label(rtl, "صفحه", "Page"), prefs.digits.format(page)), y, 9, false)? + 12.0;
        ensure!(y < 400.0, "Business details are too long for a printable header");
        table_header(y)
    };
    let footer = |page: usize| -> Result<()> {
        cr.set_source_rgb(0.75, 0.77, 0.79); cr.set_line_width(0.5); cr.move_to(36.0, 795.0); cr.line_to(559.0, 795.0); cr.stroke()?;
        draw(&format!("Sarv · {} {}", label(rtl, "صفحه", "Page"), prefs.digits.format(page)), 804.0, 8, false)?; Ok(())
    };
    let mut page = 1;
    let mut y = page_header(page)?;
    for (index, row) in rows.iter().enumerate() {
        let row_discount = row.fields.get("line_discount_irr").map(|v| v.parse::<i64>()).transpose()?.unwrap_or(0) + row.fields.get("document_discount_irr").map(|v| v.parse::<i64>()).transpose()?.unwrap_or(0);
        let name = row.fields.get("item_name").map_or("", String::as_str);
        let sku = row.fields.get("item_sku").map_or("", String::as_str);
        let qty_unit = row.fields.get("unit_name").map_or("", String::as_str);
        let price_unit = row.fields.get("price_unit_name").map_or(qty_unit, String::as_str);
        let description = format!("{name}\n{}{}", ltr(sku), if price_unit != qty_unit { format!("\n{}: {price_unit}", label(rtl, "واحد قیمت", "Price unit")) } else { String::new() });
        let cells = [prefs.digits.format(index + 1), description, format!("{}\n{qty_unit}", ltr(&prefs.digits.format(qty(row.quantity.unwrap_or(0))))), money(row.price.unwrap_or(Money::ZERO)), money(Money::from_rials(row_discount)), money(row.amount.unwrap_or(Money::ZERO))];
        let layouts: Vec<_> = cells.iter().enumerate().map(|(i, text)| text_layout(&map, &cr, text, widths[i] - 10.0, 9, false, rtl)).collect();
        let height = layouts.iter().map(|l| f64::from(l.pixel_size().1)).fold(28.0, f64::max) + 16.0;
        if y + height > 772.0 { footer(page)?; cr.show_page()?; page += 1; y = page_header(page)?; }
        ensure!(y + height <= 772.0, "Item description is too long for one printable row");
        for (i, layout) in layouts.iter().enumerate() { paint(&cr, layout, x_for(i) + 5.0, y + 8.0)?; }
        y += height;
        cr.set_source_rgb(0.85, 0.86, 0.87); cr.set_line_width(0.5); cr.move_to(36.0, y); cr.line_to(559.0, y); cr.stroke()?;
    }
    let summary = [format!("{}: {} {unit}", label(rtl, "جمع تخفیف", "Total discount"), money(Money::from_rials(discount))), format!("{}: {} {unit}", label(rtl, "مبلغ نهایی", "Total"), money(total)), label(rtl, "امضای فروشنده: __________________    امضای خریدار: __________________", "Seller signature: __________________    Buyer signature: __________________").to_owned()];
    y += 16.0;
    if y + 120.0 > 772.0 { footer(page)?; cr.show_page()?; page += 1; y = page_header(page)? + 12.0; }
    for (index, line) in summary.iter().enumerate() {
        if y + 42.0 > 772.0 { footer(page)?; cr.show_page()?; page += 1; y = page_header(page)? + 12.0; }
        y += draw(line, y, if index == 1 { 14 } else { 9 }, index == 1)? + 14.0;
    }
    for paragraph in [&profile.bank_details, &profile.terms, &profile.footer] {
        if paragraph.is_empty() { continue; }
        // Split explicit newlines before wrapping long prose so no terms can
        // overflow a page, including text pasted with many short lines.
        for line in paragraph.lines() {
            let chars: Vec<char> = line.chars().collect();
            for chunk in chars.chunks(220) {
                let text: String = chunk.iter().collect();
                let layout = text_layout(&map, &cr, &text, 523.0, 9, false, rtl);
                let height = f64::from(layout.pixel_size().1) + 8.0;
                if y + height > 772.0 { footer(page)?; cr.show_page()?; page += 1; y = page_header(page)? + 12.0; }
                y += paint(&cr, &layout, 36.0, y)? + 8.0;
            }
        }
    }
    footer(page)?; cr.show_page()?; surface.finish(); surface.status()?;
    let bytes = surface.finish_output_stream().map_err(|e| anyhow::anyhow!("{e}"))?.downcast::<Vec<u8>>().map_err(|_| anyhow::anyhow!("Invalid PDF stream"))?;
    write_new(path, &bytes)
}

/// Customer-facing A4 quotation rendered from a durable sales draft.
/// A quotation makes no financial posting; totals mirror draft arithmetic.
pub fn printable_quotation_pdf(state: &State, company_id: &str, draft_id: &str, language: &str, path: &Path) -> Result<()> {
    let draft = state
        .sales_drafts
        .iter()
        .find(|d| d.id == draft_id && d.company_id == company_id)
        .ok_or_else(|| anyhow::anyhow!("Quotation draft not found in this business"))?;
    let issue = draft
        .quote_issue
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Draft is not an issued quotation"))?;
    let company = state
        .accounting
        .companies
        .iter()
        .find(|c| c.id == company_id)
        .ok_or_else(|| anyhow::anyhow!("Business not found"))?;
    let resources = state.accounting.procurement_resources(company_id)?;
    let customer = resources
        .parties
        .iter()
        .find(|p| p.id == draft.customer_id)
        .ok_or_else(|| anyhow::anyhow!("Quotation customer not found"))?;
    let rtl = language == "fa";
    let prefs = Preferences { unit: draft.display_unit, dates: draft.date_display, digits: state.settings.display_digits };
    let profile = state.business_profiles.get(company_id).cloned().unwrap_or_else(|| BusinessProfile { phone: state.settings.business_phone.clone(), address: state.settings.business_address.clone(), terms: state.settings.default_terms.clone(), footer: state.settings.footer.clone(), ..BusinessProfile::default() });
    let unit = label(rtl, if prefs.unit == MoneyUnit::Rial { "ریال" } else { "تومان" }, if prefs.unit == MoneyUnit::Rial { "IRR" } else { "toman" });
    let money = |v: Money| ltr(&prefs.digits.format(v.display(prefs.unit)));
    // Same exact arithmetic as the draft preview: per-line rounded rial totals.
    let mut rows = Vec::new();
    let mut total = Money::ZERO;
    for (index, l) in draft.lines.iter().enumerate() {
        let item = resources.items.iter().find(|i| i.id == l.item_id).ok_or_else(|| anyhow::anyhow!("Quotation item not found"))?;
        let unit_name = resources.units.iter().find(|u| u.id == l.unit_id).map(|u| u.name.clone()).unwrap_or_default();
        let qty = crate::domain::quantity(&l.quantity)?;
        let price = Money::parse(&l.unit_price, draft.display_unit)?;
        let discount = Money::parse(&l.line_discount, draft.display_unit)?;
        let amount =
            Money::from_rials(crate::core::positive_half_up(
                i128::from(price.rials()) * i128::from(qty),
                1000,
            )?)
            .checked_sub(discount)?;
        total = total.checked_add(amount)?;
        rows.push((index + 1, item.name.clone(), item.sku.clone(), qty, unit_name, price, discount, amount));
    }
    let document_discount = Money::parse(&draft.document_discount, draft.display_unit)?;
    total = total.checked_sub(document_discount)?;
    let surface = cairo::PdfSurface::for_stream(595.0, 842.0, Vec::<u8>::new())?;
    let cr = cairo::Context::new(&surface)?;
    let map = crate::pdf_fonts::font_map();
    let _fonts = crate::pdf_fonts::private(&map, "sales")?;
    let widths = [24.0, 174.0, 65.0, 85.0, 80.0, 95.0];
    let x_for = |index: usize| -> f64 { if rtl { 559.0 - widths[..=index].iter().sum::<f64>() } else { 36.0 + widths[..index].iter().sum::<f64>() } };
    let draw = |text: &str, y: f64, size: u32, bold: bool| -> Result<f64> { paint(&cr, &text_layout(&map, &cr, text, 523.0, size, bold, rtl), 36.0, y) };
    let table_header = |y: f64| -> Result<f64> {
        cr.set_source_rgb(0.93, 0.95, 0.95); cr.rectangle(36.0, y, 523.0, 34.0); cr.fill()?;
        let headers = ["#", label(rtl, "شرح کالا", "Item / description"), label(rtl, "مقدار", "Quantity"), label(rtl, "قیمت واحد", "Unit price"), label(rtl, "تخفیف", "Discount"), label(rtl, "مبلغ خالص", "Net amount")];
        for (index, header) in headers.iter().enumerate() { paint(&cr, &text_layout(&map, &cr, header, widths[index] - 10.0, 8, true, rtl), x_for(index) + 5.0, y + 8.0)?; }
        Ok(y + 34.0)
    };
    let page_header = |page: usize| -> Result<f64> {
        let mut y = 36.0;
        y += draw(label(rtl, "پیش‌فاکتور", "Quotation"), y, 22, true)? + 8.0;
        y += draw(&company.name, y, 14, true)? + 6.0;
        let mut contacts = Vec::new();
        if !profile.phone.is_empty() { contacts.push(format!("{}: {}", label(rtl, "تلفن", "Phone"), ltr(&profile.phone))); }
        if !profile.registration_id.is_empty() { contacts.push(format!("{}: {}", label(rtl, "شناسه ملی / ثبت", "Registration ID"), ltr(&profile.registration_id))); }
        if !profile.tax_id.is_empty() { contacts.push(format!("{}: {}", label(rtl, "شناسه اقتصادی", "Tax ID"), ltr(&profile.tax_id))); }
        if !contacts.is_empty() { y += draw(&contacts.join("  ·  "), y, 9, false)? + 5.0; }
        if !profile.address.is_empty() { y += draw(&profile.address, y, 9, false)? + 5.0; }
        y += 8.0;
        y += draw(&format!("{}: {}   ·   {}: {}", label(rtl, "شماره", "Reference"), ltr(&issue.number), label(rtl, "تاریخ", "Date"), ltr(&prefs.digits.format(date(&issue.issued, prefs.dates)?))), y, 10, true)? + 5.0;
        y += draw(&format!("{}: {}", label(rtl, "خریدار", "Customer"), customer.name), y, 10, false)? + 5.0;
        if !issue.valid_until.is_empty() { y += draw(&format!("{}: {}", label(rtl, "اعتبار تا", "Valid until"), ltr(&prefs.digits.format(date(&issue.valid_until, prefs.dates)?))), y, 9, false)? + 5.0; }
        y += draw(&format!("{}: {unit} · {} {}", label(rtl, "واحد مبلغ", "Amounts in"), label(rtl, "صفحه", "Page"), prefs.digits.format(page)), y, 9, false)? + 12.0;
        ensure!(y < 400.0, "Business details are too long for a printable header");
        table_header(y)
    };
    let footer = |page: usize| -> Result<()> {
        cr.set_source_rgb(0.75, 0.77, 0.79); cr.set_line_width(0.5); cr.move_to(36.0, 795.0); cr.line_to(559.0, 795.0); cr.stroke()?;
        draw(&format!("Sarv · {} {}", label(rtl, "صفحه", "Page"), prefs.digits.format(page)), 804.0, 8, false)?; Ok(())
    };
    let mut page = 1;
    let mut y = page_header(page)?;
    for (index, name, sku, quantity_thousandths, unit_name, price, discount, amount) in &rows {
        let description = format!("{name}\n{}", ltr(sku));
        let cells = [prefs.digits.format(index), description, format!("{}\n{unit_name}", ltr(&prefs.digits.format(qty(*quantity_thousandths)))), money(*price), money(*discount), money(*amount)];
        let layouts: Vec<_> = cells.iter().enumerate().map(|(i, text)| text_layout(&map, &cr, text, widths[i] - 10.0, 9, false, rtl)).collect();
        let height = layouts.iter().map(|l| f64::from(l.pixel_size().1)).fold(28.0, f64::max) + 16.0;
        if y + height > 772.0 { footer(page)?; cr.show_page()?; page += 1; y = page_header(page)?; }
        ensure!(y + height <= 772.0, "Item description is too long for one printable row");
        for (i, layout) in layouts.iter().enumerate() { paint(&cr, layout, x_for(i) + 5.0, y + 8.0)?; }
        y += height;
        cr.set_source_rgb(0.85, 0.86, 0.87); cr.set_line_width(0.5); cr.move_to(36.0, y); cr.line_to(559.0, y); cr.stroke()?;
    }
    let summary = [
        if document_discount.rials() > 0 { format!("{}: {} {unit}", label(rtl, "تخفیف کل سند", "Document discount"), money(document_discount)) } else { String::new() },
        format!("{}: {} {unit}", label(rtl, "مبلغ نهایی", "Total"), money(total)),
        label(rtl, "این پیش‌فاکتور سند مالی نیست و پس از تأیید به سفارش فروش تبدیل می‌شود.", "This quotation is not a financial document; it becomes a sales order after approval.").to_owned(),
        label(rtl, "امضای فروشنده: __________________    امضای خریدار: __________________", "Seller signature: __________________    Buyer signature: __________________").to_owned(),
    ];
    y += 16.0;
    if y + 120.0 > 772.0 { footer(page)?; cr.show_page()?; page += 1; y = page_header(page)? + 12.0; }
    for (index, line) in summary.iter().enumerate() {
        if line.is_empty() { continue; }
        if y + 42.0 > 772.0 { footer(page)?; cr.show_page()?; page += 1; y = page_header(page)? + 12.0; }
        y += draw(line, y, if index == 1 { 14 } else { 9 }, index == 1)? + 14.0;
    }
    for paragraph in [&profile.bank_details, &profile.terms, &profile.footer] {
        if paragraph.is_empty() { continue; }
        for line in paragraph.lines() {
            let chars: Vec<char> = line.chars().collect();
            for chunk in chars.chunks(220) {
                let text: String = chunk.iter().collect();
                let layout = text_layout(&map, &cr, &text, 523.0, 9, false, rtl);
                let height = f64::from(layout.pixel_size().1) + 8.0;
                if y + height > 772.0 { footer(page)?; cr.show_page()?; page += 1; y = page_header(page)? + 12.0; }
                y += paint(&cr, &layout, 36.0, y)? + 8.0;
            }
        }
    }
    footer(page)?; cr.show_page()?; surface.finish(); surface.status()?;
    let bytes = surface.finish_output_stream().map_err(|e| anyhow::anyhow!("{e}"))?.downcast::<Vec<u8>>().map_err(|_| anyhow::anyhow!("Invalid PDF stream"))?;
    write_new(path, &bytes)
}
