use crate::{domain::*, store::write_new};
use anyhow::{Context, Result, ensure};
use gtk::pango;
use gtk::pango::prelude::FontMapExt;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};

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

const PAGE_W: f64 = 595.0;
const PAGE_H: f64 = 842.0;
const LEFT: f64 = 36.0;
const WIDTH: f64 = PAGE_W - LEFT * 2.0;
const BOTTOM: f64 = 774.0;
const INK: (f64, f64, f64) = (0.12, 0.14, 0.16);
const MUTED: (f64, f64, f64) = (0.38, 0.41, 0.44);
const RULE: (f64, f64, f64) = (0.82, 0.84, 0.86);

#[derive(Clone)]
struct Text {
    value: String,
    x: f64,
    y: f64,
    width: f64,
    size: f64,
    bold: bool,
    align: pango::Alignment,
    color: (f64, f64, f64),
}
#[derive(Clone)]
enum Element {
    Text(Text),
    Rule {
        x: f64,
        y: f64,
        width: f64,
    },
    Fill {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
}
#[derive(Default)]
struct Page {
    elements: Vec<Element>,
}
struct Document {
    pages: Vec<Page>,
}
thread_local! {
    static FONT_MAP: RefCell<Option<pango::FontMap>> = const { RefCell::new(None) };
}
struct FontScope(Option<pango::FontMap>);
impl FontScope {
    fn new() -> Self {
        Self(FONT_MAP.with(|map| map.replace(Some(pangocairo::FontMap::new()))))
    }
}
impl Drop for FontScope {
    fn drop(&mut self) {
        FONT_MAP.with(|map| map.replace(self.0.take()));
    }
}
fn layout(cr: &cairo::Context, text: &Text) -> pango::Layout {
    // Each measurement/render scope owns a font map. Reuse within one surface
    // is fast, while PDF and scaled raster contexts never share glyph caches.
    let font_map = FONT_MAP
        .with(|map| map.borrow().clone())
        .unwrap_or_else(pangocairo::FontMap::new);
    let context = font_map.create_context();
    pangocairo::functions::update_context(cr, &context);
    let layout = pango::Layout::new(&context);
    pangocairo::functions::context_set_resolution(&layout.context(), 72.0);
    layout.context().set_base_dir(pango::Direction::Rtl);
    layout.set_auto_dir(false);
    layout.set_font_description(Some(&pango::FontDescription::from_string(&format!(
        "Vazirmatn, Noto Sans Arabic {}{}",
        if text.bold { "Bold " } else { "" },
        text.size
    ))));
    layout.set_width((text.width * f64::from(pango::SCALE)) as i32);
    layout.set_alignment(text.align);
    layout.set_wrap(pango::WrapMode::WordChar);
    layout.set_text(&text.value);
    layout
}
fn label(value: impl Into<String>, x: f64, y: f64, width: f64, size: f64) -> Text {
    Text {
        value: value.into(),
        x,
        y,
        width,
        size,
        bold: false,
        align: pango::Alignment::Right,
        color: INK,
    }
}
fn height(cr: &cairo::Context, text: &Text) -> f64 {
    f64::from(layout(cr, text).pixel_size().1)
}
fn add(page: &mut Page, text: Text) {
    page.elements.push(Element::Text(text));
}
fn rule(page: &mut Page, x: f64, y: f64, width: f64) {
    page.elements.push(Element::Rule { x, y, width });
}
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
fn page_heading(q: &Quote, cr: &cairo::Context, first: bool) -> Result<(Page, f64)> {
    let mut page = Page::default();
    let mut title = label(
        "پیش‌فاکتور",
        300.0,
        30.0,
        259.0,
        if first { 22.0 } else { 15.0 },
    );
    title.bold = true;
    add(&mut page, title);
    let mut brand = label(
        if q.issuer_name.is_empty() {
            "سرو / پیش‌فاکتور"
        } else {
            &q.issuer_name
        },
        LEFT,
        36.0,
        220.0,
        10.0,
    );
    brand.bold = !q.issuer_name.is_empty();
    let brand_bottom = 36.0 + height(cr, &brand);
    brand.align = pango::Alignment::Left;
    brand.color = MUTED;
    add(&mut page, brand);
    let status = if q.approved_at.is_empty() {
        "پیش‌نمایش — تأیید نشده".to_string()
    } else {
        format!("نسخهٔ تأییدشدهٔ {}", document_digits(q, q.version))
    };
    let mut status_text = label(status, 300.0, if first { 66.0 } else { 57.0 }, 259.0, 9.0);
    status_text.color = MUTED;
    add(&mut page, status_text);
    if !first {
        let mut identification = label(
            number(q),
            LEFT,
            59.0_f64.max(brand_bottom + 2.0),
            250.0,
            7.5,
        );
        identification.align = pango::Alignment::Left;
        identification.color = MUTED;
        add(&mut page, identification);
        let separator_y = 85.0_f64.max(brand_bottom + 25.0);
        ensure!(separator_y < 220.0, "نام کسب‌وکار برای سربرگ بسیار بلند است");
        rule(&mut page, LEFT, separator_y, WIDTH);
        return Ok((page, separator_y + 14.0));
    }
    let separator_y = 82.0_f64.max(brand_bottom + 10.0);
    rule(&mut page, LEFT, separator_y, WIDTH);
    let mut customer_y = separator_y + 10.0;
    for value in [
        if q.issuer_phone.is_empty() {
            String::new()
        } else {
            format!(
                "تلفن کسب‌وکار: {}",
                ltr(&document_digits(q, &q.issuer_phone))
            )
        },
        if q.issuer_address.is_empty() {
            String::new()
        } else {
            format!("نشانی: {}", q.issuer_address)
        },
        q.header.clone(),
    ] {
        if !value.is_empty() {
            let mut block = label(value, LEFT, customer_y, WIDTH, 8.5);
            block.color = MUTED;
            let block_height = height(cr, &block);
            if block_height <= 80.0 {
                customer_y += block_height + 3.0;
                add(&mut page, block);
            }
        }
    }
    if !q.issuer_phone.is_empty() || !q.issuer_address.is_empty() || !q.header.is_empty() {
        customer_y += 3.0;
    }
    let mut customer = label(
        format!("مشتری: {}", q.customer),
        LEFT,
        customer_y,
        WIDTH,
        12.0,
    );
    customer.bold = true;
    let mut y = customer_y + height(cr, &customer) + 6.0;
    add(&mut page, customer);
    for block in customer_blocks(q) {
        if !block.is_empty() {
            let text = label(block, LEFT, y, WIDTH, 9.0);
            let block_height = height(cr, &text);
            if block_height <= 80.0 {
                y += block_height + 3.0;
                add(&mut page, text);
            }
        }
    }
    let metadata = [
        (
            "تاریخ پیش‌فاکتور",
            if q.quote_date.is_empty() {
                "ثبت نشده".into()
            } else {
                ltr(&display_date(q, &q.quote_date))
            },
        ),
        (
            "اعتبار تا",
            if q.valid_until.is_empty() {
                "تعیین نشده".into()
            } else {
                ltr(&display_date(q, &q.valid_until))
            },
        ),
        (
            "مرجع سفارش",
            if q.reference.is_empty() {
                "—".into()
            } else {
                q.reference.clone()
            },
        ),
        (
            "تلفن",
            if q.phone.is_empty() {
                "—".into()
            } else {
                ltr(&document_digits(q, &q.phone))
            },
        ),
        ("فهرست قیمت", q.catalog_name.clone()),
        (
            if q.number.is_empty() {
                "شناسهٔ پیش‌فاکتور"
            } else {
                "شمارهٔ پیش‌فاکتور"
            },
            number(q),
        ),
    ];
    for pair in metadata.chunks(2) {
        let right = label(
            format!("{}: {}", pair[0].0, pair[0].1),
            307.0,
            y,
            252.0,
            8.5,
        );
        let left = label(format!("{}: {}", pair[1].0, pair[1].1), LEFT, y, 253.0, 8.5);
        y += height(cr, &right).max(height(cr, &left)) + 3.0;
        add(&mut page, right);
        add(&mut page, left);
    }
    ensure!(
        y < BOTTOM - 90.0,
        "اطلاعات سربرگ برای یک صفحه بسیار بلند است"
    );
    Ok((page, y + 10.0))
}
fn customer_blocks(q: &Quote) -> [String; 3] {
    [
        if q.customer_contact.is_empty() {
            String::new()
        } else {
            format!("شخص رابط: {}", q.customer_contact)
        },
        if q.customer_email.is_empty() {
            String::new()
        } else {
            format!("ایمیل: {}", ltr(&q.customer_email))
        },
        if q.customer_address.is_empty() {
            String::new()
        } else {
            format!("نشانی مشتری: {}", q.customer_address)
        },
    ]
}
// The monetary unit belongs to the whole table and is always explicit. Column
// positions run left-to-right; their semantic order is RTL, with item at right.
const COLS: [(f64, f64); 6] = [
    (36.0, 97.0),
    (133.0, 97.0),
    (230.0, 43.0),
    (273.0, 49.0),
    (322.0, 201.0),
    (523.0, 36.0),
];
fn table_heading(page: &mut Page, y: f64, unit: Unit) -> f64 {
    page.elements.push(Element::Fill {
        x: LEFT,
        y,
        width: WIDTH,
        height: 35.0,
    });
    let titles = [
        format!("مبلغ ({})", unit.label()),
        format!("قیمت واحد ({})", unit.label()),
        "واحد".into(),
        "تعداد".into(),
        "شرح کالا / کد".into(),
        "ردیف".into(),
    ];
    for (index, ((x, width), title)) in COLS.iter().zip(titles).enumerate() {
        let mut text = label(title, x + 5.0, y + 8.0, width - 10.0, 8.0);
        text.bold = true;
        if matches!(index, 2 | 3 | 5) {
            text.align = pango::Alignment::Center;
        }
        add(page, text);
    }
    rule(page, LEFT, y + 35.0, WIDTH);
    y + 35.0
}
fn row(cr: &cairo::Context, q: &Quote, index: usize, y: f64) -> (Vec<Text>, f64) {
    let l = &q.lines[index];
    let values = [
        amount(q, l.total_rial),
        amount(q, l.price_rial),
        l.measure.clone(),
        document_digits(q, qty_display(l.quantity_milli)),
        l.name.clone(),
        document_digits(q, index + 1),
    ];
    let mut cells = Vec::new();
    let mut row_height: f64 = 34.0;
    for (column, ((x, width), value)) in COLS.iter().zip(values).enumerate() {
        let mut text = label(value, x + 5.0, y + 6.0, width - 10.0, 9.5);
        if matches!(column, 2 | 3 | 5) {
            text.align = pango::Alignment::Center;
        }
        if column == 4 {
            let name_height = height(cr, &text);
            let mut code = label(
                ltr(&l.code),
                x + 5.0,
                y + 6.0 + name_height + 1.0,
                width - 10.0,
                8.5,
            );
            code.color = MUTED;
            let code_height = height(cr, &code);
            let mut source = label(
                format!(
                    "منبع: {} · {}",
                    ltr(&display_timestamp(q, &l.source_at)),
                    l.source_unit.label()
                ),
                x + 5.0,
                code.y + code_height + 1.0,
                width - 10.0,
                7.5,
            );
            source.color = MUTED;
            row_height = row_height.max(source.y - y + height(cr, &source) + 6.0);
            cells.push(code);
            cells.push(source);
        }
        row_height = row_height.max(height(cr, &text) + 12.0);
        cells.push(text);
    }
    (cells, row_height)
}
fn next_page(doc: &mut Document, q: &Quote, cr: &cairo::Context, table: bool) -> Result<f64> {
    let (mut page, y) = page_heading(q, cr, false)?;
    let y = if table {
        table_heading(&mut page, y, q.display_unit)
    } else {
        y
    };
    doc.pages.push(page);
    Ok(y)
}
fn paragraph(
    doc: &mut Document,
    q: &Quote,
    cr: &cairo::Context,
    value: &str,
    y: &mut f64,
    size: f64,
) -> Result<()> {
    // Pango determines the actual word wraps. Drawing one shaped line at a time
    // permits unlimited notes to continue on following pages without clipping.
    let template = label(value, LEFT, 0.0, WIDTH, size);
    let wrapped = layout(cr, &template);
    for line in wrapped.lines_readonly() {
        let start = line.start_index() as usize;
        let end = start + line.length() as usize;
        let mut text = label(
            value[start..end].trim_end_matches('\n'),
            LEFT,
            *y,
            WIDTH,
            size,
        );
        text.color = MUTED;
        let h = height(cr, &text).max(size + 5.0);
        if *y + h > BOTTOM {
            *y = next_page(doc, q, cr, false)?;
            text.y = *y;
        }
        add(doc.pages.last_mut().unwrap(), text);
        *y += h + 2.0;
    }
    Ok(())
}
fn document(q: &Quote) -> Result<Document> {
    let _fonts = FontScope::new();
    let subtotal = validate_amounts(q)?;
    let surface = cairo::RecordingSurface::create(cairo::Content::ColorAlpha, None)?;
    let cr = cairo::Context::new(&surface)?;
    let (page, mut y) = page_heading(q, &cr, true)?;
    let mut doc = Document { pages: vec![page] };
    for (title, value) in [
        ("نشانی کسب‌وکار", &q.issuer_address),
        ("سربرگ کسب‌وکار", &q.header),
    ] {
        let display = if title == "نشانی کسب‌وکار" {
            format!("نشانی: {value}")
        } else {
            value.clone()
        };
        if !value.trim().is_empty() && height(&cr, &label(display, LEFT, 0.0, WIDTH, 8.5)) > 80.0 {
            paragraph(&mut doc, q, &cr, title, &mut y, 9.0)?;
            paragraph(&mut doc, q, &cr, value, &mut y, 8.5)?;
            y += 10.0;
        }
    }
    for block in customer_blocks(q) {
        if !block.is_empty() && height(&cr, &label(&block, LEFT, 0.0, WIDTH, 9.0)) > 80.0 {
            paragraph(&mut doc, q, &cr, &block, &mut y, 9.0)?;
            y += 10.0;
        }
    }
    if y + 75.0 > BOTTOM {
        y = next_page(&mut doc, q, &cr, false)?;
    }
    y = table_heading(doc.pages.last_mut().unwrap(), y, q.display_unit);
    for index in 0..q.lines.len() {
        let (_, row_height) = row(&cr, q, index, y);
        ensure!(
            row_height <= BOTTOM - 134.0,
            "ردیف {} برای یک صفحه بسیار بلند است؛ متن یا واحد کالا را کوتاه کنید",
            fa(index + 1)
        );
        if y + row_height > BOTTOM {
            y = next_page(&mut doc, q, &cr, true)?;
        }
        let (cells, _) = row(&cr, q, index, y);
        let page = doc.pages.last_mut().unwrap();
        for cell in cells {
            add(page, cell);
        }
        y += row_height;
        rule(page, LEFT, y, WIDTH);
    }
    y += 12.0;
    // Keep monetary rows together; the configured tax is an explicit optional
    // whole-document amount after discount, never a guessed statutory rate.
    let mut totals = vec![
        ("جمع کالاها".to_string(), subtotal),
        ("تخفیف کل".to_string(), q.discount_rial),
    ];
    if q.tax_basis_points != 0 {
        totals.push((tax_label(q), q.tax_rial));
    }
    totals.push(("مبلغ نهایی".to_string(), q.total_rial));
    let last = totals.len() - 1;
    let totals_height = totals
        .iter()
        .enumerate()
        .map(|(index, (title, value))| {
            let size = if index == last { 11.0 } else { 9.0 };
            height(&cr, &label(title, LEFT + 133.0, 0.0, 127.0, size)).max(height(
                &cr,
                &label(display_amount(q, *value), LEFT, 0.0, 130.0, size),
            )) + 5.0
        })
        .sum::<f64>()
        + 5.0;
    if y + totals_height > BOTTOM {
        y = next_page(&mut doc, q, &cr, false)?;
    }
    let page = doc.pages.last_mut().unwrap();
    for (index, (title, value)) in totals.into_iter().enumerate() {
        let final_row = index == last;
        if final_row {
            rule(page, LEFT, y, 260.0);
            y += 5.0;
        }
        let size = if final_row { 11.0 } else { 9.0 };
        let mut title = label(title, LEFT + 133.0, y, 127.0, size);
        let mut value = label(display_amount(q, value), LEFT, y, 130.0, size);
        title.bold = final_row;
        value.bold = final_row;
        let h = height(&cr, &title).max(height(&cr, &value));
        add(page, title);
        add(page, value);
        y += h + 5.0;
    }
    y += 10.0;
    if q.stale_ack {
        paragraph(
            &mut doc,
            q,
            &cr,
            "هشدار قیمت: استفاده از قیمت قدیمی توسط کاربر تأیید شده است. زمان منبع هر کالا در جدول درج شده است.",
            &mut y,
            8.5,
        )?;
        y += 10.0;
    }
    if !q.notes.trim().is_empty() {
        if y + 60.0 > BOTTOM {
            y = next_page(&mut doc, q, &cr, false)?;
        }
        paragraph(&mut doc, q, &cr, "یادداشت", &mut y, 9.5)?;
        paragraph(&mut doc, q, &cr, &q.notes, &mut y, 9.0)?;
        y += 10.0;
    }
    if !q.terms.trim().is_empty() {
        if y + 60.0 > BOTTOM {
            y = next_page(&mut doc, q, &cr, false)?;
        }
        paragraph(&mut doc, q, &cr, "شرایط پیش‌فاکتور", &mut y, 9.5)?;
        paragraph(&mut doc, q, &cr, &q.terms, &mut y, 9.0)?;
        y += 10.0;
    }
    if q.tax_basis_points != 0 {
        paragraph(
            &mut doc,
            q,
            &cr,
            "نرخ مالیات صرفاً تنظیم کاربر است؛ این پیش‌فاکتور، تأیید انطباق مالیاتی یا فاکتور قانونی نیست.",
            &mut y,
            7.5,
        )?;
        y += 5.0;
    }
    let configured_footer = label(&q.footer, 260.0, 803.0, 299.0, 7.0);
    let short_footer = !q.footer.trim().is_empty() && height(&cr, &configured_footer) <= 25.0;
    if !q.footer.trim().is_empty() && !short_footer {
        paragraph(&mut doc, q, &cr, &q.footer, &mut y, 8.0)?;
        y += 8.0;
    }
    let approval = if q.approved_at.is_empty() {
        "این پیش‌نمایش هنوز تأیید و ذخیره نشده است.".into()
    } else {
        format!("تأیید نسخه: {}", ltr(&display_timestamp(q, &q.approved_at)))
    };
    paragraph(&mut doc, q, &cr, &approval, &mut y, 7.5)?;
    paragraph(
        &mut doc,
        q,
        &cr,
        "فقط پیش‌فاکتور؛ بدون ثبت سند حسابداری، تعهد موجودی یا وضعیت ارسال به مشتری.",
        &mut y,
        7.5,
    )?;
    let count = doc.pages.len();
    for (index, page) in doc.pages.iter_mut().enumerate() {
        rule(page, LEFT, 795.0, WIDTH);
        let footer_text = if short_footer {
            q.footer.clone()
        } else if !q.issuer_name.is_empty() {
            format!("{} · پیش‌فاکتور", q.issuer_name)
        } else {
            "سرو · پیش‌فاکتور · دادهٔ محلی".into()
        };
        let mut footer = label(footer_text, 260.0, 803.0, 299.0, 7.0);
        if height(&cr, &footer) > 25.0 {
            footer.value = "پیش‌فاکتور · دادهٔ محلی".into();
        }
        footer.color = MUTED;
        add(page, footer);
        let mut number = label(
            format!(
                "صفحهٔ {} از {}",
                document_digits(q, index + 1),
                document_digits(q, count)
            ),
            LEFT,
            803.0,
            180.0,
            7.0,
        );
        number.align = pango::Alignment::Left;
        number.color = MUTED;
        add(page, number);
    }
    Ok(doc)
}
fn render_page(cr: &cairo::Context, page: &Page) -> Result<()> {
    let _fonts = FontScope::new();
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.paint()?;
    for element in &page.elements {
        match element {
            Element::Text(text) => {
                cr.set_source_rgb(text.color.0, text.color.1, text.color.2);
                cr.move_to(text.x, text.y);
                pangocairo::functions::show_layout(cr, &layout(cr, text));
            }
            Element::Rule { x, y, width } => {
                cr.set_source_rgb(RULE.0, RULE.1, RULE.2);
                cr.set_line_width(0.5);
                cr.move_to(*x, *y);
                cr.line_to(x + width, *y);
                cr.stroke()?;
            }
            Element::Fill {
                x,
                y,
                width,
                height,
            } => {
                cr.set_source_rgb(0.955, 0.96, 0.965);
                cr.rectangle(*x, *y, *width, *height);
                cr.fill()?;
            }
        }
    }
    cr.status()?;
    Ok(())
}
pub fn pdf(q: &Quote, path: &Path) -> Result<()> {
    let doc = document(q)?;
    // Render into memory first. A failed Cairo/Pango operation never creates a
    // partial destination, and write_new refuses an existing file or symlink.
    let surface = cairo::PdfSurface::for_stream(PAGE_W, PAGE_H, Vec::<u8>::new())?;
    let cr = cairo::Context::new(&surface)?;
    for page in &doc.pages {
        render_page(&cr, page)?;
        cr.show_page()?;
    }
    surface.finish();
    surface.status()?;
    let bytes = surface
        .finish_output_stream()
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .downcast::<Vec<u8>>()
        .map_err(|_| anyhow::anyhow!("جریان PDF نامعتبر است"))?;
    write_new(path, &bytes)
}
/// Native GtkPicture-ready A4 pages, from the same layout and renderer as PDF.
/// The caller owns these temporary files and should remove them after closing.
/// This function never changes GTK windows or labels and never overwrites files.
pub fn png_pages(q: &Quote, directory: &Path) -> Result<Vec<PathBuf>> {
    let doc = document(q)?;
    ensure!(directory.is_dir(), "پوشهٔ پیش‌نمایش موجود نیست");
    let prefix = id();
    let mut created = Vec::new();
    let result = (|| -> Result<()> {
        for (index, page) in doc.pages.iter().enumerate() {
            let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1190, 1684)?;
            let cr = cairo::Context::new(&surface)?;
            cr.scale(2.0, 2.0);
            render_page(&cr, page)?;
            surface.flush();
            surface.status()?;
            let mut bytes = Vec::new();
            surface.write_to_png(&mut bytes)?;
            let path = directory.join(format!("sarv-preview-{prefix}-{:04}.png", index + 1));
            write_new(&path, &bytes)?;
            created.push(path);
        }
        Ok(())
    })();
    if let Err(error) = result {
        for path in &created {
            let _ = std::fs::remove_file(path);
        }
        return Err(error);
    }
    Ok(created)
}
