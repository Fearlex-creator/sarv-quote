#![cfg(feature = "pdf-export")]
use sarv_quote::{
    accounting::{AccountingState, Money, MoneyUnit},
    core::*,
    core_import,
    domain::{DateDisplay, DisplayDigits, State},
    procurement::*,
    sales::*,
    sales_export::*,
    store::{Store, backup_state},
};
use std::{fs, path::Path, process::Command};
fn money(n: i64) -> Money {
    Money::from_rials(n)
}
struct Fixture {
    invoice_time_state: AccountingState,
    sale_order: String,
    fulfillment: String,
    returned: String,
    return_final: String,
    fulfillment_final: String,
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
            name: "شرکت مصنوعی TEST-SALES".into(),
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
        invoice_time_state: AccountingState::default(),
        sale_order: String::new(),
        fulfillment: String::new(),
        returned: String::new(),
        return_final: String::new(),
        fulfillment_final: String::new(),
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
    let sale = SalesCommand::Order(SalesOrder {
        context: context(&f, "SALE-ORDER", "2026-03-27"),
        customer_id: "CU-A".into(),
        reference: "=SALE-REFERENCE()".into(),
        document_date: "2026-03-27".into(),
        lines: vec![SalesOrderLine {
            id: "SALE-LINE".into(),
            item_id: "ITEM-A".into(),
            warehouse_id: "W1".into(),
            unit_id: "PIECE".into(),
            quantity_thousandths: 10000,
            unit_price: money(20),
            line_discount: Money::ZERO,
            price_reference: "=SALE-PRICE()".into(),
            price_date: "2026-03-20".into(),
            override_reason: None,
        }],
        document_discount: money(1),
        supplied_total: money(199),
        quotation: None,
    });
    f.sale_order = f.state.post_sales_command(sale).unwrap().record_id;
    f.fulfillment = f
        .state
        .post_sales_command(SalesCommand::Fulfill(FulfillOrder {
            context: context(&f, "SALE-FULFILL", "2026-03-28"),
            order_id: f.sale_order.clone(),
            delivery_id: "DELIVERY-A".into(),
            invoice_id: "INVOICE-A".into(),
            delivery_reference: "=DELIVERY-A()".into(),
            invoice_reference: "=INVOICE-A()".into(),
            document_date: "2026-03-27".into(),
            due_date: "2026-04-15".into(),
            lines: vec![FulfillLine {
                id: "ISSUE-A".into(),
                order_line_id: "SALE-LINE".into(),
                warehouse_id: "W1".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: 5000,
            }],
            supplied_total: money(100),
        }))
        .unwrap()
        .record_id;
    f.invoice_time_state = f.state.clone();
    for (source, return_id, credit_id, line, quantity, credit, cost, date) in [
        (
            "SALE-RETURN",
            "RETURN-A",
            "CREDIT-A",
            "RETURN-LINE-A",
            2000,
            40,
            23,
            "2026-03-29",
        ),
        (
            "SALE-RETURN-FINAL",
            "RETURN-FINAL",
            "CREDIT-FINAL",
            "RETURN-LINE-FINAL",
            3000,
            60,
            35,
            "2026-03-30",
        ),
    ] {
        let result = f
            .state
            .post_sales_command(SalesCommand::Return(ReturnSale {
                context: context(&f, source, date),
                fulfillment_id: f.fulfillment.clone(),
                return_id: return_id.into(),
                credit_id: credit_id.into(),
                return_reference: format!("REF-{return_id}"),
                credit_reference: format!("REF-{credit_id}"),
                document_date: date.into(),
                lines: vec![ReturnLine {
                    id: line.into(),
                    fulfillment_line_id: "ISSUE-A".into(),
                    quantity_thousandths: quantity,
                    supplied_credit: money(credit),
                    supplied_cost: money(cost),
                }],
                supplied_credit: money(credit),
                supplied_cost: money(cost),
            }))
            .unwrap();
        if quantity == 2000 {
            f.returned = result.record_id
        } else {
            f.return_final = result.record_id
        }
    }
    f.fulfillment_final = f
        .state
        .post_sales_command(SalesCommand::Fulfill(FulfillOrder {
            context: context(&f, "SALE-FULFILL-FINAL", "2026-03-31"),
            order_id: f.sale_order.clone(),
            delivery_id: "DELIVERY-B".into(),
            invoice_id: "INVOICE-B".into(),
            delivery_reference: "REF-DELIVERY-B".into(),
            invoice_reference: "REF-INVOICE-B".into(),
            document_date: "2026-03-31".into(),
            due_date: "2026-04-15".into(),
            lines: vec![FulfillLine {
                id: "ISSUE-B".into(),
                order_line_id: "SALE-LINE".into(),
                warehouse_id: "W1".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: 5000,
            }],
            supplied_total: money(99),
        }))
        .unwrap()
        .record_id;
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
fn literal_controls_asof_receivables_and_original_cost_returns() {
    let f = fixture();
    let before = f.state.clone();
    let dir = tempfile::tempdir().unwrap();
    for (date, inventory, ar, sales, returns, cogs, profit, tb) in [
        ("2026-03-27", 480, 200, 0, 0, 0, 0, 1431),
        ("2026-03-28", 422, 300, 100, 0, 58, 42, 1531),
        ("2026-03-29", 445, 260, 100, 40, 35, 25, 1531),
        ("2026-03-30", 480, 200, 100, 100, 0, 0, 1531),
        ("2026-03-31", 422, 299, 199, 100, 58, 41, 1630),
    ] {
        let path = dir.path().join(format!("{date}.csv"));
        export_csv(&f, date, &path);
        let (h, rows) = read_csv(&path);
        for (role, value) in [
            ("inventory", inventory),
            ("receivables", ar),
            ("sales", sales),
            ("sales_returns", returns),
            ("cogs", cogs),
        ] {
            let row = rows
                .iter()
                .find(|r| cell(&h, r, "row_kind") == "control" && cell(&h, r, "role") == role)
                .unwrap();
            assert_eq!(cell(&h, row, "ledger_irr"), value.to_string());
            assert_eq!(cell(&h, row, "detail_irr"), value.to_string());
            assert_eq!(cell(&h, row, "difference_irr"), "0");
        }
        for (kind, value) in [
            ("profit_total", profit),
            ("trial_total_debit", tb),
            ("receivables_total", ar),
        ] {
            let row = rows
                .iter()
                .find(|r| cell(&h, r, "row_kind") == kind)
                .unwrap();
            assert_eq!(cell(&h, row, "amount_irr"), value.to_string());
        }
        assert!(rows.iter().all(|r| cell(&h, r, "base_currency") == "IRR"
            && cell(&h, r, "basis") == "accrual_perpetual_end_of_day"));
        if date == "2026-03-29" {
            let invoice = rows
                .iter()
                .find(|r| {
                    cell(&h, r, "row_kind") == "receivable_remaining"
                        && cell(&h, r, "row_id") == "INVOICE-A"
                })
                .unwrap();
            assert_eq!(cell(&h, invoice, "amount_irr"), "60");
            assert_eq!(cell(&h, invoice, "display_amount"), "6");
            assert_eq!(cell(&h, invoice, "original_value_irr"), "100");
            assert_eq!(cell(&h, invoice, "document_date"), "2026-03-27");
            assert_eq!(cell(&h, invoice, "due_date"), "2026-04-15");
            assert_eq!(cell(&h, invoice, "credit_ids_json"), "[\"CREDIT-A\"]");
        }
    }
    assert_eq!(f.state, before);
}
#[test]
fn paired_identities_exact_allocations_formula_guards_and_immutable_snapshots() {
    let f = fixture();
    let dir = tempfile::tempdir().unwrap();
    for (id, kind, gross, discount, net, cost) in [
        (f.sale_order.as_str(), "order_line", 200, 1, 199, None),
        ("INVOICE-A", "fulfillment_line", 101, 1, 100, Some(58)),
        ("CREDIT-A", "return_line", 40, 0, 40, Some(23)),
        ("CREDIT-FINAL", "return_line", 61, 1, 60, Some(35)),
        ("INVOICE-B", "fulfillment_line", 99, 0, 99, Some(58)),
    ] {
        let path = dir.path().join(format!("{id}.csv"));
        document_csv(
            &f.state,
            &f.company,
            id,
            MoneyUnit::Toman,
            DateDisplay::Jalali,
            DisplayDigits::Latin,
            &path,
        )
        .unwrap();
        let (h, rows) = read_csv(&path);
        let line = rows
            .iter()
            .find(|r| cell(&h, r, "row_kind") == kind)
            .unwrap();
        assert_eq!(cell(&h, line, "gross_irr"), gross.to_string());
        assert_eq!(
            cell(&h, line, "document_discount_irr"),
            discount.to_string()
        );
        assert_eq!(cell(&h, line, "net_irr"), net.to_string());
        if let Some(cost) = cost {
            assert_eq!(cell(&h, line, "cost_irr"), cost.to_string());
        }
        let snapshot = rows
            .iter()
            .find(|r| cell(&h, r, "row_kind") == "complete_snapshot")
            .unwrap();
        let raw: serde_json::Value =
            serde_json::from_str(cell(&h, snapshot, "snapshot_json")).unwrap();
        let record: SalesRecord = serde_json::from_value(raw["document"].clone()).unwrap();
        assert!(f.state.sales.records.contains(&record));
        assert!(
            rows.iter()
                .all(|r| !cell(&h, r, "source_reference").starts_with('=')
                    && !cell(&h, r, "price_reference").starts_with('='))
        );
        assert_eq!(cell(&h, &rows[0], "quote_id"), "");
        let original = fs::read(&path).unwrap();
        assert!(
            document_csv(
                &f.state,
                &f.company,
                id,
                MoneyUnit::Rial,
                DateDisplay::Iso,
                DisplayDigits::Latin,
                &path
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    let before_returns = f.invoice_time_state.clone();
    let mut later = f.state.clone();
    later
        .post_procurement_command(ProcurementCommand::Resources(RegisterResources {
            context: context(&f, "UNRELATED-MASTER", "2026-04-01"),
            parties: vec![CoreParty {
                id: "LATER-PARTY".into(),
                company_id: f.company.clone(),
                name: "شخص جدید".into(),
                customer: true,
                supplier: false,
            }],
            items: vec![],
            warehouses: vec![],
            units: vec![],
        }))
        .unwrap();
    let a = dir.path().join("before.csv");
    let b = dir.path().join("after.csv");
    for (state, path) in [(&before_returns, &a), (&later, &b)] {
        document_csv(
            state,
            &f.company,
            "INVOICE-A",
            MoneyUnit::Rial,
            DateDisplay::Iso,
            DisplayDigits::Latin,
            path,
        )
        .unwrap();
    }
    assert_eq!(fs::read(a).unwrap(), fs::read(b).unwrap());
}
#[test]
fn restored_exports_are_identical_foreign_missing_and_early_books_fail() {
    let f = fixture();
    let dir = tempfile::tempdir().unwrap();
    let mut state = State::default();
    state.accounting = f.state.clone();
    let backup = dir.path().join("sales.sarv");
    backup_state(&state, &backup).unwrap();
    let restored = Store::restore_preview(&backup).unwrap();
    let a = dir.path().join("before.csv");
    let b = dir.path().join("after.csv");
    export_csv(&f, "2026-03-31", &a);
    reconciliation_csv(
        &restored.accounting,
        &f.company,
        "2026-03-31",
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        DisplayDigits::Latin,
        &b,
    )
    .unwrap();
    assert_eq!(fs::read(a).unwrap(), fs::read(b).unwrap());
    for (company, id) in [("FOREIGN", "INVOICE-A"), (f.company.as_str(), "MISSING")] {
        let path = dir.path().join(format!("{company}-{id}.pdf"));
        assert!(
            document_pdf(
                &f.state,
                company,
                id,
                MoneyUnit::Rial,
                DateDisplay::Iso,
                DisplayDigits::Latin,
                &path
            )
            .is_err()
        );
        assert!(!path.exists());
    }
    let early = dir.path().join("early.csv");
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
    let far = dir.path().join("far.csv");
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
    assert_eq!(cell(&h, &rows[0], "display_calendar"), "gregorian_fallback");
    assert!(cell(&h, &rows[0], "display_as_of").contains("Gregorian; Jalali unavailable"));
}
#[test]
fn real_persian_pdf_embeds_licensed_font_repeated_headings_and_exact_lineage() {
    let f = fixture();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("reconciliation.pdf");
    reconciliation_pdf(
        &f.state,
        &f.company,
        "2026-03-31",
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
        "TEST-SALES",
        "2026-03-31",
        "1405/01/11",
        "IRR",
        "toman",
        "core-policy-v1",
        "163",
        "INVOICE-A",
        "INVOICE-B",
        "CREDIT-A",
        "CREDIT-FINAL",
        "SKU-A",
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
    assert_eq!(text.matches("TEST-SALES").count(), pages);
    let fonts = Command::new("pdffonts").arg(&p).output().unwrap();
    assert!(fonts.status.success());
    let fonts = String::from_utf8(fonts.stdout).unwrap();
    assert!(
        fonts.lines().any(|l| l.contains("Vazirmatn")
            && l.split_whitespace().filter(|c| *c == "yes").count() >= 2),
        "bundled Vazirmatn must be embedded and subset: {fonts}"
    );
    if let Ok(destination) = std::env::var("SARV_SALES_EXPORT_ARTIFACT_DIR") {
        let destination = Path::new(&destination);
        fs::create_dir_all(destination).unwrap();
        fs::copy(&p, destination.join("sales-reconciliation.pdf")).unwrap();
        fs::write(destination.join("sales-reconciliation-text.txt"), &text).unwrap();
        fs::write(destination.join("sales-reconciliation-fonts.txt"), fonts).unwrap();
        reconciliation_csv(
            &f.state,
            &f.company,
            "2026-03-31",
            MoneyUnit::Toman,
            DateDisplay::Jalali,
            DisplayDigits::Latin,
            &destination.join("sales-reconciliation.csv"),
        )
        .unwrap();
        for (name, id) in [
            ("sales-order", f.sale_order.as_str()),
            ("delivery", "DELIVERY-A"),
            ("invoice", "INVOICE-A"),
            ("return", "RETURN-A"),
            ("credit", "CREDIT-A"),
            ("credit-final", "CREDIT-FINAL"),
            ("invoice-final", "INVOICE-B"),
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
#[test]
fn authentic_approved_quote_provenance_exports_full_hash_and_line_mapping() {
    use sarv_quote::imports::{accept_catalog, catalog_csv};
    let f = fixture();
    let mut state = State::default();
    state.accounting = f.state.clone();
    let at = chrono::Utc::now().format("%Y-%m-%dT00:00:00Z").to_string();
    let catalog = catalog_csv(
        &format!(
            "code,name,measure,price,currency,source_at,aliases\nSKU-A,کالای الف,عدد,20,rial,{at},"
        ),
        "synthetic approved quotation",
    )
    .unwrap();
    accept_catalog(&mut state, catalog).unwrap();
    state.draft.customer = "مشتری الف".into();
    state.draft.lines = vec![sarv_quote::domain::OrderLine {
        query: "SKU-A".into(),
        quantity_milli: 10000,
        chosen_code: None,
    }];
    state.draft.discount_rial = 1;
    state.draft.quote_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    state.approve().unwrap();
    let quote = state.quotes[0].clone();
    let mut draft = state.sales_draft_from_quote(&quote.id, &f.company).unwrap();
    draft.lines[0].warehouse_id = "W1".into();
    let draft_id = draft.id.clone();
    state.save_sales_draft(draft, None).unwrap();
    let result = state
        .post_sales_from_draft(&draft_id, 1, "export-quote-conversion", "tester")
        .unwrap();
    state.validate().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("linked-order.csv");
    document_csv(
        &state.accounting,
        &f.company,
        &result.record_id,
        MoneyUnit::Rial,
        DateDisplay::Jalali,
        DisplayDigits::Latin,
        &path,
    )
    .unwrap();
    let (h, rows) = read_csv(&path);
    let row = rows
        .iter()
        .find(|r| cell(&h, r, "row_kind") == "order_line")
        .unwrap();
    assert_eq!(cell(&h, row, "quote_id"), quote.id);
    assert_eq!(cell(&h, row, "quote_number"), quote.number);
    assert_eq!(cell(&h, row, "quote_revision"), quote.version.to_string());
    assert_eq!(cell(&h, row, "quote_source_line_index"), "0");
    use sha2::{Digest, Sha256};
    assert_eq!(
        cell(&h, row, "quote_sha256"),
        format!("{:x}", Sha256::digest(serde_json::to_vec(&quote).unwrap()))
    );
    assert_eq!(cell(&h, row, "net_irr"), "199");
    assert_eq!(
        cell(&h, row, "price_reference"),
        format!("quotation:{}:line:0:catalog:{}", quote.id, quote.catalog_id)
    );
    let preview = document_preview(
        &state.accounting,
        &f.company,
        &result.record_id,
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        DisplayDigits::Persian,
    )
    .unwrap();
    assert!(
        preview
            .lines
            .iter()
            .any(|l| l.contains(&DisplayDigits::Persian.format(&quote.id)) && l.contains("SHA"))
    );
    if let Ok(destination) = std::env::var("SARV_SALES_EXPORT_ARTIFACT_DIR") {
        let destination = Path::new(&destination);
        fs::create_dir_all(destination).unwrap();
        fs::copy(&path, destination.join("quote-linked-order.csv")).unwrap();
        document_pdf(
            &state.accounting,
            &f.company,
            &result.record_id,
            MoneyUnit::Toman,
            DateDisplay::Jalali,
            DisplayDigits::Latin,
            &destination.join("quote-linked-order.pdf"),
        )
        .unwrap();
        fs::write(
            destination.join("quote-linked-approved-source.json"),
            serde_json::to_vec_pretty(&quote).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn zero_price_and_zero_cost_keep_operational_quantity_without_invented_financial_effect() {
    let mut f = fixture();
    let date = "2026-04-01";
    f.state
        .post_procurement_command(ProcurementCommand::Resources(RegisterResources {
            context: context(&f, "FREE-WH", date),
            parties: vec![],
            units: vec![],
            items: vec![],
            warehouses: vec![CoreWarehouse {
                id: "FREE-WH-ID".into(),
                company_id: f.company.clone(),
                name: "انبار نمونهٔ رایگان".into(),
            }],
        }))
        .unwrap();
    let po = f
        .state
        .post_procurement_command(ProcurementCommand::Order(PurchaseOrder {
            context: context(&f, "FREE-PO", date),
            supplier_id: "SU-A".into(),
            reference: "FREE-PO-REF".into(),
            document_date: date.into(),
            lines: vec![OrderLine {
                id: "FREE-PO-LINE".into(),
                item_id: "ITEM-A".into(),
                warehouse_id: "FREE-WH-ID".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: 1000,
                unit_price: Money::ZERO,
                line_discount: Money::ZERO,
                price_reference: "FREE-PRICE".into(),
                price_date: date.into(),
                override_reason: None,
            }],
            document_discount: Money::ZERO,
            supplied_total: Money::ZERO,
        }))
        .unwrap();
    f.state
        .post_procurement_command(ProcurementCommand::Receive(ReceiveOrder {
            context: context(&f, "FREE-RECEIVE", date),
            order_id: po.record_id,
            reference: "FREE-RECEIVE-REF".into(),
            document_date: date.into(),
            lines: vec![ReceiveLine {
                id: "FREE-RECEIVE-LINE".into(),
                order_line_id: "FREE-PO-LINE".into(),
                warehouse_id: "FREE-WH-ID".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: 1000,
            }],
            supplied_total: Money::ZERO,
        }))
        .unwrap();
    let order = f
        .state
        .post_sales_command(SalesCommand::Order(SalesOrder {
            context: context(&f, "FREE-SALE", date),
            customer_id: "CU-A".into(),
            reference: "FREE-SALE-REF".into(),
            document_date: date.into(),
            lines: vec![SalesOrderLine {
                id: "FREE-SALE-LINE".into(),
                item_id: "ITEM-A".into(),
                warehouse_id: "FREE-WH-ID".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: 1000,
                unit_price: Money::ZERO,
                line_discount: Money::ZERO,
                price_reference: "FREE-PRICE".into(),
                price_date: date.into(),
                override_reason: None,
            }],
            document_discount: Money::ZERO,
            supplied_total: Money::ZERO,
            quotation: None,
        }))
        .unwrap();
    let journal_count = f.state.journals.len();
    let fulfillment = f
        .state
        .post_sales_command(SalesCommand::Fulfill(FulfillOrder {
            context: context(&f, "FREE-FULFILL", date),
            order_id: order.record_id,
            delivery_id: "FREE-DELIVERY".into(),
            invoice_id: "FREE-INVOICE".into(),
            delivery_reference: "FREE-DELIVERY-REF".into(),
            invoice_reference: "FREE-INVOICE-REF".into(),
            document_date: date.into(),
            due_date: date.into(),
            lines: vec![FulfillLine {
                id: "FREE-ISSUE".into(),
                order_line_id: "FREE-SALE-LINE".into(),
                warehouse_id: "FREE-WH-ID".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: 1000,
            }],
            supplied_total: Money::ZERO,
        }))
        .unwrap();
    assert!(fulfillment.journal_id.is_none());
    assert_eq!(f.state.journals.len(), journal_count);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("free.csv");
    document_csv(
        &f.state,
        &f.company,
        "FREE-INVOICE",
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        DisplayDigits::Latin,
        &path,
    )
    .unwrap();
    let (h, rows) = read_csv(&path);
    assert_eq!(
        cell(&h, &rows[0], "included_states"),
        "recorded_nonfinancial_event"
    );
    assert_eq!(cell(&h, &rows[0], "financial_effect_id"), "");
    let line = rows
        .iter()
        .find(|r| cell(&h, r, "row_kind") == "fulfillment_line")
        .unwrap();
    assert_eq!(cell(&h, line, "base_quantity_thousandths"), "1000");
    assert_eq!(cell(&h, line, "net_irr"), "0");
    assert_eq!(cell(&h, line, "cost_irr"), "0");
    document_pdf(
        &f.state,
        &f.company,
        "FREE-INVOICE",
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        DisplayDigits::Latin,
        &dir.path().join("free.pdf"),
    )
    .unwrap();
}
#[test]
fn ordinary_sales_adjustments_have_journal_lineage_separate_from_typed_invoice_totals() {
    use sarv_quote::accounting::{JournalKind, JournalLineInput, PostCommand};
    let mut f = fixture();
    let setup = f
        .state
        .core
        .setups
        .iter()
        .find(|s| s.id == f.setup)
        .unwrap();
    let sales = setup.account_id(CoreRole::Sales).unwrap().to_string();
    let expense = setup
        .account_id(CoreRole::OperatingExpense)
        .unwrap()
        .to_string();
    let id = f
        .state
        .post(PostCommand {
            company_id: f.company.clone(),
            period_id: f.period.clone(),
            date: "2026-04-01".into(),
            source_id: "LEGACY-ADJUSTMENT-SOURCE".into(),
            source_revision: 1,
            event: "ordinary.adjustment.v1".into(),
            request_key: "ordinary-adjustment-key".into(),
            actor: "tester".into(),
            reason: "synthetic ordinary journal; no invoice".into(),
            kind: JournalKind::Manual,
            lines: vec![
                JournalLineInput {
                    account_id: expense,
                    debit: money(7),
                    credit: Money::ZERO,
                    description: "ordinary expense adjustment".into(),
                
                    dimension: String::new(),
},
                JournalLineInput {
                    account_id: sales,
                    debit: Money::ZERO,
                    credit: money(7),
                    description: "ordinary sales adjustment".into(),
                
                    dimension: String::new(),
},
            ],
        })
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("adjusted.csv");
    export_csv(&f, "2026-04-01", &path);
    let (h, rows) = read_csv(&path);
    let find = |kind| {
        rows.iter()
            .find(|r| cell(&h, r, "row_kind") == kind)
            .unwrap()
    };
    assert_eq!(cell(&h, find("sales_total"), "amount_irr"), "206");
    assert_eq!(cell(&h, find("typed_sales_total"), "amount_irr"), "199");
    let adjustment = find("ordinary_journal_adjustment");
    assert_eq!(cell(&h, adjustment, "event_id"), id);
    assert_eq!(
        cell(&h, adjustment, "financial_effect_kind"),
        "ordinary_journal"
    );
    assert_eq!(cell(&h, adjustment, "role"), "sales");
    assert_eq!(cell(&h, adjustment, "amount_irr"), "7");
    assert!(
        !f.state
            .sales
            .records
            .iter()
            .any(|r| r.journal_id.as_deref() == Some(&id))
    );
}
#[test]
fn divisible_partial_allocations_disclose_derived_gross_and_original_price() {
    let mut f = fixture();
    let date = "2026-04-01";
    f.state
        .post_procurement_command(ProcurementCommand::Resources(RegisterResources {
            context: context(&f, "DIV-MASTER", date),
            parties: vec![],
            units: vec![],
            warehouses: vec![],
            items: vec![CoreItem {
                id: "DIV-ITEM".into(),
                company_id: f.company.clone(),
                sku: "DIV-SKU".into(),
                name: "کالای تقسیم‌پذیر مصنوعی".into(),
                base_unit_id: "PIECE".into(),
                indivisible: false,
                packs: vec![],
            }],
        }))
        .unwrap();
    let po = f
        .state
        .post_procurement_command(ProcurementCommand::Order(PurchaseOrder {
            context: context(&f, "DIV-PO", date),
            supplier_id: "SU-A".into(),
            reference: "DIV-PO-REF".into(),
            document_date: date.into(),
            lines: vec![OrderLine {
                id: "DIV-PO-LINE".into(),
                item_id: "DIV-ITEM".into(),
                warehouse_id: "W1".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: 1500,
                unit_price: Money::ZERO,
                line_discount: Money::ZERO,
                price_reference: "DIV-FREE-SUPPLY".into(),
                price_date: date.into(),
                override_reason: None,
            }],
            document_discount: Money::ZERO,
            supplied_total: Money::ZERO,
        }))
        .unwrap();
    f.state
        .post_procurement_command(ProcurementCommand::Receive(ReceiveOrder {
            context: context(&f, "DIV-RECEIVE", date),
            order_id: po.record_id,
            reference: "DIV-RECEIVE-REF".into(),
            document_date: date.into(),
            lines: vec![ReceiveLine {
                id: "DIV-RECEIVE-LINE".into(),
                order_line_id: "DIV-PO-LINE".into(),
                warehouse_id: "W1".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: 1500,
            }],
            supplied_total: Money::ZERO,
        }))
        .unwrap();
    let order = f
        .state
        .post_sales_command(SalesCommand::Order(SalesOrder {
            context: context(&f, "DIV-SALE", date),
            customer_id: "CU-A".into(),
            reference: "DIV-SALE-REF".into(),
            document_date: date.into(),
            lines: vec![SalesOrderLine {
                id: "DIV-SALE-LINE".into(),
                item_id: "DIV-ITEM".into(),
                warehouse_id: "W1".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: 1500,
                unit_price: money(1),
                line_discount: Money::ZERO,
                price_reference: "DIV-ORIGINAL-PRICE".into(),
                price_date: date.into(),
                override_reason: None,
            }],
            document_discount: money(1),
            supplied_total: money(1),
            quotation: None,
        }))
        .unwrap();
    let SalesData::Order(original) = &f.state.sales_document(&order.record_id).unwrap().data else {
        panic!("order")
    };
    assert_eq!(
        (
            original.lines[0].gross,
            original.lines[0].document_discount,
            original.total
        ),
        (money(2), money(1), money(1))
    );
    let dir = tempfile::tempdir().unwrap();
    for (index, gross, discount, net) in [(0, 0, 0, 0), (1, 2, 1, 1), (2, 0, 0, 0)] {
        let invoice = format!("DIV-INVOICE-{index}");
        let result = f
            .state
            .post_sales_command(SalesCommand::Fulfill(FulfillOrder {
                context: context(&f, &format!("DIV-FULFILL-{index}"), date),
                order_id: order.record_id.clone(),
                delivery_id: format!("DIV-DELIVERY-{index}"),
                invoice_id: invoice.clone(),
                delivery_reference: format!("DIV-DELIVERY-REF-{index}"),
                invoice_reference: format!("DIV-INVOICE-REF-{index}"),
                document_date: date.into(),
                due_date: date.into(),
                lines: vec![FulfillLine {
                    id: format!("DIV-ISSUE-{index}"),
                    order_line_id: "DIV-SALE-LINE".into(),
                    warehouse_id: "W1".into(),
                    unit_id: "PIECE".into(),
                    quantity_thousandths: 500,
                }],
                supplied_total: money(net),
            }))
            .unwrap();
        let SalesData::Fulfill(posted) = &f.state.sales_document(&result.record_id).unwrap().data
        else {
            panic!("fulfill")
        };
        assert_eq!(
            (
                posted.lines[0].gross,
                posted.lines[0].document_discount,
                posted.total
            ),
            (money(gross), money(discount), money(net))
        );
        let path = dir.path().join(format!("partial-{index}.csv"));
        document_csv(
            &f.state,
            &f.company,
            &invoice,
            MoneyUnit::Toman,
            DateDisplay::Jalali,
            DisplayDigits::Latin,
            &path,
        )
        .unwrap();
        let (h, rows) = read_csv(&path);
        let row = rows
            .iter()
            .find(|r| cell(&h, r, "row_kind") == "fulfillment_line")
            .unwrap();
        assert_eq!(cell(&h, row, "gross_irr"), gross.to_string());
        assert_eq!(cell(&h, row, "document_discount_irr"), discount.to_string());
        assert_eq!(cell(&h, row, "net_irr"), net.to_string());
        assert_eq!(cell(&h, row, "unit_price_irr"), "1");
        assert_eq!(cell(&h, row, "price_unit_id"), "PIECE");
        assert_eq!(
            cell(&h, row, "allocation_method"),
            "cumulative_original_net_and_discounts_half_up_final_residual_gross_sum"
        );
        let preview = document_preview(
            &f.state,
            &f.company,
            &invoice,
            MoneyUnit::Rial,
            DateDisplay::Iso,
            DisplayDigits::Latin,
        )
        .unwrap();
        assert!(
            preview
                .lines
                .iter()
                .any(|l| l.contains("ناخالص تخصیص‌یافته از سفارش"))
        );
        assert!(preview.lines.iter().any(|l| l.contains("قیمت اصلی سفارش")));
        assert!(
            preview
                .lines
                .iter()
                .any(|l| l.contains("اختلاف الزاماً یک ریال نیست"))
        );
        if let Ok(destination) = std::env::var("SARV_SALES_EXPORT_ARTIFACT_DIR") {
            let destination = Path::new(&destination);
            fs::create_dir_all(destination).unwrap();
            fs::copy(
                &path,
                destination.join(format!("divisible-partial-{index}.csv")),
            )
            .unwrap();
            document_pdf(
                &f.state,
                &f.company,
                &invoice,
                MoneyUnit::Toman,
                DateDisplay::Jalali,
                DisplayDigits::Latin,
                &destination.join(format!("divisible-partial-{index}.pdf")),
            )
            .unwrap();
        }
    }
    assert_eq!(
        f.state.sales_order_remaining(&order.record_id).unwrap()[0].remaining_quantity_thousandths,
        0
    );
}

#[test]
fn printable_sales_invoice_is_a4_exact_and_embeds_persian_fonts() {
    let f = fixture();
    let mut state = State::default();
    state.accounting = f.state.clone();
    state.settings.date_display = DateDisplay::Jalali;
    state.settings.display_digits = DisplayDigits::Persian;
    state.business_profiles.insert(f.company.clone(), sarv_quote::domain::BusinessProfile {
        phone: "021-12345678".into(), address: "تهران، خیابان سرو".into(), registration_id: "REG-2026".into(), tax_id: "TAX-123".into(), invoice_prefix: "INV".into(), terms: "پرداخت در زمان تحویل کالا".into(), footer: "سپاس از خرید شما".into(), bank_details: String::new(),
    });
    let out = Path::new("/tmp/sarv-printable-pdf");
    fs::create_dir_all(out).unwrap();
    for (language, id) in [("fa", &f.fulfillment), ("en", &f.sale_order), ("fa-credit", &f.returned)] {
        let pdf = out.join(format!("{language}.pdf"));
        let _ = fs::remove_file(&pdf);
        printable_document_pdf(&state, &f.company, id, if language.starts_with("fa") { "fa" } else { "en" }, &pdf).unwrap();
        let info = Command::new("pdfinfo").arg(&pdf).output().unwrap();
        let info = String::from_utf8_lossy(&info.stdout);
        assert!(info.contains("595 x 842"), "{info}");
        let fonts = Command::new("pdffonts").arg(&pdf).output().unwrap();
        assert!(String::from_utf8_lossy(&fonts.stdout).contains("Vazirmatn"));
        let text = Command::new("pdftotext").args(["-layout"]).arg(&pdf).arg("-").output().unwrap();
        let text = String::from_utf8_lossy(&text.stdout);
        assert!(text.contains("REG-2026"), "{text}");
        assert!(text.contains("021-12345678"), "{text}");
        assert!(!text.contains("source_line_id"));
        assert!(printable_document_pdf(&state, "foreign-company", id, "en", &out.join("foreign.pdf")).is_err());
    }
    state.business_profiles.get_mut(&f.company).unwrap().terms = "Payment terms line\n".repeat(80) + "TERMS-END";
    let pdf = out.join("multipage.pdf");
    let _ = fs::remove_file(&pdf);
    printable_document_pdf(&state, &f.company, &f.fulfillment, "en", &pdf).unwrap();
    let info = Command::new("pdfinfo").arg(&pdf).output().unwrap();
    let info = String::from_utf8_lossy(&info.stdout);
    let pages: usize = info.lines().find(|line| line.starts_with("Pages:")).unwrap().split_whitespace().last().unwrap().parse().unwrap();
    assert!(pages > 1);
    let text = Command::new("pdftotext").arg("-bbox").arg(&pdf).arg("-").output().unwrap();
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(text.contains("TERMS-END"));
    for segment in text.split("yMax=\"").skip(1) {
        let y: f64 = segment.split('"').next().unwrap().parse().unwrap();
        assert!(y <= 842.0, "Text overflows A4 page: {y}");
    }
}
