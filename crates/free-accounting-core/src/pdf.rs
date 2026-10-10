//! Real, searchable PDF exports with Pango text shaping (including Persian).
use super::*;
use cairo::{Context, PdfSurface};
use pangocairo::pango::{EllipsizeMode, FontDescription};

fn money_text(minor: &str, currency: &str, s: &Settings) -> Result<String> {
    let decimals = money::decimals(currency)? as usize;
    let number = minor.parse::<i64>()?;
    let raw = format!("{:0width$}", number.abs(), width = decimals + 1);
    let value = if decimals > 0 {
        format!(
            "{}.{}",
            &raw[..raw.len() - decimals],
            &raw[raw.len() - decimals..]
        )
    } else {
        raw
    };
    let text = format!("{}{value} {currency}", if number < 0 { "−" } else { "" });
    Ok(if s.digits == "persian" {
        text.chars()
            .map(|c| {
                if c.is_ascii_digit() {
                    char::from_u32('۰' as u32 + c as u32 - '0' as u32).unwrap()
                } else {
                    c
                }
            })
            .collect()
    } else {
        text
    })
}

fn draw(context: &Context, text: &str, x: f64, y: f64, width: i32, bold: bool) {
    let layout = pangocairo::functions::create_layout(context);
    layout.set_font_description(Some(&FontDescription::from_string(if bold {
        "Inter, Vazirmatn Bold 11"
    } else {
        "Inter, Vazirmatn 9"
    })));
    layout.set_width(width * pangocairo::pango::SCALE);
    layout.set_ellipsize(EllipsizeMode::End);
    layout.set_text(text);
    context.move_to(x, y);
    pangocairo::functions::show_layout(context, &layout);
}

pub(super) fn export_pdf(conn: &Connection, payload: &Value) -> Result<Value> {
    let s = settings(conn)?;
    let scope = text(payload, "scope")?;
    let mut filters: Filters =
        serde_json::from_value(payload.get("filters").cloned().unwrap_or(json!({})))?;
    filters.offset = 0;
    filters.limit = 500;
    if filters.currency.is_empty() {
        filters.currency = s.currency.clone();
    }
    let statement = scope == "statement_pdf";
    let title = if statement {
        ensure!(
            !filters.contact.is_empty(),
            "Select a contact for a statement"
        );
        let name: String = conn.query_row(
            "SELECT name FROM contacts WHERE id=?1",
            [&filters.contact],
            |r| r.get(0),
        )?;
        format!("Contact statement · {name}")
    } else {
        "Transaction ledger".into()
    };
    let surface = PdfSurface::for_stream(842.0, 595.0, Vec::<u8>::new())?;
    let context = Context::new(&surface)?;
    let mut page_number = 0;
    let mut y = 0.;
    let start_page = |page_number: &mut i32, y: &mut f64| -> Result<()> {
        *page_number += 1;
        context.set_source_rgb(0.08, 0.08, 0.08);
        draw(&context, &s.profile, 36., 24., 720, true);
        draw(&context, &title, 36., 46., 720, true);
        draw(
            &context,
            &format!("{} · {} · Page {}", today(), filters.currency, page_number),
            36.,
            69.,
            720,
            false,
        );
        if statement {
            draw(
                &context,
                "Transactions and outstanding balances are derived from the shared journal.",
                36.,
                88.,
                760,
                false,
            );
        }
        let headers = [
            ("Date", 36., 76),
            ("Payee", 120., 176),
            ("Category", 304., 135),
            ("Account", 447., 135),
            ("Amount", 590., 105),
            ("Balance", 705., 105),
        ];
        for (text, x, w) in headers {
            draw(&context, text, x, 112., w, true);
        }
        context.set_source_rgb(0.88, 0.88, 0.88);
        context.move_to(36., 135.);
        context.line_to(806., 135.);
        context.stroke()?;
        context.set_source_rgb(0.1, 0.1, 0.1);
        *y = 145.;
        Ok(())
    };
    start_page(&mut page_number, &mut y)?;
    loop {
        let page = views::ledger(conn, filters.clone())?;
        for row in page["rows"].as_array().unwrap() {
            if y > 540. {
                context.show_page()?;
                start_page(&mut page_number, &mut y)?;
            }
            let currency = row["currency"].as_str().unwrap();
            for (text, x, w) in [
                (row["date"].as_str().unwrap().to_string(), 36., 76),
                (row["payee"].as_str().unwrap().to_string(), 120., 176),
                (row["category"].as_str().unwrap().to_string(), 304., 135),
                (row["account"].as_str().unwrap().to_string(), 447., 135),
                (
                    money_text(row["amountMinor"].as_str().unwrap(), currency, &s)?,
                    590.,
                    105,
                ),
                (
                    money_text(row["balanceMinor"].as_str().unwrap(), currency, &s)?,
                    705.,
                    105,
                ),
            ] {
                draw(&context, &text, x, y, w, false);
            }
            y += 24.;
        }
        if let Some(next) = page["nextOffset"].as_u64() {
            filters.offset = next as u32;
        } else {
            break;
        }
    }
    if statement {
        let workspace = views::workspace(conn, &json!({}))?;
        if let Some(contact) = workspace["contacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == filters.contact)
        {
            if y > 470. {
                context.show_page()?;
                start_page(&mut page_number, &mut y)?;
            }
            y += 18.;
            for balance in contact["balances"].as_array().unwrap() {
                let currency = balance["currency"].as_str().unwrap();
                draw(
                    &context,
                    &format!(
                        "Receivable: {}    Payable: {}",
                        money_text(balance["receivableMinor"].as_str().unwrap(), currency, &s)?,
                        money_text(balance["payableMinor"].as_str().unwrap(), currency, &s)?
                    ),
                    36.,
                    y,
                    760,
                    true,
                );
                y += 22.;
            }
        }
    }
    context.show_page()?;
    drop(context);
    let bytes = surface
        .finish_output_stream()
        .map_err(|e| anyhow::anyhow!("PDF export: {e:?}"))?
        .downcast::<Vec<u8>>()
        .map_err(|_| anyhow::anyhow!("Invalid PDF stream"))?;
    Ok(
        json!({"name":format!("{}-{}.pdf",if statement{"statement"}else{"ledger"},today()),"mime":"application/pdf","content":"","bytes":bytes}),
    )
}
