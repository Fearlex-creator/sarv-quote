#![cfg(feature = "pdf-export")]
use sarv_quote::{
    accounting::{AccountingState, Money, MoneyUnit},
    core::*,
    core_import,
    domain::{DateDisplay, DisplayDigits},
    procurement::*,
    procurement_export::*,
    sales::*,
};
use std::{fs, path::Path, process::Command};
fn money(n: i64) -> Money {
    Money::from_rials(n)
}
#[allow(dead_code)]
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
    fixture_with_sku("SKU-A")
}
fn fixture_with_sku(sku: &str) -> Fixture {
    let mut state = AccountingState::default();
    let c = state
        .setup_core_company(NewCoreCompany {
            name: "شرکت مصنوعی TEST-PURCHASE-RETURNS".into(),
            fiscal_name: "۱۴۰۵".into(),
            start_date: "2026-03-21".into(),
            end_date: "2027-03-20".into(),
            timezone: CompanyTimezone::AsiaTehran,
            chart: default_chart(),
            actor: "آزمایشگر".into(),
        })
        .unwrap();
    let input = include_str!("../../../docs/assets/accounting-m3/opening.csv")
        .replace("F2-core-v1", &c.company_id)
        .replace("SKU-A", sku);
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
fn returned(
    f: &mut Fixture,
    id: &str,
    receipt_line: &str,
    bill_line: Option<&str>,
    quantity: i64,
    value: i64,
    date: &str,
) -> String {
    let cmd = ProcurementCommand::Return(ReturnPurchase {
        context: context(f, id, date),
        supplier_id: "SU-A".into(),
        return_id: format!("RETURN-{id}"),
        return_reference: format!("=RETURN-{id}()"),
        credit_id: bill_line.map(|_| format!("CREDIT-{id}")),
        credit_reference: bill_line.map(|_| format!("=CREDIT-{id}()")),
        document_date: "2026-03-31".into(),
        lines: vec![PurchaseReturnLine {
            id: format!("LINE-{id}"),
            receipt_line_id: receipt_line.into(),
            bill_line_id: bill_line.map(str::to_string),
            quantity_thousandths: quantity,
            supplied_value: money(value),
        }],
        supplied_total: money(value),
    });
    f.state.post_procurement_command(cmd).unwrap().record_id
}
fn document(f: &Fixture, id: &str, path: &Path) {
    document_csv(
        &f.state,
        &f.company,
        id,
        MoneyUnit::Toman,
        DateDisplay::Jalali,
        DisplayDigits::Latin,
        path,
    )
    .unwrap();
}
fn row<'a>(
    headers: &csv::StringRecord,
    rows: &'a [csv::StringRecord],
    kind: &str,
) -> &'a csv::StringRecord {
    rows.iter()
        .find(|r| cell(headers, r, "row_kind") == kind)
        .unwrap()
}
#[test]
fn persian_return_preview_preserves_numeric_and_mixed_source_identities() {
    let mut f = fixture_with_sku("123456");
    let posted = f
        .state
        .post_procurement_command(ProcurementCommand::Return(ReturnPurchase {
            context: context(&f, "SOURCE-123", "2026-04-01"),
            supplier_id: "SU-A".into(),
            return_id: "RETURN-123".into(),
            return_reference: "12345".into(),
            credit_id: Some("CREDIT-456".into()),
            credit_reference: Some("67890".into()),
            document_date: "2026-04-01".into(),
            lines: vec![PurchaseReturnLine {
                id: "LINE-789".into(),
                receipt_line_id: "R-B-LINE".into(),
                bill_line_id: Some("BILL-B-LINE-B".into()),
                quantity_thousandths: 3000,
                supplied_value: money(45),
            }],
            supplied_total: money(45),
        }))
        .unwrap();
    let unchanged = f.state.clone();
    for id in [&posted.record_id, &"CREDIT-456".into()] {
        let preview = document_preview(
            &f.state,
            &f.company,
            id,
            MoneyUnit::Rial,
            DateDisplay::Jalali,
            DisplayDigits::Persian,
        )
        .unwrap();
        let text = preview.lines.join("\n");
        for source in [
            "RETURN-123",
            "CREDIT-456",
            "67890",
            "12345",
            "BILL-B-LINE-B",
            "R-B-LINE",
            "123456",
        ] {
            assert!(
                text.contains(source),
                "source identity was transformed: {source}"
            );
        }
        let record = f.state.procurement_document(&posted.record_id).unwrap();
        assert!(text.contains(record.journal_id.as_deref().unwrap()));
        assert!(text.contains("۴۵"));
        assert!(text.contains("۲۰۲۶-۰۴-۰۱"));
        assert!(text.contains("۳.۰۰۰"));
    }
    if let Ok(destination) = std::env::var("SARV_PURCHASE_RETURN_EXPORT_ARTIFACT_DIR") {
        let destination = Path::new(&destination);
        fs::create_dir_all(destination).unwrap();
        let path = destination.join("persian-identities-reconciliation.pdf");
        reconciliation_pdf(
            &f.state,
            &f.company,
            "2026-04-01",
            MoneyUnit::Rial,
            DateDisplay::Jalali,
            DisplayDigits::Persian,
            &path,
        )
        .unwrap();
        let output = Command::new("pdftotext")
            .arg("-layout")
            .arg(&path)
            .arg("-")
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        fs::write(
            destination.join("persian-identities-reconciliation.txt"),
            &text,
        )
        .unwrap();
        for code in ["RETURN-123", "CREDIT-456", "123456"] {
            assert!(text.contains(code), "reconciliation lost code {code}");
        }
        assert!(text.contains("۴۵"));
        assert!(text.contains("۲۰۲۶-۰۴-۰۱"));
    }
    assert_eq!(f.state, unchanged);
}
#[test]
fn billed_return_credit_identities_exact_cost_lineage_remaining_ap_and_historical_snapshot() {
    let mut f = fixture();
    let initial = f
        .state
        .procurement_reconcile(&f.company, "2026-03-31")
        .unwrap();
    for (role, expected) in [
        (CoreRole::Inventory, 422),
        (CoreRole::Payables, 329),
        (CoreRole::Grni, 50),
    ] {
        assert_eq!(
            initial
                .controls
                .iter()
                .find(|c| c.role == role)
                .unwrap()
                .ledger,
            money(expected)
        );
    }
    assert_eq!(initial.total_debit, money(1630));
    let id = returned(
        &mut f,
        "PART",
        "R-B-LINE",
        Some("BILL-B-LINE-B"),
        3000,
        45,
        "2026-04-01",
    );
    let state_after = f.state.clone();
    let dir = tempfile::tempdir().unwrap();
    let before = dir.path().join("before.csv");
    document(&f, &id, &before);
    for (doc_id, kind, line_kind) in [
        (&id, "purchase_return", "purchase_return_line"),
        (
            &"RETURN-PART".into(),
            "purchase_return",
            "purchase_return_line",
        ),
        (
            &"CREDIT-PART".into(),
            "supplier_credit",
            "supplier_credit_line",
        ),
    ] {
        let path = dir
            .path()
            .join(format!("{kind}-{}.csv", doc_id.replace('/', "_")));
        document(&f, doc_id, &path);
        let (h, rows) = read_csv(&path);
        let line = row(&h, &rows, line_kind);
        for (key, expected) in [
            ("document_kind", kind),
            ("amount_irr", "45"),
            ("display_amount", "4.5"),
            ("base_currency", "IRR"),
            ("display_unit", "toman"),
            ("display_calendar", "jalali"),
            ("display_digits", "latin"),
            ("return_id", "RETURN-PART"),
            ("credit_id", "CREDIT-PART"),
            ("return_state", "returned"),
            ("credit_state", "posted"),
            ("receipt_line_id", "R-B-LINE"),
            ("bill_line_id", "BILL-B-LINE-B"),
            ("bill_id", f.bill_b.as_str()),
            ("receipt_id", f.receipt_b.as_str()),
            ("warehouse_id", "W1"),
            ("original_quantity_thousandths", "7000"),
            ("original_value_irr", "105"),
            ("remaining_quantity_thousandths", "4000"),
            ("remaining_value_irr", "60"),
            ("remaining_payable_irr", "105"),
            ("financial_date", "2026-04-01"),
            ("document_date", "2026-03-31"),
            ("source_reference", "'=RETURN-PART()"),
        ] {
            if kind == "supplier_credit" && key == "source_reference" {
                assert_eq!(cell(&h, line, key), "'=CREDIT-PART()");
            } else {
                assert_eq!(cell(&h, line, key), expected, "{key}");
            }
        }
        assert_eq!(
            rows.iter()
                .filter(|r| cell(&h, r, "row_kind") == "stock_movement")
                .count(),
            usize::from(kind == "purchase_return")
        );
        let snapshot: serde_json::Value = serde_json::from_str(cell(
            &h,
            row(&h, &rows, "complete_snapshot"),
            "snapshot_json",
        ))
        .unwrap();
        assert_eq!(snapshot["related_sources"].as_array().unwrap().len(), 3);
    }
    assert_eq!(f.state, state_after, "exports must be read-only");
    let final_id = returned(
        &mut f,
        "PURCHASE-FINAL",
        "R-B-LINE",
        Some("BILL-B-LINE-B"),
        4000,
        60,
        "2026-04-02",
    );
    let path = dir.path().join("final.csv");
    document(&f, &final_id, &path);
    let (h, rows) = read_csv(&path);
    let line = row(&h, &rows, "purchase_return_line");
    assert_eq!(cell(&h, line, "amount_irr"), "60");
    assert_eq!(cell(&h, line, "remaining_value_irr"), "0");
    assert_eq!(cell(&h, line, "remaining_payable_irr"), "45");
    for (date, inventory, ap, tb) in [
        ("2026-04-01", 377, 284, 1585),
        ("2026-04-02", 317, 224, 1525),
    ] {
        let path = dir.path().join(format!("reconcile-{date}.csv"));
        reconciliation_csv(
            &f.state,
            &f.company,
            date,
            MoneyUnit::Rial,
            DateDisplay::Iso,
            DisplayDigits::Latin,
            &path,
        )
        .unwrap();
        let (h, rows) = read_csv(&path);
        for (role, expected) in [("inventory", inventory), ("payables", ap), ("grni", 50)] {
            let control = rows
                .iter()
                .find(|r| cell(&h, r, "row_kind") == "control" && cell(&h, r, "role") == role)
                .unwrap();
            assert_eq!(cell(&h, control, "ledger_irr"), expected.to_string());
            assert_eq!(cell(&h, control, "difference_irr"), "0");
        }
        assert_eq!(
            cell(&h, row(&h, &rows, "trial_total_debit"), "amount_irr"),
            tb.to_string()
        );
    }
    // Later allocations and unrelated appended catalogue resources cannot rewrite issued exports.
    f.state
        .post_procurement_command(ProcurementCommand::Resources(RegisterResources {
            context: context(&f, "LATER-CATALOG", "2026-04-03"),
            parties: vec![],
            units: vec![],
            warehouses: vec![],
            items: vec![CoreItem {
                id: "NEW-ITEM".into(),
                company_id: f.company.clone(),
                sku: "NEW-SKU".into(),
                name: "کالای بعدی".into(),
                base_unit_id: "PIECE".into(),
                packs: vec![],
                indivisible: true,
            }],
        }))
        .unwrap();
    let after = dir.path().join("after.csv");
    document(&f, &id, &after);
    assert_eq!(fs::read(&before).unwrap(), fs::read(&after).unwrap());
    assert!(
        document_preview(
            &f.state,
            "foreign",
            "CREDIT-PART",
            MoneyUnit::Rial,
            DateDisplay::Iso,
            DisplayDigits::Latin
        )
        .is_err()
    );
}
fn tiny_receipt(
    f: &mut Fixture,
    id: &str,
    count: usize,
    quantity: i64,
    price: i64,
    discount: i64,
) -> Vec<String> {
    let date = f
        .state
        .procurement
        .records
        .iter()
        .map(|r| r.command.context().date.as_str())
        .max()
        .unwrap_or("2026-04-01")
        .max("2026-04-01")
        .to_string();
    let total = (quantity * price / 1000 - discount) * count as i64;
    let order = ProcurementCommand::Order(PurchaseOrder {
        context: context(f, &format!("{id}-PO"), &date),
        supplier_id: "SU-A".into(),
        reference: format!("{id}-PO"),
        document_date: "2026-03-31".into(),
        lines: (0..count)
            .map(|n| OrderLine {
                id: format!("{id}-PO-{n}"),
                item_id: "ITEM-A".into(),
                warehouse_id: "W1".into(),
                unit_id: "PIECE".into(),
                quantity_thousandths: quantity,
                unit_price: money(price),
                line_discount: money(discount),
                price_reference: "ORIGINAL-SNAPSHOT".into(),
                price_date: "2026-03-31".into(),
                override_reason: None,
            })
            .collect(),
        document_discount: Money::ZERO,
        supplied_total: money(total),
    });
    let order_id = f.state.post_procurement_command(order).unwrap().record_id;
    let lines = (0..count)
        .map(|n| format!("{id}-R-{n}"))
        .collect::<Vec<_>>();
    f.state
        .post_procurement_command(ProcurementCommand::Receive(ReceiveOrder {
            context: context(f, &format!("{id}-R"), &date),
            order_id,
            reference: format!("{id}-RECEIPT"),
            document_date: "2026-03-31".into(),
            lines: lines
                .iter()
                .enumerate()
                .map(|(n, id)| ReceiveLine {
                    id: id.clone(),
                    order_line_id: format!(
                        "{id_prefix}-PO-{n}",
                        id_prefix = id.split("-R-").next().unwrap()
                    ),
                    warehouse_id: "W1".into(),
                    unit_id: "PIECE".into(),
                    quantity_thousandths: quantity,
                })
                .collect(),
            supplied_total: money(total),
        }))
        .unwrap();
    lines
}
fn bill(f: &mut Fixture, id: &str, receipt: &str, quantity: i64, value: i64, date: &str) {
    f.state
        .post_procurement_command(ProcurementCommand::Bill(MatchBill {
            context: context(f, id, date),
            supplier_id: "SU-A".into(),
            reference: format!("BILL-{id}"),
            document_date: "2026-03-31".into(),
            due_date: "2026-04-10".into(),
            lines: vec![BillLine {
                id: format!("{id}-LINE"),
                receipt_line_id: receipt.into(),
                quantity_thousandths: quantity,
                supplied_value: money(value),
            }],
            supplied_total: money(value),
        }))
        .unwrap();
}
fn artifact(f: &Fixture, id: &str, name: &str) {
    if let Ok(destination) = std::env::var("SARV_PURCHASE_RETURN_EXPORT_ARTIFACT_DIR") {
        let destination = Path::new(&destination);
        fs::create_dir_all(destination).unwrap();
        document(f, id, &destination.join(format!("{name}.csv")));
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
        let pdf = destination.join(format!("{name}.pdf"));
        let text = Command::new("pdftotext")
            .arg(&pdf)
            .arg("-")
            .output()
            .unwrap();
        assert!(text.status.success());
        fs::write(destination.join(format!("{name}-text.txt")), text.stdout).unwrap();
        let fonts = Command::new("pdffonts").arg(&pdf).output().unwrap();
        assert!(fonts.status.success());
        fs::write(destination.join(format!("{name}-fonts.txt")), fonts.stdout).unwrap();
        assert!(
            Command::new("pdftoppm")
                .args(["-png", "-r", "90"])
                .arg(&pdf)
                .arg(destination.join(format!("{name}-page")))
                .status()
                .unwrap()
                .success()
        );
    }
}
#[test]
fn unbilled_interleaving_tiny_zero_credit_and_final_residual_remain_operational_and_exact() {
    let mut f = fixture();
    let receipt = tiny_receipt(&mut f, "TINY", 1, 3000, 1, 2).remove(0);
    bill(&mut f, "ZERO-FIRST", &receipt, 1000, 0, "2026-04-02");
    let unbilled = returned(&mut f, "UNBILLED", &receipt, None, 1000, 1, "2026-04-03");
    bill(&mut f, "ZERO-FINAL", &receipt, 1000, 0, "2026-04-04");
    let zero = returned(
        &mut f,
        "ZERO-CREDIT",
        &receipt,
        Some("ZERO-FIRST-LINE"),
        1000,
        0,
        "2026-04-05",
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unbilled.csv");
    document(&f, &unbilled, &path);
    let (h, rows) = read_csv(&path);
    let line = row(&h, &rows, "purchase_return_line");
    for (key, value) in [
        ("amount_irr", "1"),
        ("display_amount", "0.1"),
        ("original_quantity_thousandths", "3000"),
        ("original_value_irr", "1"),
        ("remaining_quantity_thousandths", "1000"),
        ("remaining_value_irr", "0"),
        ("credit_id", ""),
        ("credit_state", ""),
        ("bill_line_id", ""),
    ] {
        assert_eq!(cell(&h, line, key), value, "{key}");
    }
    assert!(
        document_preview(
            &f.state,
            &f.company,
            "CREDIT-UNBILLED",
            MoneyUnit::Rial,
            DateDisplay::Iso,
            DisplayDigits::Latin
        )
        .is_err()
    );
    let path = dir.path().join("zero.csv");
    document(&f, "CREDIT-ZERO-CREDIT", &path);
    let (h, rows) = read_csv(&path);
    let line = row(&h, &rows, "supplier_credit_line");
    for (key, value) in [
        ("amount_irr", "0"),
        ("quantity_thousandths", "1000"),
        ("original_value_irr", "0"),
        ("credit_id", "CREDIT-ZERO-CREDIT"),
        ("credit_state", "posted"),
        ("remaining_value_irr", "0"),
        ("remaining_quantity_thousandths", "0"),
    ] {
        assert_eq!(cell(&h, line, key), value, "{key}");
    }
    let record = f.state.procurement_document(&zero).unwrap();
    assert!(record.journal_id.is_none());
    assert_eq!(record.movements.len(), 1);
    let snapshot: serde_json::Value = serde_json::from_str(cell(
        &h,
        row(&h, &rows, "complete_snapshot"),
        "snapshot_json",
    ))
    .unwrap();
    assert!(snapshot["document"]["journal_id"].is_null());
    artifact(&f, &unbilled, "unbilled-return-one-rial");
    artifact(&f, "CREDIT-ZERO-CREDIT", "zero-supplier-credit");
    // 3 units / 2 IRR reverses 1, 0, then the final exact 1 IRR residual.
    let residual = tiny_receipt(&mut f, "RESIDUAL", 1, 3000, 1, 1).remove(0);
    for (id, value, date, remaining) in [
        ("RES-1", 1, "2026-04-05", 1),
        ("RES-2", 0, "2026-04-06", 1),
        ("RES-3", 1, "2026-04-07", 0),
    ] {
        let record = returned(&mut f, id, &residual, None, 1000, value, date);
        let path = dir.path().join(format!("{id}.csv"));
        document(&f, &record, &path);
        let (h, rows) = read_csv(&path);
        let line = row(&h, &rows, "purchase_return_line");
        assert_eq!(cell(&h, line, "amount_irr"), value.to_string());
        assert_eq!(cell(&h, line, "remaining_value_irr"), remaining.to_string());
    }
}
#[test]
fn physical_and_credit_pdf_layouts_repeat_headings_and_embed_local_persian_font_on_every_page() {
    let mut f = fixture();
    let id = returned(
        &mut f,
        "PDF",
        "R-B-LINE",
        Some("BILL-B-LINE-B"),
        3000,
        45,
        "2026-04-01",
    );
    artifact(&f, &id, "billed-physical-return");
    artifact(&f, "CREDIT-PDF", "billed-supplier-credit");
    let lines = tiny_receipt(&mut f, "LONG", 12, 1000, 15, 0);
    let long_id = f
        .state
        .post_procurement_command(ProcurementCommand::Return(ReturnPurchase {
            context: context(&f, "LONG-RETURN", "2026-04-02"),
            supplier_id: "SU-A".into(),
            return_id: "LONG-PHYSICAL-RETURN".into(),
            return_reference: "LONG-RETURN-REFERENCE".into(),
            credit_id: None,
            credit_reference: None,
            document_date: "2026-03-31".into(),
            lines: lines
                .iter()
                .enumerate()
                .map(|(n, receipt)| PurchaseReturnLine {
                    id: format!("LONG-RETURN-LINE-{n}"),
                    receipt_line_id: receipt.clone(),
                    bill_line_id: None,
                    quantity_thousandths: 1000,
                    supplied_value: money(15),
                })
                .collect(),
            supplied_total: money(180),
        }))
        .unwrap()
        .record_id;
    artifact(&f, &long_id, "long-physical-return");
    let dir = tempfile::tempdir().unwrap();
    for (doc, kind) in [
        (id.as_str(), "physical"),
        ("CREDIT-PDF", "credit"),
        (long_id.as_str(), "long"),
    ] {
        let path = dir.path().join(format!("{kind}.pdf"));
        document_pdf(
            &f.state,
            &f.company,
            doc,
            MoneyUnit::Toman,
            DateDisplay::Jalali,
            DisplayDigits::Latin,
            &path,
        )
        .unwrap();
        let output = Command::new("pdftotext")
            .arg(&path)
            .arg("-")
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        let pages = text
            .split('\u{c}')
            .filter(|p| !p.trim().is_empty())
            .collect::<Vec<_>>();
        assert!(!pages.is_empty());
        if kind == "long" {
            assert!(pages.len() >= 4);
            for n in 0..12 {
                assert!(text.contains(&format!("LONG-RETURN-LINE-{n}")));
            }
        }
        for page in &pages {
            for token in [
                "TEST-PURCHASE-RETURNS",
                "IRR",
                "toman",
                "core-policy-v1",
                "2026-04",
            ] {
                assert!(page.contains(token), "missing repeated {token} in {kind}");
            }
        }
        if kind != "long" {
            for token in [
                "RETURN-PDF",
                "CREDIT-PDF",
                "R-B-LINE",
                "BILL-B-LINE-B",
                "4.5",
                "2026-03-31",
            ] {
                assert!(text.contains(token), "missing {token}");
            }
        }
        let fonts = Command::new("pdffonts").arg(&path).output().unwrap();
        assert!(fonts.status.success());
        let fonts = String::from_utf8(fonts.stdout).unwrap();
        assert!(fonts.contains("Vazirmatn"), "{fonts}");
        for font in fonts.lines().skip(2) {
            let values = font.split_whitespace().collect::<Vec<_>>();
            assert!(values.contains(&"yes"), "unembedded {font}");
        }
        let raster = dir.path().join(format!("{kind}-page"));
        let status = Command::new("pdftoppm")
            .args(["-png", "-r", "72"])
            .arg(&path)
            .arg(&raster)
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(
            fs::read_dir(dir.path())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| e
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&format!("{kind}-page-")))
                .count(),
            pages.len()
        );
    }
}
