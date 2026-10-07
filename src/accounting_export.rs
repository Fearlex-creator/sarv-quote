//! Snapshot exports for the bounded manual ledger. All amounts are exact IRR.
use crate::{
    accounting::{MoneyUnit, TrialBalance},
    domain::{ltr, text},
    export::safe_cell,
    store::write_new,
};
use anyhow::{Result, ensure};
use gtk::pango::{self, prelude::FontMapExt};
use std::path::Path;

fn validate(company: &str, balance: &TrialBalance) -> Result<()> {
    text(company)?;
    let mut debit = crate::accounting::Money::ZERO;
    let mut credit = debit;
    for row in &balance.rows {
        ensure!(
            row.debit.rials() >= 0
                && row.credit.rials() >= 0
                && (row.debit.rials() == 0 || row.credit.rials() == 0),
            "ماندهٔ گزارش نامعتبر است"
        );
        debit = debit.checked_add(row.debit)?;
        credit = credit.checked_add(row.credit)?;
    }
    ensure!(
        debit == balance.total_debit && credit == balance.total_credit && debit == credit,
        "جمع تراز آزمایشی معتبر نیست"
    );
    Ok(())
}
fn unit_name(unit: MoneyUnit) -> &'static str {
    match unit {
        MoneyUnit::Rial => "ریال",
        MoneyUnit::Toman => "تومان",
    }
}

pub fn trial_csv(
    company: &str,
    balance: &TrialBalance,
    unit: MoneyUnit,
    path: &Path,
) -> Result<()> {
    validate(company, balance)?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "company_id",
        "company",
        "as_of_gregorian",
        "included_states",
        "basis",
        "base_currency",
        "display_unit",
        "account_id",
        "account_code",
        "account_name",
        "debit_irr",
        "credit_irr",
        "display_debit",
        "display_credit",
    ])?;
    for row in &balance.rows {
        writer.write_record([
            safe_cell(&balance.company_id),
            safe_cell(company),
            safe_cell(&balance.as_of),
            "posted_including_reversals".into(),
            "cumulative_from_initial_opening".into(),
            "IRR".into(),
            if unit == MoneyUnit::Rial {
                "rial".into()
            } else {
                "toman".into()
            },
            safe_cell(&row.account_id),
            safe_cell(&row.code),
            safe_cell(&row.name),
            row.debit.rials().to_string(),
            row.credit.rials().to_string(),
            row.debit.display(unit),
            row.credit.display(unit),
        ])?;
    }
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend(writer.into_inner()?);
    write_new(path, &bytes)
}

fn draw(
    cr: &cairo::Context,
    map: &pango::FontMap,
    value: &str,
    area: (f64, f64, f64),
    size: u32,
    align: pango::Alignment,
) -> Result<f64> {
    let (x, y, width) = area;
    let context = map.create_context();
    pangocairo::functions::update_context(cr, &context);
    context.set_base_dir(pango::Direction::Rtl);
    let layout = pango::Layout::new(&context);
    pangocairo::functions::context_set_resolution(&context, 72.0);
    layout.set_auto_dir(false);
    layout.set_font_description(Some(&pango::FontDescription::from_string(&format!(
        "Vazirmatn, Noto Sans Arabic {size}"
    ))));
    layout.set_width((width * f64::from(pango::SCALE)) as i32);
    layout.set_wrap(pango::WrapMode::WordChar);
    layout.set_alignment(align);
    layout.set_text(value);
    cr.move_to(x, y);
    pangocairo::functions::show_layout(cr, &layout);
    cr.status()?;
    Ok(f64::from(layout.pixel_size().1))
}

/// A4 paged report with repeated company/cutoff/unit and column headings.
/// Renders to memory before atomic, no-overwrite publication.
pub fn trial_pdf(
    company: &str,
    balance: &TrialBalance,
    unit: MoneyUnit,
    path: &Path,
) -> Result<()> {
    validate(company, balance)?;
    let surface = cairo::PdfSurface::for_stream(595.0, 842.0, Vec::<u8>::new())?;
    let cr = cairo::Context::new(&surface)?;
    let map = pangocairo::FontMap::new();
    let heading = |page: usize| -> Result<f64> {
        cr.set_source_rgb(0.08, 0.12, 0.16);
        let h = draw(
            &cr,
            &map,
            &format!("تراز آزمایشی • {company}"),
            (36.0, 32.0, 523.0),
            16,
            pango::Alignment::Right,
        )?;
        let y = 40.0 + h;
        let h = draw(
            &cr,
            &map,
            &format!(
                "تا تاریخ میلادی {} • واحد: {} • پایه IRR\nهمهٔ اسناد ثبت‌شده و برگشت‌ها از ابتدای دفتر • صفحه {page}",
                ltr(&balance.as_of),
                unit_name(unit)
            ),
            (36.0, y, 523.0),
            10,
            pango::Alignment::Right,
        )?;
        let y = y + h + 16.0;
        for (v, x, width) in [
            ("بستانکار", 36.0, 104.0),
            ("بدهکار", 148.0, 104.0),
            ("نام حساب", 260.0, 200.0),
            ("کد", 468.0, 91.0),
        ] {
            draw(&cr, &map, v, (x, y, width), 11, pango::Alignment::Right)?;
        }
        ensure!(
            y + 30.0 < 650.0,
            "سربرگ در صفحه جا نمی‌شود؛ گزارش CSV را انتخاب کنید"
        );
        Ok(y + 30.0)
    };
    let mut page = 1;
    let mut y = heading(page)?;
    for row in &balance.rows {
        // Measure on a recording surface so pagination never draws a row
        // beyond the page before discovering that it needs to wrap.
        let measure_surface = cairo::RecordingSurface::create(cairo::Content::ColorAlpha, None)?;
        let measure = cairo::Context::new(&measure_surface)?;
        let cells = [
            (ltr(&row.credit.display(unit)), 36.0, 104.0),
            (ltr(&row.debit.display(unit)), 148.0, 104.0),
            (row.name.clone(), 260.0, 200.0),
            (ltr(&row.code), 468.0, 91.0),
        ];
        let mut height: f64 = 22.0;
        for (value, _, width) in &cells {
            height = height.max(draw(
                &measure,
                &map,
                value,
                (0.0, 0.0, *width),
                10,
                pango::Alignment::Right,
            )?);
        }
        height += 10.0;
        if y + height > 778.0 {
            cr.show_page()?;
            page += 1;
            y = heading(page)?;
        }
        ensure!(
            y + height <= 778.0,
            "ردیف حساب در یک صفحه جا نمی‌شود؛ گزارش CSV را انتخاب کنید"
        );
        cr.set_source_rgb(0.08, 0.12, 0.16);
        for (v, x, width) in [
            (ltr(&row.credit.display(unit)), 36.0, 104.0),
            (ltr(&row.debit.display(unit)), 148.0, 104.0),
            (row.name.clone(), 260.0, 200.0),
            (ltr(&row.code), 468.0, 91.0),
        ] {
            draw(&cr, &map, &v, (x, y, width), 10, pango::Alignment::Right)?;
        }
        y += height;
        cr.set_source_rgb(0.8, 0.84, 0.87);
        cr.set_line_width(0.5);
        cr.move_to(36.0, y - 5.0);
        cr.line_to(559.0, y - 5.0);
        cr.stroke()?;
    }
    let total = format!(
        "جمع بدهکار: {} • جمع بستانکار: {} {}",
        ltr(&balance.total_debit.display(unit)),
        ltr(&balance.total_credit.display(unit)),
        unit_name(unit)
    );
    let measure_surface = cairo::RecordingSurface::create(cairo::Content::ColorAlpha, None)?;
    let measure = cairo::Context::new(&measure_surface)?;
    let height = draw(
        &measure,
        &map,
        &total,
        (0.0, 0.0, 523.0),
        12,
        pango::Alignment::Right,
    )? + 16.0;
    if y + height > 778.0 {
        cr.show_page()?;
        page += 1;
        y = heading(page)?;
    }
    ensure!(
        y + height <= 778.0,
        "جمع گزارش در صفحه جا نمی‌شود؛ گزارش CSV را انتخاب کنید"
    );
    cr.set_source_rgb(0.08, 0.12, 0.16);
    draw(
        &cr,
        &map,
        &total,
        (36.0, y + 8.0, 523.0),
        12,
        pango::Alignment::Right,
    )?;
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
