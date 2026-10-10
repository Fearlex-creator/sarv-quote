#![cfg(feature = "pdf-export")]
use sarv_quote::{
    accounting::{AccountingState, Money, MoneyUnit},
    core::*,
    core_import,
    domain::{DateDisplay, DisplayDigits, State},
    procurement::*,
    procurement_export::*,
    store::{Store, backup_state},
};
use std::{fs, path::Path, process::Command};
fn money(n: i64) -> Money {
    Money::from_rials(n)
}
struct Fixture {
    state: AccountingState,
    company: String,
    period: String,
    setup: String,
    order: String,
    receipt_a: String,
    receipt_b: String,
    bill_a: String,
    bill_b: String,
    transfer: String,
}
fn context(f: &Fixture, id: &str, date: &str) -> ProcurementContext {
    ProcurementContext {
        company_id: f.company.clone(),
        period_id: f.period.clone(),
        setup_id: f.setup.clone(),
        source_id: id.into(),
        source_revision: 1,
        date: date.into(),
        request_key: format!("key-{id}"),
        actor: "آزمایشگر".into(),
        reason: "منبع مستقل مصنوعی".into(),
    }
}
fn fixture() -> Fixture {
    let mut state = AccountingState::default();
    let c = state
        .setup_core_company(NewCoreCompany {
            name: "شرکت مصنوعی TEST-PROCUREMENT".into(),
            fiscal_name: "۱۴۰۵".into(),
            start_date: "2026-03-21".into(),
            end_date: "2027-03-20".into(),
            timezone: CompanyTimezone::AsiaTehran,
            chart: default_chart(),
            actor: "آزمایشگر".into(),
        })
        .unwrap();
    let input = include_str!("../../../docs/assets/accounting-m3/opening.csv")
        .replace("F2-core-v1", &c.company_id);
    let opening =
        core_import::opening_csv(&state, &c.company_id, &input, "آزمایشگر", "opening").unwrap();
    state.post_opening(opening).unwrap();
    let mut f = Fixture {
        state,
        company: c.company_id,
        period: c.period_id,
        setup: c.setup_id,
        order: String::new(),
        receipt_a: String::new(),
        receipt_b: String::new(),
        bill_a: String::new(),
        bill_b: String::new(),
        transfer: String::new(),
    };
    let po = ProcurementCommand::Order(PurchaseOrder {
        context: context(&f, "=PO-SOURCE()", "2026-03-22"),
        supplier_id: "SU-A".into(),
        reference: "=PO-REFERENCE()".into(),
        document_date: "2026-03-22".into(),
        lines: vec![OrderLine {
            id: "PO-LINE".into(),
            item_id: "ITEM-A".into(),
            warehouse_id: "W1".into(),
            unit_id: "PIECE".into(),
            quantity_thousandths: 12000,
            unit_price: money(15),
            line_discount: Money::ZERO,
            price_reference: "=PRICE-ORIGIN()".into(),
            price_date: "2026-03-20".into(),
            override_reason: None,
        }],
        document_discount: Money::ZERO,
        supplied_total: money(180),
    });
    f.order = f.state.post_procurement_command(po).unwrap().record_id;
    for (id, line, qty, value, date) in [
        ("=R-A-SOURCE()", "R-A-LINE", 5000, 75, "2026-03-22"),
        ("R-B-SOURCE", "R-B-LINE", 7000, 105, "2026-03-23"),
    ] {
        let cmd = ProcurementCommand::Receive(ReceiveOrder {
            context: context(&f, id, date),
            order_id: f.order.clone(),
            reference: format!("REF-{line}"),
            document_date: date.into(),
            lines: vec![ReceiveLine {
                id: line.into(),
                order_line_id: "PO-LINE".into(),
                warehouse_id: "W1".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: qty,
            }],
            supplied_total: money(value),
        });
        let result = f.state.post_procurement_command(cmd).unwrap();
        if qty == 5000 {
            f.receipt_a = result.record_id
        } else {
            f.receipt_b = result.record_id
        }
    }
    for (id, date, lines, total) in [
        (
            "BILL-A",
            "2026-03-24",
            vec![BillLine {
                id: "BILL-A-LINE".into(),
                receipt_line_id: "R-A-LINE".into(),
                quantity_thousandths: 2000,
                supplied_value: money(30),
            }],
            30,
        ),
        (
            "BILL-B",
            "2026-03-25",
            vec![
                BillLine {
                    id: "BILL-B-LINE-A".into(),
                    receipt_line_id: "R-A-LINE".into(),
                    quantity_thousandths: 3000,
                    supplied_value: money(45),
                },
                BillLine {
                    id: "BILL-B-LINE-B".into(),
                    receipt_line_id: "R-B-LINE".into(),
                    quantity_thousandths: 7000,
                    supplied_value: money(105),
                },
            ],
            150,
        ),
    ] {
        let cmd = ProcurementCommand::Bill(MatchBill {
            context: context(&f, id, date),
            supplier_id: "SU-A".into(),
            reference: format!("=INVOICE-{id}()"),
            document_date: date.into(),
            due_date: "2026-04-01".into(),
            lines,
            supplied_total: money(total),
        });
        let result = f.state.post_procurement_command(cmd).unwrap();
        if total == 30 {
            f.bill_a = result.record_id
        } else {
            f.bill_b = result.record_id
        }
    }
    let cmd = ProcurementCommand::Transfer(TransferStock {
        context: context(&f, "TRANSFER-SOURCE", "2026-03-26"),
        item_id: "ITEM-A".into(),
        source_warehouse_id: "W1".into(),
        target_warehouse_id: "W2".into(),
        unit_id: "PIECE".into(),
        quantity_thousandths: 6000,
    });
    f.transfer = f.state.post_procurement_command(cmd).unwrap().record_id;
    f
}
fn read_csv(path: &Path) -> (csv::StringRecord, Vec<csv::StringRecord>) {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..3], &[0xef, 0xbb, 0xbf]);
    let mut r = csv::Reader::from_reader(&bytes[3..]);
    let h = r.headers().unwrap().clone();
    let rows = r.records().map(|r| r.unwrap()).collect();
    (h, rows)
}
fn cell<'a>(h: &csv::StringRecord, r: &'a csv::StringRecord, k: &str) -> &'a str {
    &r[h.iter().position(|x| x == k).unwrap()]
}
fn export_csv(f: &Fixture, date: &str, path: &Path) {
    reconciliation_csv(
        &f.state,
        &f.company,
        date,
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        DisplayDigits::Latin,
        path,
    )
    .unwrap();
}
#[test]
fn literal_end_of_day_controls_and_partial_sources_remain_exact_and_read_only() {
    let f = fixture();
    let before = f.state.clone();
    let dir = tempfile::tempdir().unwrap();
    for (date, stock, ap, grni, tb) in [
        ("2026-03-21", 300, 149, 50, 1251),
        ("2026-03-22", 375, 149, 125, 1326),
        ("2026-03-23", 480, 149, 230, 1431),
        ("2026-03-24", 480, 179, 200, 1431),
        ("2026-03-25", 480, 329, 50, 1431),
        ("2026-03-26", 480, 329, 50, 1431),
    ] {
        let p = dir.path().join(format!("{date}.csv"));
        export_csv(&f, date, &p);
        let (h, rows) = read_csv(&p);
        for (role, expected) in [("inventory", stock), ("payables", ap), ("grni", grni)] {
            let row = rows
                .iter()
                .find(|r| cell(&h, r, "row_kind") == "control" && cell(&h, r, "role") == role)
                .unwrap();
            assert_eq!(cell(&h, row, "ledger_irr"), expected.to_string());
            assert_eq!(cell(&h, row, "detail_irr"), expected.to_string());
            assert_eq!(cell(&h, row, "difference_irr"), "0");
            assert_eq!(cell(&h, row, "basis"), "accrual_perpetual_end_of_day");
        }
        assert!(
            rows.iter()
                .filter(|r| cell(&h, r, "row_kind") == "control")
                .all(|r| cell(&h, r, "difference_irr") == "0")
        );
        let total = rows
            .iter()
            .find(|r| cell(&h, r, "row_kind") == "trial_total_debit")
            .unwrap();
        assert_eq!(cell(&h, total, "amount_irr"), tb.to_string());
        if date == "2026-03-21" {
            assert_eq!(cell(&h, total, "display_amount"), "125.1");
        }
        if date == "2026-03-24" {
            for (id, qty, value) in [
                ("R-A-LINE", "3000", "45"),
                ("R-B-LINE", "7000", "105"),
                ("R1", "5000", "50"),
            ] {
                let row = rows
                    .iter()
                    .find(|r| {
                        cell(&h, r, "row_kind") == "receipt_remaining"
                            && cell(&h, r, "row_id") == id
                    })
                    .unwrap();
                assert_eq!(cell(&h, row, "remaining_quantity_thousandths"), qty);
                assert_eq!(cell(&h, row, "amount_irr"), value);
                assert!(!cell(&h, row, "event_id").is_empty());
            }
        }
        if date == "2026-03-26" {
            for (wh, qty, value) in [("W1", "30000", "350"), ("W2", "12000", "130")] {
                let row = rows
                    .iter()
                    .find(|r| {
                        cell(&h, r, "row_kind") == "stock_pool" && cell(&h, r, "warehouse_id") == wh
                    })
                    .unwrap();
                assert_eq!(cell(&h, row, "base_quantity_thousandths"), qty);
                assert_eq!(cell(&h, row, "amount_irr"), value);
            }
        }
    }
    assert_eq!(f.state, before);
}
#[test]
fn frozen_documents_full_source_reconstruction_formula_guards_and_no_overwrite() {
    let f = fixture();
    let before = f.state.clone();
    let dir = tempfile::tempdir().unwrap();
    for id in [
        &f.order,
        &f.receipt_a,
        &f.receipt_b,
        &f.bill_a,
        &f.bill_b,
        &f.transfer,
    ] {
        let p = dir.path().join(format!("{id}.csv"));
        document_csv(
            &f.state,
            &f.company,
            id,
            MoneyUnit::Toman,
            DateDisplay::Jalali,
            DisplayDigits::Latin,
            &p,
        )
        .unwrap();
        let (h, rows) = read_csv(&p);
        let snapshot = rows
            .iter()
            .find(|r| cell(&h, r, "row_kind") == "complete_snapshot")
            .unwrap();
        let raw: serde_json::Value =
            serde_json::from_str(cell(&h, snapshot, "snapshot_json")).unwrap();
        let record: ProcurementRecord = serde_json::from_value(raw["document"].clone()).unwrap();
        assert_eq!(&record, f.state.procurement_document(id).unwrap());
        for row in &rows {
            let source = cell(&h, row, "report_source_id");
            assert!(!source.starts_with('='));
            let reference = cell(&h, row, "source_reference");
            assert!(!reference.starts_with('='));
            assert_eq!(cell(&h, row, "base_currency"), "IRR");
            assert_eq!(cell(&h, row, "policy_version"), "core-policy-v1");
        }
        let bytes = fs::read(&p).unwrap();
        assert!(
            document_csv(
                &f.state,
                &f.company,
                id,
                MoneyUnit::Rial,
                DateDisplay::Iso,
                DisplayDigits::Latin,
                &p
            )
            .is_err()
        );
        assert_eq!(fs::read(&p).unwrap(), bytes);
        if id == &f.transfer {
            assert_eq!(
                cell(&h, &rows[0], "included_states"),
                "recorded_nonfinancial_event"
            );
            assert_eq!(cell(&h, &rows[0], "financial_effect_kind"), "nonfinancial");
            assert!(record.journal_id.is_none());
            let line = rows
                .iter()
                .find(|r| cell(&h, r, "row_kind") == "transfer_summary")
                .unwrap();
            assert_eq!(cell(&h, line, "amount_irr"), "70");
            assert_eq!(cell(&h, line, "base_quantity_thousandths"), "6000");
        }
    }
    assert_eq!(f.state, before);
    // A later unrelated registry event must not rewrite an old document export.
    let old = dir.path().join("order-before-resources.csv");
    document_csv(
        &f.state,
        &f.company,
        &f.order,
        MoneyUnit::Rial,
        DateDisplay::Iso,
        DisplayDigits::Latin,
        &old,
    )
    .unwrap();
    let mut later = f.state.clone();
    later
        .post_procurement_command(ProcurementCommand::Resources(RegisterResources {
            context: context(&f, "LATER-MASTERS", "2026-03-27"),
            parties: vec![CoreParty {
                id: "LATER-PARTY".into(),
                company_id: f.company.clone(),
                name: "شخص بعدی غیرمرتبط".into(),
                customer: false,
                supplier: true,
            }],
            units: vec![],
            items: vec![],
            warehouses: vec![],
        }))
        .unwrap();
    let after = dir.path().join("order-after-resources.csv");
    document_csv(
        &later,
        &f.company,
        &f.order,
        MoneyUnit::Rial,
        DateDisplay::Iso,
        DisplayDigits::Latin,
        &after,
    )
    .unwrap();
    assert_eq!(fs::read(old).unwrap(), fs::read(after).unwrap());
    let ProcurementCommand::Order(mut free_order) = f
        .state
        .procurement_document(&f.order)
        .unwrap()
        .command
        .clone()
    else {
        panic!("fixture PO");
    };
    free_order.context = context(&f, "FREE-PO", "2026-03-27");
    free_order.reference = "FREE-PO-REF".into();
    free_order.document_date = "2026-03-27".into();
    free_order.lines[0].id = "FREE-PO-LINE".into();
    free_order.lines[0].quantity_thousandths = 1000;
    free_order.lines[0].unit_price = Money::ZERO;
    free_order.supplied_total = Money::ZERO;
    let before_journals = later.journals.len();
    let free_id = later
        .post_procurement_command(ProcurementCommand::Order(free_order))
        .unwrap()
        .record_id;
    let receipt = later
        .post_procurement_command(ProcurementCommand::Receive(ReceiveOrder {
            context: context(&f, "FREE-RECEIPT", "2026-03-27"),
            order_id: free_id,
            reference: "FREE-RECEIPT-REF".into(),
            document_date: "2026-03-27".into(),
            lines: vec![ReceiveLine {
                id: "FREE-RECEIPT-LINE".into(),
                order_line_id: "FREE-PO-LINE".into(),
                warehouse_id: "W1".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: 1000,
            }],
            supplied_total: Money::ZERO,
        }))
        .unwrap();
    assert!(receipt.journal_id.is_none());
    assert_eq!(later.journals.len(), before_journals);
    let free_csv = dir.path().join("zero-value-receipt.csv");
    document_csv(
        &later,
        &f.company,
        &receipt.record_id,
        MoneyUnit::Rial,
        DateDisplay::Iso,
        DisplayDigits::Latin,
        &free_csv,
    )
    .unwrap();
    let (h, rows) = read_csv(&free_csv);
    assert_eq!(cell(&h, &rows[0], "financial_effect_kind"), "nonfinancial");
    assert_eq!(
        cell(&h, &rows[0], "included_states"),
        "recorded_nonfinancial_event"
    );
    let line = rows
        .iter()
        .find(|r| cell(&h, r, "row_kind") == "receipt_line")
        .unwrap();
    assert_eq!(cell(&h, line, "base_quantity_thousandths"), "1000");
    assert_eq!(cell(&h, line, "amount_irr"), "0");
    let free_pdf = dir.path().join("zero-value-receipt.pdf");
    document_pdf(
        &later,
        &f.company,
        &receipt.record_id,
        MoneyUnit::Rial,
        DateDisplay::Iso,
        DisplayDigits::Latin,
        &free_pdf,
    )
    .unwrap();
    assert!(free_pdf.exists());
}
#[test]
fn actual_persian_pdf_canonical_dates_codes_units_source_markers_and_repeated_headers() {
    let f = fixture();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("reconciliation.pdf");
    reconciliation_pdf(
        &f.state,
        &f.company,
        "2026-03-26",
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        DisplayDigits::Latin,
        &p,
    )
    .unwrap();
    let out = Command::new("pdftotext").arg(&p).arg("-").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    for value in [
        "TEST-PROCUREMENT",
        "2026-03-26",
        "1405/01/06",
        "IRR",
        "toman",
        "core-policy-v1",
        "143.1",
        "R-A-LINE",
        "R-B-LINE",
        "R1",
        "30.000",
        "12.000",
        "SKU-A",
        "2101",
        "2201",
    ] {
        assert!(text.contains(value), "missing PDF text {value}");
    }
    let info = Command::new("pdfinfo").arg(&p).output().unwrap();
    assert!(info.status.success());
    let info = String::from_utf8(info.stdout).unwrap();
    let pages = info
        .lines()
        .find(|l| l.starts_with("Pages:"))
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap()
        .parse::<usize>()
        .unwrap();
    assert!(pages >= 2);
    assert_eq!(text.matches("TEST-PROCUREMENT").count(), pages);
    let transfer = dir.path().join("transfer.pdf");
    document_pdf(
        &f.state,
        &f.company,
        &f.transfer,
        MoneyUnit::Toman,
        DateDisplay::Iso,
        DisplayDigits::Latin,
        &transfer,
    )
    .unwrap();
    let out = Command::new("pdftotext")
        .arg(&transfer)
        .arg("-")
        .output()
        .unwrap();
    let transfer_text = String::from_utf8(out.stdout).unwrap();
    assert!(
        transfer_text.contains("6.000")
            && transfer_text.contains("70")
            && transfer_text.contains("2026-03-26")
    );
    if let Ok(destination) = std::env::var("SARV_PROCUREMENT_EXPORT_ARTIFACT_DIR") {
        let destination = Path::new(&destination);
        fs::create_dir_all(destination).unwrap();
        fs::copy(&p, destination.join("procurement-reconciliation.pdf")).unwrap();
        fs::write(
            destination.join("procurement-reconciliation-text.txt"),
            text,
        )
        .unwrap();
        fs::copy(&transfer, destination.join("warehouse-transfer.pdf")).unwrap();
        reconciliation_csv(
            &f.state,
            &f.company,
            "2026-03-26",
            MoneyUnit::Toman,
            DateDisplay::Jalali,
            DisplayDigits::Latin,
            &destination.join("procurement-reconciliation.csv"),
        )
        .unwrap();
        for (name, id) in [
            ("purchase-order", &f.order),
            ("receipt-a", &f.receipt_a),
            ("receipt-b", &f.receipt_b),
            ("bill-a", &f.bill_a),
            ("bill-b", &f.bill_b),
            ("warehouse-transfer", &f.transfer),
        ] {
            document_csv(
                &f.state,
                &f.company,
                id,
                MoneyUnit::Toman,
                DateDisplay::Jalali,
                DisplayDigits::Latin,
                &destination.join(format!("{name}.csv")),
            )
            .unwrap();
            if name != "warehouse-transfer" {
                document_pdf(
                    &f.state,
                    &f.company,
                    id,
                    MoneyUnit::Toman,
                    DateDisplay::Jalali,
                    DisplayDigits::Latin,
                    &destination.join(format!("{name}.pdf")),
                )
                .unwrap();
            }
        }
    }
}
#[test]
fn restored_procurement_exports_remain_identical_and_uninitialized_or_foreign_books_reject() {
    let f = fixture();
    let dir = tempfile::tempdir().unwrap();
    let mut state = State::default();
    state.accounting = f.state.clone();
    let backup = dir.path().join("procurement.sarv");
    backup_state(&state, &backup).unwrap();
    let restored = Store::restore_preview(&backup).unwrap();
    let a = dir.path().join("before.csv");
    let b = dir.path().join("after.csv");
    export_csv(&f, "2026-03-26", &a);
    reconciliation_csv(
        &restored.accounting,
        &f.company,
        "2026-03-26",
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        DisplayDigits::Latin,
        &b,
    )
    .unwrap();
    assert_eq!(fs::read(a).unwrap(), fs::read(b).unwrap());
    let missing = dir.path().join("missing.pdf");
    assert!(
        document_pdf(
            &f.state,
            "FOREIGN-COMPANY",
            &f.bill_a,
            MoneyUnit::Rial,
            DateDisplay::Iso,
            DisplayDigits::Latin,
            &missing
        )
        .is_err()
    );
    assert!(!missing.exists());
    let early = dir.path().join("before-opening.csv");
    assert!(
        reconciliation_csv(
            &f.state,
            &f.company,
            "2026-03-20",
            MoneyUnit::Rial,
            DateDisplay::Iso,
            DisplayDigits::Latin,
            &early
        )
        .is_err()
    );
    assert!(!early.exists());
    let source_dates = dir.path().join("receipt-source-dates.csv");
    export_csv(&f, "2026-03-26", &source_dates);
    let (h, rows) = read_csv(&source_dates);
    let r1 = rows
        .iter()
        .find(|r| cell(&h, r, "row_kind") == "receipt_remaining" && cell(&h, r, "row_id") == "R1")
        .unwrap();
    assert_eq!(cell(&h, r1, "document_date"), "2026-03-15");
    assert_eq!(cell(&h, r1, "availability_date"), "2026-03-21");
    let far = dir.path().join("supported-gregorian-cutoff.csv");
    reconciliation_csv(
        &f.state,
        &f.company,
        "9999-12-31",
        MoneyUnit::Rial,
        DateDisplay::Jalali,
        DisplayDigits::Latin,
        &far,
    )
    .unwrap();
    let (h, rows) = read_csv(&far);
    assert_eq!(cell(&h, &rows[0], "as_of_gregorian"), "9999-12-31");
    assert_eq!(cell(&h, &rows[0], "display_calendar"), "gregorian_fallback");
    assert!(cell(&h, &rows[0], "display_as_of").contains("Gregorian; Jalali unavailable"));
    let empty = dir.path().join("uninitialized.csv");
    assert!(
        reconciliation_csv(
            &AccountingState::default(),
            &f.company,
            "2026-03-26",
            MoneyUnit::Rial,
            DateDisplay::Iso,
            DisplayDigits::Latin,
            &empty
        )
        .is_err()
    );
    assert!(!empty.exists());
}
