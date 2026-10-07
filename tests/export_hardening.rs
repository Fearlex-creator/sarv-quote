use sarv_quote::{domain::*, export};
use std::{path::Path, process::Command};

fn quote(count: usize) -> Quote {
    let lines: Vec<QuoteLine> = (0..count)
        .map(|index| QuoteLine {
            query: format!("P-{index:03}"),
            code: format!("P-{index:03}/A4"),
            name: if index % 4 == 0 {
                "کاغذ A4 سفید مخصوص دستگاه چاپ و تکثیر، بستهٔ پانصد برگی با توضیح کامل محصول و اندازهٔ دقیق برای بررسی خوانایی عنوان بلند".into()
            } else { "خودکار آبی نمونه برای آزمایش".into() },
            measure: "بسته".into(),
            quantity_milli: 1250,
            price_rial: 125_003,
            total_rial: line_total(125_003, 1250).unwrap(),
            source_at: "2026-09-01T08:00:00Z".into(),
            source_unit: Unit::Rial,
        }).collect();
    let total_rial = lines.iter().map(|line| line.total_rial).sum::<i64>() - 13;
    Quote {
        id: "9d22d792-3f03-49da-81a3-96d55125c711".into(),
        series: "36bcab63-536d-4563-874b-a090b9df8bcf".into(),
        version: 2,
        customer: "فروشگاه نمونهٔ پارس — دادهٔ ساختگی برای آزمون".into(),
        customer_id: String::new(),
        customer_contact: String::new(),
        customer_email: String::new(),
        customer_address: String::new(),
        phone: "09120000000".into(),
        approved_at: "2026-10-07T10:00:00Z".into(),
        catalog_id: "76cf9c87-528d-42c8-9174-c6d4801874bc".into(),
        catalog_name: "فهرست قیمت نمونه / مهر ۱۴۰۵".into(),
        lines,
        discount_rial: 13,
        total_rial,
        display_unit: Unit::Toman,
        stale_ack: true,
        number: String::new(), issuer_name: String::new(), issuer_phone: String::new(), issuer_address: String::new(), header: String::new(), footer: String::new(), terms: String::new(), tax_basis_points: 0, tax_rial: 0, display_digits: DisplayDigits::Persian, date_display: DateDisplay::Iso,
        reference: "PO-2026/۱۲۳".into(),
        quote_date: "2026-10-07".into(),
        valid_until: "2026-10-14".into(),
        notes: "یادداشت نمونه: قیمت‌ها فقط براساس فهرست تأییدشده است.\nاین سند پیشنهاد قیمت است و سند حسابداری محسوب نمی‌شود.".into(),
    }
}

#[test]
fn csv_neutralizes_all_untrusted_text_and_keeps_exact_rial_numbers() {
    let mut q = quote(1);
    q.id = "=ID()".into();
    q.customer = " \t=SUM(1,2)".into();
    q.phone = "+989120000000".into();
    q.catalog_name = "\u{feff}@catalog".into();
    q.reference = "\r-1+2".into();
    q.notes = "\n\t=HYPERLINK(\"x\")".into();
    q.quote_date = "\u{0000}+date".into();
    q.valid_until = "@expiry".into();
    q.approved_at = "=timestamp".into();
    q.lines[0].query = "@query".into();
    q.lines[0].code = "+SKU".into();
    q.lines[0].name = "-name".into();
    q.lines[0].measure = "=unit".into();
    q.lines[0].source_at = "=source".into();
    let bytes = export::csv_bytes(&q).unwrap();
    assert!(bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    let mut reader = csv::Reader::from_reader(&bytes[3..]);
    let headers = reader.headers().unwrap().clone();
    let record = reader.records().next().unwrap().unwrap();
    for name in [
        "quote_id",
        "customer",
        "phone",
        "catalog",
        "requested_item",
        "code",
        "name",
        "measure",
        "source_at",
        "approved_at",
        "reference",
        "quote_date",
        "valid_until",
        "notes",
    ] {
        let index = headers.iter().position(|header| header == name).unwrap();
        assert!(record[index].starts_with('\''), "unsafe column {name}");
    }
    for (name, expected) in [
        ("unit_price_rial", "125003"),
        ("line_total_rial", "156254"),
        ("discount_rial", "13"),
        ("quote_total_rial", "156241"),
    ] {
        let index = headers.iter().position(|header| header == name).unwrap();
        assert_eq!(&record[index], expected);
    }
    assert_eq!(
        export::safe_cell("ordinary,\"quoted\"\ntext"),
        "ordinary,\"quoted\"\ntext"
    );
}

#[test]
fn exports_never_overwrite_and_failed_render_leaves_no_file() {
    let q = quote(3);
    let dir = tempfile::tempdir().unwrap();
    for extension in ["pdf", "csv"] {
        let target = dir.path().join(format!("existing.{extension}"));
        std::fs::write(&target, b"keep original").unwrap();
        let result = if extension == "pdf" {
            export::pdf(&q, &target)
        } else {
            export::csv(&q, &target)
        };
        assert!(result.is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"keep original");
    }
    let mut broken = q.clone();
    broken.total_rial += 1;
    let target = dir.path().join("bad.pdf");
    assert!(export::pdf(&broken, &target).is_err());
    assert!(!target.exists());
    assert!(export::csv_bytes(&broken).is_err());
    let empty = dir.path().join("pngs");
    std::fs::create_dir(&empty).unwrap();
    assert!(export::png_pages(&broken, &empty).is_err());
    assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0);
    assert!(export::pdf(&q, &dir.path().join("absent/quote.pdf")).is_err());
}

#[cfg(unix)]
#[test]
fn export_refuses_existing_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original");
    let link = dir.path().join("quote.pdf");
    std::fs::write(&original, b"keep original").unwrap();
    std::os::unix::fs::symlink(&original, &link).unwrap();
    assert!(export::pdf(&quote(1), &link).is_err());
    assert_eq!(std::fs::read(&original).unwrap(), b"keep original");
}

fn pdf_page_count(path: &Path) -> Option<usize> {
    let output = Command::new("pdfinfo").arg(path).output().ok()?;
    assert!(
        output.status.success(),
        "invalid PDF: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| {
            line.strip_prefix("Pages:")
                .and_then(|v| v.trim().parse().ok())
        })
}

#[test]
fn multipage_pdf_matches_native_preview_and_long_notes_continue() {
    let mut q = quote(44);
    q.notes = (0..70).map(|index| format!("یادداشت آزمایشی {}: متن فارسی همراه SKU-X/123 و توضیح شرایط پیشنهاد برای سنجش شکست خط و ادامهٔ متن در صفحهٔ بعد.\n", fa(index + 1))).collect();
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("many.pdf");
    export::pdf(&q, &pdf).unwrap();
    let pngs = export::png_pages(&q, dir.path()).unwrap();
    assert!(pngs.len() >= 5);
    if let Some(count) = pdf_page_count(&pdf) {
        assert_eq!(count, pngs.len());
    }
    for png in &pngs {
        assert!(
            std::fs::read(png)
                .unwrap()
                .starts_with(b"\x89PNG\r\n\x1a\n")
        );
    }
    let next = export::png_pages(&q, dir.path()).unwrap();
    assert_eq!(next.len(), pngs.len());
    assert_ne!(next, pngs, "repeated preview must use fresh filenames");
    for (first, repeated) in pngs.iter().zip(&next) {
        assert_eq!(
            std::fs::read(first).unwrap(),
            std::fs::read(repeated).unwrap(),
            "repeated rendering must preserve glyph positions and monetary columns"
        );
    }
    // Optional evidence artifact creation is explicitly controlled by the caller.
    if let Ok(path) = std::env::var("SARV_EXPORT_ARTIFACT_DIR") {
        let destination = Path::new(&path);
        std::fs::create_dir_all(destination).unwrap();
        std::fs::copy(&pdf, destination.join("multipage-quote.pdf")).unwrap();
        for (index, png) in pngs.iter().enumerate() {
            std::fs::copy(
                png,
                destination.join(format!("multipage-{:02}.png", index + 1)),
            )
            .unwrap();
        }
        let sample = quote(4);
        export::pdf(&sample, &destination.join("quote-table.pdf")).unwrap();
        let previews = export::png_pages(&sample, destination).unwrap();
        std::fs::copy(&previews[0], destination.join("quote-table.png")).unwrap();
        for png in previews {
            std::fs::remove_file(png).unwrap();
        }
        let mut draft = sample;
        draft.id.clear();
        draft.version = 0;
        draft.approved_at.clear();
        let previews = export::png_pages(&draft, destination).unwrap();
        std::fs::copy(&previews[0], destination.join("quote-draft.png")).unwrap();
        for png in previews {
            std::fs::remove_file(png).unwrap();
        }
    }
}

#[test]
fn unreasonably_tall_row_is_an_explicit_error_not_clipped_output() {
    let mut q = quote(1);
    q.lines[0].measure = "واحد بسیار بلند ".repeat(200);
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("clipped.pdf");
    let error = export::pdf(&q, &destination).unwrap_err();
    assert!(error.to_string().contains("بسیار بلند"));
    assert!(!destination.exists());
}

#[test]
fn largest_supported_amount_and_rial_unit_render_without_float_rounding() {
    let mut q = quote(1);
    q.display_unit = Unit::Rial;
    q.lines[0].quantity_milli = 1000;
    q.lines[0].price_rial = MAX_MONEY;
    q.lines[0].total_rial = MAX_MONEY;
    q.total_rial = MAX_MONEY - q.discount_rial;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("largest.pdf");
    export::pdf(&q, &path).unwrap();
    assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF"));
    assert!(!export::png_pages(&q, dir.path()).unwrap().is_empty());
    let bytes = export::csv_bytes(&q).unwrap();
    let csv = String::from_utf8(bytes).unwrap();
    assert!(csv.contains(&MAX_MONEY.to_string()));
    assert!(csv.contains(&(MAX_MONEY - 13).to_string()));
}

fn ink_count(path: &Path, x: usize, y: usize, width: usize, height: usize) -> usize {
    let mut file = std::fs::File::open(path).unwrap();
    let mut image = cairo::ImageSurface::create_from_png(&mut file).unwrap();
    let stride = image.stride() as usize;
    let data = image.data().unwrap();
    let mut count = 0;
    for row in y..y + height {
        for column in x..x + width {
            let start = row * stride + column * 4;
            let pixel = u32::from_ne_bytes(data[start..start + 4].try_into().unwrap());
            let (red, green, blue) = ((pixel >> 16) & 255, (pixel >> 8) & 255, pixel & 255);
            if red < 180 && green < 180 && blue < 180 {
                count += 1;
            }
        }
    }
    count
}

fn monetary_row_bounds(path: &Path, count: usize) -> Vec<(usize, usize)> {
    let mut file = std::fs::File::open(path).unwrap();
    let mut image = cairo::ImageSurface::create_from_png(&mut file).unwrap();
    let stride = image.stride() as usize;
    let height = image.height() as usize;
    assert_eq!(image.width(), 1190, "the document raster must be 144 dpi");
    let data = image.data().unwrap();
    let pixel = |x: usize, y: usize| {
        let start = y * stride + x * 4;
        let pixel = u32::from_ne_bytes(data[start..start + 4].try_into().unwrap());
        ((pixel >> 16) & 255, (pixel >> 8) & 255, pixel & 255)
    };
    // Read the actual table's shaded header and full-width row rules. Pango
    // measures wrapped descriptions, so row heights legitimately change with
    // font metrics; probing old absolute y coordinates can inspect empty space.
    let header = (0..height)
        .find(|&y| {
            let (r, g, b) = pixel(74, y);
            (242..=249).contains(&r) && g > r && b > g
        })
        .expect("the table must have a visible shaded header");
    let mut rules = Vec::new();
    let mut previous_was_rule = false;
    for y in header..height {
        let rule_pixels = (74..1116)
            .filter(|&x| {
                let (r, g, b) = pixel(x, y);
                r > 180 && b < 242 && g > r && b > g
            })
            .count();
        let is_rule = rule_pixels > 1000;
        if is_rule && !previous_was_rule {
            rules.push(y);
        }
        previous_was_rule = is_rule;
        if rules.len() == count + 1 {
            break;
        }
    }
    assert_eq!(rules.len(), count + 1, "every table row must have a rule");
    rules
        .windows(2)
        .map(|pair| (pair[0] + 3, pair[1] - pair[0] - 6))
        .collect()
}

#[test]
fn final_raster_has_every_monetary_cell_and_matches_independent_pdf_raster() {
    let mut q = quote(4);
    let directory = tempfile::tempdir().unwrap();
    let approved = export::png_pages(&q, directory.path()).unwrap();
    let pdf = directory.path().join("independent.pdf");
    export::pdf(&q, &pdf).unwrap();
    let prefix = directory.path().join("independent-raster");
    let independent = Command::new("pdftoppm")
        .args(["-r", "144", "-png", "-singlefile"])
        .arg(&pdf)
        .arg(&prefix)
        .output()
        .ok();
    if let Some(output) = &independent {
        assert!(output.status.success());
    }
    let independent_png = prefix.with_extension("png");
    q.id.clear();
    q.version = 0;
    q.approved_at.clear();
    let draft = export::png_pages(&q, directory.path()).unwrap();
    let approved_bounds = monetary_row_bounds(&approved[0], q.lines.len());
    let draft_bounds = monetary_row_bounds(&draft[0], q.lines.len());
    let independent_bounds = independent
        .as_ref()
        .map(|_| monetary_row_bounds(&independent_png, q.lines.len()));
    // Monetary column positions are fixed. Each measured row region contains
    // only that row's numeric value, excluding headings, totals and rules.
    for (index, &(y, height)) in approved_bounds.iter().enumerate() {
        for x in [72, 266] {
            let count = ink_count(&approved[0], x, y, 194, height);
            assert!(count > 120, "missing monetary glyphs in cell {x}/{y}");
            let (draft_y, draft_height) = draft_bounds[index];
            assert_eq!(
                count,
                ink_count(&draft[0], x, draft_y, 194, draft_height),
                "draft monetary glyph positions differ"
            );
            if let Some(bounds) = &independent_bounds {
                let (independent_y, independent_height) = bounds[index];
                let independent_count =
                    ink_count(&independent_png, x, independent_y, 194, independent_height);
                assert!(
                    count.abs_diff(independent_count) < count / 5,
                    "native monetary cell disagrees with independently rasterized PDF"
                );
            }
        }
    }
    assert!(
        ink_count(&approved[0], 0, 0, 1190, 1150) > 10_000,
        "document title, metadata and table must all contain visible text"
    );
}

fn configured_quote() -> Quote {
    let mut q = quote(2);
    q.number = "SQ-0042".into();
    q.issuer_name = "پخش نمونه TEST-ISSUER".into();
    q.issuer_phone = "09121111111".into();
    q.issuer_address = "نشانی نمونه ADDRESS-SNAPSHOT".into();
    q.header = "سربرگ نمونه HEADER-SNAPSHOT".into();
    q.footer = "FOOTER-SNAPSHOT · شرایط نمونه".into();
    q.terms = "TERMS-SNAPSHOT · این شرایط صرفاً برای پیشنهاد آزمایشی است.".into();
    q.tax_basis_points = 925; // Deliberately configured test rate, no legal rate claim.
    q.tax_rial = tax_amount(q.total_rial, q.tax_basis_points).unwrap();
    q.total_rial += q.tax_rial;
    q.display_digits = DisplayDigits::Latin;
    q
}
fn pdf_text(path: &Path, directory: &Path) -> Option<String> {
    let output_path = directory.join("extracted.txt");
    let result = Command::new("pdftotext")
        .arg(path)
        .arg(&output_path)
        .output()
        .ok()?;
    assert!(result.status.success());
    Some(std::fs::read_to_string(output_path).unwrap())
}

#[test]
fn configured_business_number_tax_and_digits_are_in_real_pdf_and_csv() {
    let q = configured_quote();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("configured.pdf");
    export::pdf(&q, &path).unwrap();
    let pngs = export::png_pages(&q, dir.path()).unwrap();
    assert!(!pngs.is_empty());
    let bytes = export::csv_bytes(&q).unwrap();
    let mut reader = csv::Reader::from_reader(&bytes[3..]);
    let headers = reader.headers().unwrap().clone();
    let first = reader.records().next().unwrap().unwrap();
    for (column, expected) in [
        ("quote_number", "SQ-0042"),
        ("issuer_name", q.issuer_name.as_str()),
        ("issuer_address", q.issuer_address.as_str()),
        ("header", q.header.as_str()),
        ("footer", q.footer.as_str()),
        ("terms", q.terms.as_str()),
        ("tax_basis_points", "925"),
        ("tax_rial", "28906"),
        ("quote_total_rial", "341401"),
        ("display_digits", "latin"),
    ] {
        let index = headers.iter().position(|value| value == column).unwrap();
        assert_eq!(&first[index], expected, "CSV snapshot field {column}");
    }
    if let Some(text) = pdf_text(&path, dir.path()) {
        for marker in [
            "SQ-0042",
            "TEST-ISSUER",
            "ADDRESS-SNAPSHOT",
            "HEADER-SNAPSHOT",
            "FOOTER-SNAPSHOT",
            "TERMS-SNAPSHOT",
            "9.25",
            "34,140.1",
        ] {
            assert!(text.contains(marker), "missing PDF marker {marker}");
        }
    }
    if let Ok(folder) = std::env::var("SARV_SETTINGS_EXPORT_ARTIFACT_DIR") {
        let destination = Path::new(&folder);
        std::fs::create_dir_all(destination).unwrap();
        std::fs::copy(&path, destination.join("business-quote.pdf")).unwrap();
        std::fs::copy(&pngs[0], destination.join("business-quote.png")).unwrap();
        for (index, png) in pngs.iter().enumerate() {
            std::fs::copy(
                png,
                destination.join(format!("business-quote-{:02}.png", index + 1)),
            )
            .unwrap();
        }
        export::csv(&q, &destination.join("business-quote.csv")).unwrap();
    }
}

#[test]
fn business_snapshot_csv_fields_are_formula_safe() {
    let mut q = configured_quote();
    q.number = "+number".into();
    q.issuer_name = "=name".into();
    q.issuer_phone = "+phone".into();
    q.issuer_address = "@address".into();
    q.header = "\t=header".into();
    q.footer = "\n-footer".into();
    q.terms = "\u{feff}@terms".into();
    let bytes = export::csv_bytes(&q).unwrap();
    let mut reader = csv::Reader::from_reader(&bytes[3..]);
    let headers = reader.headers().unwrap().clone();
    let first = reader.records().next().unwrap().unwrap();
    for name in [
        "quote_number",
        "issuer_name",
        "issuer_phone",
        "issuer_address",
        "header",
        "footer",
        "terms",
    ] {
        let index = headers.iter().position(|value| value == name).unwrap();
        assert!(first[index].starts_with('\''), "unprotected {name}");
    }
}

#[test]
fn tax_errors_and_maximum_total_fail_before_any_export_file_is_created() {
    let directory = tempfile::tempdir().unwrap();
    let mut bad_tax = configured_quote();
    bad_tax.tax_rial += 1;
    assert!(export::pdf(&bad_tax, &directory.path().join("bad-tax.pdf")).is_err());
    assert!(export::csv_bytes(&bad_tax).is_err());
    bad_tax.tax_basis_points = 10_001;
    assert!(export::pdf(&bad_tax, &directory.path().join("bad-rate.pdf")).is_err());
    let mut too_large = quote(1);
    too_large.lines[0].quantity_milli = 1000;
    too_large.lines[0].price_rial = MAX_MONEY;
    too_large.lines[0].total_rial = MAX_MONEY;
    too_large.tax_basis_points = 10_000;
    too_large.tax_rial = MAX_MONEY - 13;
    too_large.total_rial = (MAX_MONEY - 13) * 2;
    assert!(export::pdf(&too_large, &directory.path().join("overflow.pdf")).is_err());
    assert!(export::csv_bytes(&too_large).is_err());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn legacy_quote_deserializes_without_settings_and_keeps_zero_tax() {
    let q = quote(2);
    let mut value = serde_json::to_value(&q).unwrap();
    for field in [
        "number",
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
    ] {
        value.as_object_mut().unwrap().remove(field);
    }
    let old: Quote = serde_json::from_value(value).unwrap();
    assert_eq!(old.tax_rial, 0);
    assert_eq!(old.tax_basis_points, 0);
    assert_eq!(old.display_digits, DisplayDigits::Persian);
    assert_eq!(old.date_display, DateDisplay::Iso);
    assert!(old.number.is_empty());
    assert_eq!(old.total_rial, q.total_rial);
    let dir = tempfile::tempdir().unwrap();
    export::pdf(&old, &dir.path().join("legacy.pdf")).unwrap();
}

#[test]
fn long_valid_business_blocks_are_paginated_without_losing_content() {
    let mut q = configured_quote();
    q.issuer_address = (0..80)
        .map(|index| format!("ADDRESS-LONG-{index:03}\n"))
        .collect();
    q.header = (0..80)
        .map(|index| format!("HEADER-LONG-{index:03}\n"))
        .collect();
    q.footer = (0..80)
        .map(|index| format!("FOOTER-LONG-{index:03}\n"))
        .collect();
    q.terms = (0..80)
        .map(|index| format!("TERMS-LONG-{index:03}\n"))
        .collect();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("long-business.pdf");
    export::pdf(&q, &path).unwrap();
    if let Some(text) = pdf_text(&path, dir.path()) {
        for prefix in ["ADDRESS-LONG", "HEADER-LONG", "FOOTER-LONG", "TERMS-LONG"] {
            for index in 0..80 {
                assert!(
                    text.contains(&format!("{prefix}-{index:03}")),
                    "missing {prefix} row {index}"
                );
            }
        }
    }
}

#[test]
fn saved_date_display_changes_pdf_but_keeps_csv_canonical_iso() {
    let mut q = configured_quote();
    q.date_display = DateDisplay::DayMonthYear;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("day-month-year.pdf");
    export::pdf(&q, &path).unwrap();
    if let Some(text) = pdf_text(&path, directory.path()) {
        for date in [
            "07/10/2026",
            "14/10/2026",
            "01/09/2026 08:00 UTC",
            "07/10/2026 10:00 UTC",
        ] {
            assert!(text.contains(date), "missing formatted PDF date {date}");
        }
    }
    let bytes = export::csv_bytes(&q).unwrap();
    let mut reader = csv::Reader::from_reader(&bytes[3..]);
    let headers = reader.headers().unwrap().clone();
    let first = reader.records().next().unwrap().unwrap();
    for (column, expected) in [
        ("quote_date", "2026-10-07"),
        ("valid_until", "2026-10-14"),
        ("source_at", "2026-09-01T08:00:00Z"),
        ("approved_at", "2026-10-07T10:00:00Z"),
        ("date_display", "day_month_year"),
    ] {
        let index = headers.iter().position(|value| value == column).unwrap();
        assert_eq!(
            &first[index], expected,
            "CSV {column} must retain canonical stored value"
        );
    }
    assert_eq!(q.quote_date, "2026-10-07");
    assert_eq!(q.lines[0].source_at, "2026-09-01T08:00:00Z");
}

#[test]
fn customer_snapshot_exports_keep_exact_fields_and_paginate_long_addresses() {
    let mut q = configured_quote();
    q.customer_id = "2d037712-a88d-4303-9a6b-9e859a2b8aa0".into();
    q.customer = format!(
        "CUSTOMER-LONG-NAME {}",
        "فروشگاه نمونه و شعبهٔ مرکزی ".repeat(8)
    );
    q.customer_contact = "شخص رابط CONTACT-SNAPSHOT".into();
    q.customer_email = "snapshot@example.test".into();
    q.customer_address = (0..100)
        .map(|index| format!("نشانی مشتری CUSTOMER-ADDRESS-{index:03}\n"))
        .collect();
    let original_total = q.total_rial;
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("customer-snapshot.pdf");
    export::pdf(&q, &pdf).unwrap();
    if let Some(text) = pdf_text(&pdf, dir.path()) {
        assert!(text.contains("CUSTOMER-LONG-NAME"));
        assert!(text.contains("CONTACT-SNAPSHOT"));
        assert!(text.contains("snapshot@example.test"));
        for index in 0..100 {
            assert!(
                text.contains(&format!("CUSTOMER-ADDRESS-{index:03}")),
                "missing customer address line {index}"
            );
        }
    }
    let bytes = export::csv_bytes(&q).unwrap();
    let mut reader = csv::Reader::from_reader(&bytes[3..]);
    let headers = reader.headers().unwrap().clone();
    let first = reader.records().next().unwrap().unwrap();
    for (column, expected) in [
        ("customer", q.customer.as_str()),
        ("customer_id", q.customer_id.as_str()),
        ("customer_contact", q.customer_contact.as_str()),
        ("customer_email", q.customer_email.as_str()),
        ("customer_address", q.customer_address.as_str()),
    ] {
        let index = headers.iter().position(|value| value == column).unwrap();
        assert_eq!(
            &first[index], expected,
            "CSV must retain customer snapshot {column}"
        );
    }
    let total = headers
        .iter()
        .position(|value| value == "quote_total_rial")
        .unwrap();
    assert_eq!(&first[total], original_total.to_string());
    assert!(pdf_page_count(&pdf).unwrap_or(3) >= 3);
}

#[test]
fn customer_snapshot_csv_is_formula_safe_and_legacy_defaults_remain_absent() {
    let original = configured_quote();
    let value = serde_json::to_value(&original).unwrap();
    for field in [
        "customer_id",
        "customer_contact",
        "customer_email",
        "customer_address",
    ] {
        assert!(
            value.get(field).is_none(),
            "empty optional snapshot {field} must preserve legacy serialization"
        );
    }
    let decoded: Quote = serde_json::from_value(value).unwrap();
    assert_eq!(
        export::csv_bytes(&original).unwrap(),
        export::csv_bytes(&decoded).unwrap()
    );
    assert_eq!(decoded.total_rial, original.total_rial);
    let mut q = original;
    q.customer_id = "=unsafe-id".into();
    q.customer_contact = "\t=unsafe-contact".into();
    q.customer_email = "+unsafe-email".into();
    q.customer_address = "\n@unsafe-address".into();
    let bytes = export::csv_bytes(&q).unwrap();
    let mut reader = csv::Reader::from_reader(&bytes[3..]);
    let headers = reader.headers().unwrap().clone();
    let first = reader.records().next().unwrap().unwrap();
    for column in [
        "customer_id",
        "customer_contact",
        "customer_email",
        "customer_address",
    ] {
        let index = headers.iter().position(|value| value == column).unwrap();
        assert!(
            first[index].starts_with('\''),
            "unprotected customer snapshot {column}"
        );
    }
}
