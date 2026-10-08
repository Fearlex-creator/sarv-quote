//! Reconciled opening snapshots; independent controls, definitions and provenance.
use crate::{
    accounting::{AccountingState, Money, MoneyUnit},
    core::*,
    domain::{DateDisplay, DisplayDigits, ltr, text},
    export::safe_cell,
    store::write_new,
};
use anyhow::{Result, ensure};
use gtk::pango::{self, prelude::FontMapExt};
use std::path::Path;

#[derive(Clone)]
struct Detail {
    kind: String,
    id: String,
    label: String,
    amount: Option<Money>,
    quantity: Option<i64>,
    base_quantity: Option<i64>,
    reference: String,
    snapshot: String,
}
#[allow(clippy::too_many_arguments)]
fn detail<T: serde::Serialize>(
    kind: &str,
    id: &str,
    label: String,
    amount: Option<Money>,
    quantity: Option<i64>,
    base_quantity: Option<i64>,
    reference: &str,
    record: &T,
) -> Result<Detail> {
    Ok(Detail {
        kind: kind.into(),
        id: id.into(),
        label,
        amount,
        quantity,
        base_quantity,
        reference: reference.into(),
        snapshot: serde_json::to_string(record)?,
    })
}
fn account_label(state: &AccountingState, company_id: &str, account_id: &str) -> Result<String> {
    let account = state
        .accounts
        .iter()
        .find(|a| a.id == account_id && a.company_id == company_id)
        .ok_or_else(|| anyhow::anyhow!("حساب تاریخی گزارش پیدا نشد؛ نام حساب قابل حدس نیست"))?;
    Ok(format!("{} • کد {}", account.name, ltr(&account.code)))
}
fn snapshot<'a>(
    state: &'a AccountingState,
    company_id: &str,
) -> Result<(&'a OpeningRecord, OpeningPreview, Vec<Detail>)> {
    let opening = state
        .core
        .openings
        .iter()
        .find(|o| o.command.company_id == company_id)
        .ok_or_else(|| {
            anyhow::anyhow!("افتتاحیهٔ تطبیق‌شده هنوز ثبت نشده است؛ گزارش تأییدشده قابل صدور نیست")
        })?;
    let c = &opening.command;
    let p = state.reconcile_opening(company_id, &c.as_of)?;
    ensure!(
        p.balanced && p.reconciled && p.controls.iter().all(|r| r.difference == Money::ZERO),
        "افتتاحیه تطبیق نشده است"
    );
    let mut rows = vec![];
    rows.push(detail(
        "policy",
        &opening.frozen_setup.id,
        format!(
            "سیاست و نگاشت حساب‌ها • {}",
            opening.frozen_setup.command.policy.summary()
        ),
        None,
        None,
        None,
        &opening.frozen_setup.command.policy.version,
        &opening.frozen_setup,
    )?);
    for x in &c.parties {
        rows.push(detail(
            "party",
            &x.id,
            format!(
                "{} • مشتری {} • تأمین‌کننده {}",
                x.name,
                if x.customer { "بله" } else { "خیر" },
                if x.supplier { "بله" } else { "خیر" }
            ),
            None,
            None,
            None,
            "",
            x,
        )?);
    }
    for x in &c.units {
        rows.push(detail(
            "unit",
            &x.id,
            x.name.clone(),
            None,
            None,
            None,
            "",
            x,
        )?);
    }
    for x in &c.items {
        rows.push(detail(
            "item",
            &x.id,
            format!(
                "{} • SKU {} • پایه {} • غیرقابل تقسیم {} • بسته‌ها {}",
                x.name,
                ltr(&x.sku),
                ltr(&x.base_unit_id),
                if x.indivisible { "بله" } else { "خیر" },
                x.packs
                    .iter()
                    .map(|p| ltr(&format!(
                        "1 {} = {} / {} {}",
                        c.units
                            .iter()
                            .find(|u| u.id == p.unit_id)
                            .map_or(p.unit_id.as_str(), |u| u.name.as_str()),
                        p.numerator,
                        p.denominator,
                        c.units
                            .iter()
                            .find(|u| u.id == x.base_unit_id)
                            .map_or(x.base_unit_id.as_str(), |u| u.name.as_str())
                    )))
                    .collect::<Vec<_>>()
                    .join("؛ ")
            ),
            None,
            None,
            None,
            &x.sku,
            x,
        )?);
    }
    for x in &c.warehouses {
        rows.push(detail(
            "warehouse",
            &x.id,
            x.name.clone(),
            None,
            None,
            None,
            "",
            x,
        )?);
    }
    for x in &c.balances {
        rows.push(detail(
            "ledger",
            &x.account_id,
            format!(
                "{} • بدهکار {} IRR • بستانکار {} IRR",
                account_label(state, &c.company_id, &x.account_id)?,
                ltr(&x.debit.rials().to_string()),
                ltr(&x.credit.rials().to_string())
            ),
            None,
            None,
            None,
            &x.source_reference,
            x,
        )?);
    }
    for x in &c.cash {
        rows.push(detail(
            "cash",
            &x.id,
            x.name.clone(),
            Some(x.balance),
            None,
            None,
            &x.source_reference,
            x,
        )?);
    }
    for x in &c.banks {
        rows.push(detail(
            "bank",
            &x.id,
            format!(
                "{} • بانک {} • ماندهٔ صورت‌حساب {} IRR",
                x.name,
                ltr(&x.bank_reference),
                ltr(&x.statement_balance.rials().to_string())
            ),
            Some(x.balance),
            None,
            None,
            &x.source_reference,
            x,
        )?);
    }
    for x in &c.open_items {
        rows.push(detail(
            if x.side == OpenItemSide::Receivable {
                "ar"
            } else {
                "ap"
            },
            &x.id,
            format!(
                "طرف {} • تاریخ میلادی {} • سررسید میلادی {}",
                ltr(&x.party_id),
                ltr(&x.document_date),
                ltr(&x.due_date)
            ),
            Some(x.remaining),
            None,
            None,
            &x.document_reference,
            x,
        )?);
    }
    for x in &c.advances {
        rows.push(detail(
            if x.side == AdvanceSide::Supplier {
                "supplier_advance"
            } else {
                "customer_advance"
            },
            &x.id,
            format!(
                "طرف {} • تاریخ میلادی {}",
                ltr(&x.party_id),
                ltr(&x.document_date)
            ),
            Some(x.remaining),
            None,
            None,
            &x.source_reference,
            x,
        )?);
    }
    for x in &c.cheques {
        rows.push(detail(
            if x.direction == ChequeDirection::Receivable {
                "cheque_receivable"
            } else {
                "cheque_payable"
            },
            &x.id,
            format!(
                "طرف {} • وضعیت {} • بانک {} • دارنده {} • صدور میلادی {} • سررسید میلادی {} • تسویهٔ تاریخی {}",
                ltr(&x.party_id),
                if x.state==OpeningChequeState::Held {"در دست / وصول‌نشده"} else {"صادرشده / وصول‌نشده"},
                ltr(&x.bank_reference),
                x.holder,
                ltr(&x.issue_date),
                ltr(&x.due_date),
                ltr(&x.settlement_reference)
            ),
            Some(x.amount),
            None,
            None,
            &x.cheque_reference,
            x,
        )?);
    }
    for x in &c.stock {
        rows.push(detail(
            "stock",
            &x.id,
            format!(
                "کالا {} • انبار {} • واحد {}",
                ltr(&x.item_id),
                ltr(&x.warehouse_id),
                ltr(&x.unit_id)
            ),
            Some(x.value),
            Some(x.quantity_thousandths),
            Some(c.base_quantity(&x.item_id, &x.unit_id, x.quantity_thousandths)?),
            &x.source_reference,
            x,
        )?);
    }
    for x in &c.grni {
        rows.push(detail("grni",&x.id,format!("طرف {} • کالا {} • انبار {} • واحد {} • دریافت میلادی {} • زیرمجموعهٔ موجودی، بدون افزایش دوباره",ltr(&x.party_id),ltr(&x.item_id),ltr(&x.warehouse_id),ltr(&x.unit_id),ltr(&x.received_date)),Some(x.value),Some(x.quantity_thousandths),Some(c.base_quantity(&x.item_id,&x.unit_id,x.quantity_thousandths)?),&x.receipt_reference,x)?);
    }
    Ok((opening, p, rows))
}
fn effect_description(effect: OpeningEffect) -> &'static str {
    match effect {
        OpeningEffect::Journal => "سند مالی افتتاحیه",
        OpeningEffect::ZeroBalanceInitialization => "آغاز با ماندهٔ صفر؛ بدون سند مالی",
    }
}
fn unit_key(unit: MoneyUnit) -> &'static str {
    if unit == MoneyUnit::Rial {
        "rial"
    } else {
        "toman"
    }
}
fn shown_date(date: &str, dates: DateDisplay) -> Result<String> {
    if dates == DateDisplay::Jalali {
        Ok(crate::calendar::format_date(
            crate::calendar::parse_date(date, crate::calendar::Calendar::Gregorian)?,
            crate::calendar::Calendar::Jalali,
        )?)
    } else {
        Ok(dates.format(date))
    }
}
fn quantity(q: i64) -> String {
    format!("{}.{:03}", q / 1000, q % 1000)
}

pub fn opening_csv(
    state: &AccountingState,
    company_id: &str,
    unit: MoneyUnit,
    dates: DateDisplay,
    path: &Path,
) -> Result<()> {
    opening_csv_with_preferences(state, company_id, unit, dates, DisplayDigits::Latin, path)
}
pub fn opening_csv_with_preferences(
    state: &AccountingState,
    company_id: &str,
    unit: MoneyUnit,
    dates: DateDisplay,
    digits: DisplayDigits,
    path: &Path,
) -> Result<()> {
    let (o, p, rows) = snapshot(state, company_id)?;
    let c = &o.command;
    let company = state
        .companies
        .iter()
        .find(|x| x.id == company_id)
        .ok_or_else(|| anyhow::anyhow!("شرکت پیدا نشد"))?;
    let mut w = csv::Writer::from_writer(Vec::new());
    w.write_record([
        "format_version",
        "company_id",
        "company",
        "as_of_gregorian",
        "display_as_of",
        "display_calendar",
        "included_states",
        "basis",
        "policy_version",
        "base_currency",
        "display_unit",
        "opening_effect",
        "row_kind",
        "id",
        "role",
        "account_id",
        "label",
        "amount_irr",
        "display_amount",
        "quantity_thousandths",
        "base_quantity_thousandths",
        "ledger_irr",
        "detail_irr",
        "difference_irr",
        "display_ledger",
        "display_detail",
        "display_difference",
        "source_reference",
        "snapshot_json",
    ])?;
    let common = vec![
        "sarv-opening-report-v1".into(),
        safe_cell(company_id),
        safe_cell(&company.name),
        c.as_of.clone(),
        digits.format(shown_date(&c.as_of, dates)?),
        if dates == DateDisplay::Jalali {
            "jalali".into()
        } else {
            "gregorian".into()
        },
        match o.effect {
            OpeningEffect::Journal => "posted_reconciled_opening".into(),
            OpeningEffect::ZeroBalanceInitialization => {
                "initialized_reconciled_zero_opening".into()
            }
        },
        "initial_opening_beginning_of_day".into(),
        o.frozen_setup.command.policy.version.clone(),
        "IRR".into(),
        unit_key(unit).into(),
        match o.effect {
            OpeningEffect::Journal => "journal".into(),
            OpeningEffect::ZeroBalanceInitialization => "zero_balance_initialization".into(),
        },
    ];
    let mut emit = |tail: Vec<String>| -> Result<()> {
        let mut cells = common.clone();
        cells.extend(tail);
        w.write_record(cells)?;
        Ok(())
    };
    for x in &p.controls {
        emit(vec![
            "control".into(),
            safe_cell(x.role.key()),
            x.role.key().into(),
            safe_cell(&x.account_id),
            safe_cell(
                &account_label(state, company_id, &x.account_id)?
                    .replace(['\u{2066}', '\u{2069}'], ""),
            ),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            x.ledger.rials().to_string(),
            x.detail.rials().to_string(),
            x.difference.rials().to_string(),
            digits.format(x.ledger.display(unit)),
            digits.format(x.detail.display(unit)),
            digits.format(x.difference.display(unit)),
            safe_cell(&c.source_id),
            serde_json::to_string(x)?,
        ])?;
    }
    for x in rows {
        emit(vec![
            x.kind,
            safe_cell(&x.id),
            String::new(),
            String::new(),
            safe_cell(&x.label.replace(['\u{2066}', '\u{2069}'], "")),
            x.amount.map(|m| m.rials().to_string()).unwrap_or_default(),
            x.amount
                .map(|m| digits.format(m.display(unit)))
                .unwrap_or_default(),
            x.quantity.map(|q| q.to_string()).unwrap_or_default(),
            x.base_quantity.map(|q| q.to_string()).unwrap_or_default(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            safe_cell(&x.reference),
            safe_cell(&x.snapshot),
        ])?;
    }
    for (kind, amount) in [
        ("total_debit", p.total_debit),
        ("total_credit", p.total_credit),
    ] {
        emit(vec![
            kind.into(),
            kind.into(),
            String::new(),
            String::new(),
            String::new(),
            amount.rials().to_string(),
            digits.format(amount.display(unit)),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            safe_cell(&c.source_id),
            String::new(),
        ])?;
    }
    emit(vec![
        "opening_snapshot".into(),
        safe_cell(&o.journal_id),
        String::new(),
        String::new(),
        effect_description(o.effect).into(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        safe_cell(&c.source_id),
        safe_cell(&serde_json::to_string(o)?),
    ])?;
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend(w.into_inner()?);
    write_new(path, &bytes)
}
fn layout(
    cr: &cairo::Context,
    map: &pango::FontMap,
    value: &str,
    width: f64,
    size: u32,
) -> pango::Layout {
    let context = map.create_context();
    pangocairo::functions::update_context(cr, &context);
    context.set_base_dir(pango::Direction::Rtl);
    pangocairo::functions::context_set_resolution(&context, 72.0);
    let l = pango::Layout::new(&context);
    l.set_auto_dir(false);
    l.set_font_description(Some(&pango::FontDescription::from_string(&format!(
        "Vazirmatn, Noto Sans Arabic {size}"
    ))));
    l.set_width((width * f64::from(pango::SCALE)) as i32);
    l.set_wrap(pango::WrapMode::WordChar);
    l.set_alignment(pango::Alignment::Right);
    l.set_text(value);
    l
}
fn draw(cr: &cairo::Context, map: &pango::FontMap, value: &str, y: f64, size: u32) -> Result<f64> {
    let l = layout(cr, map, value, 523.0, size);
    cr.move_to(36.0, y);
    pangocairo::functions::show_layout(cr, &l);
    cr.status()?;
    Ok(f64::from(l.pixel_size().1))
}
pub fn opening_pdf(
    state: &AccountingState,
    company_id: &str,
    unit: MoneyUnit,
    dates: DateDisplay,
    path: &Path,
) -> Result<()> {
    opening_pdf_with_preferences(state, company_id, unit, dates, DisplayDigits::Persian, path)
}
pub fn opening_pdf_with_preferences(
    state: &AccountingState,
    company_id: &str,
    unit: MoneyUnit,
    dates: DateDisplay,
    digits: DisplayDigits,
    path: &Path,
) -> Result<()> {
    let (o, p, rows) = snapshot(state, company_id)?;
    let c = &o.command;
    let company = state
        .companies
        .iter()
        .find(|x| x.id == company_id)
        .ok_or_else(|| anyhow::anyhow!("شرکت پیدا نشد"))?;
    text(&company.name)?;
    let surface = cairo::PdfSurface::for_stream(595.0, 842.0, Vec::<u8>::new())?;
    let cr = cairo::Context::new(&surface)?;
    let map = pangocairo::FontMap::new();
    let cutoff = shown_date(&c.as_of, dates)?;
    let jalali = shown_date(&c.as_of, DateDisplay::Jalali)
        .unwrap_or_else(|_| "خارج از دامنهٔ نمایش شمسی؛ تاریخ میلادی معتبر است".into());
    let heading = |page: usize| -> Result<f64> {
        cr.set_source_rgb(0.08, 0.12, 0.16);
        let mut y = 32.0;
        y += draw(
            &cr,
            &map,
            &format!("تطبیق افتتاحیه • {}", company.name),
            y,
            15,
        )? + 6.0;
        y += draw(
            &cr,
            &map,
            &format!(
                "شرکت {} • افتتاح ابتدای روز • ثبت آغاز و تطبیق تأییدشده\nتاریخ نمایش {} • شمسی {} • میلادی {}\nپایه IRR • واحد {} • سیاست {} • صفحه {}",
                ltr(company_id),
                ltr(&digits.format(&cutoff)),
                ltr(&digits.format(&jalali)),
                ltr(&digits.format(&c.as_of)),
                if unit == MoneyUnit::Rial {
                    "ریال (rial)"
                } else {
                    "تومان (toman)"
                },
                ltr(&o.frozen_setup.command.policy.version),
                digits.format(page)
            ),
            y,
            9,
        )? + 10.0;
        y += draw(
            &cr,
            &map,
            "کنترل / ردیف / منبع • دفتر کل • جزئیات • اختلاف • مقدار و واحد",
            y,
            10,
        )? + 12.0;
        ensure!(
            y < 620.0,
            "سربرگ بزرگ‌تر از صفحه است؛ نام کوتاه‌تر یا CSV را انتخاب کنید"
        );
        Ok(y)
    };
    let mut blocks = vec![format!(
        "منبع {} • {} • مرجع آغاز {} • مبنا: تعهدی / موجودی دائمی • دلیل {}",
        ltr(&c.source_id),
        effect_description(o.effect),
        ltr(&o.journal_id),
        c.reason
    )];
    if let Some(source) = &c.import_source {
        blocks.push(format!(
            "منبع CSV SHA256 {} • {} ردیف اصلی؛ مقادیر اصلی در خروجی CSV محفوظ است",
            ltr(&source.sha256),
            digits.format(source.rows.len())
        ));
    }
    for x in &p.controls {
        blocks.push(format!(
            "کنترل • {}\nدفتر {} • جزئیات {} • اختلاف {} {}",
            digits.format(account_label(state, company_id, &x.account_id)?),
            ltr(&digits.format(x.ledger.display(unit))),
            ltr(&digits.format(x.detail.display(unit))),
            ltr(&digits.format(x.difference.display(unit))),
            unit_key(unit)
        ));
    }
    blocks.push(format!(
        "جمع بدهکار {} • جمع بستانکار {} {} • دقیق IRR: {} / {}",
        ltr(&digits.format(p.total_debit.display(unit))),
        ltr(&digits.format(p.total_credit.display(unit))),
        unit_key(unit),
        ltr(&digits.format(p.total_debit.rials())),
        ltr(&digits.format(p.total_credit.rials()))
    ));
    for x in rows {
        let mut value = if x.kind == "ledger" {
            format!("دفتر کل • {}", digits.format(&x.label))
        } else {
            format!(
                "{} • {} • {}",
                ltr(&x.kind),
                ltr(&x.id),
                digits.format(&x.label)
            )
        };
        if let Some(m) = x.amount {
            value.push_str(&format!(
                "\nمبلغ {} {} • دقیق IRR {}",
                ltr(&digits.format(m.display(unit))),
                unit_key(unit),
                ltr(&digits.format(m.rials()))
            ));
        }
        if let Some(q) = x.quantity {
            value.push_str(&format!(
                " • مقدار انتخابی {} • مقدار پایه {} (هزارم {})",
                ltr(&digits.format(quantity(q))),
                ltr(&digits.format(quantity(x.base_quantity.unwrap()))),
                ltr(&digits.format(x.base_quantity.unwrap()))
            ));
        }
        if !x.reference.is_empty() {
            value.push_str(&format!("\nمرجع {}", ltr(&x.reference)));
        }
        blocks.push(value);
    }
    let measure_surface = cairo::RecordingSurface::create(cairo::Content::ColorAlpha, None)?;
    let measure = cairo::Context::new(&measure_surface)?;
    let mut page = 1;
    let mut y = heading(page)?;
    for value in blocks {
        let l = layout(&measure, &map, &value, 523.0, 9);
        let h = f64::from(l.pixel_size().1) + 12.0;
        if y + h > 778.0 {
            cr.show_page()?;
            page += 1;
            y = heading(page)?;
        }
        ensure!(
            y + h <= 778.0,
            "ردیف در یک صفحه جا نمی‌شود؛ CSV را انتخاب کنید"
        );
        cr.set_source_rgb(0.08, 0.12, 0.16);
        draw(&cr, &map, &value, y, 9)?;
        y += h;
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
