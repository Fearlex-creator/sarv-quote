use sarv_quote::{
    accounting::{AccountingState, Money},
    core::CoreRole,
    procurement::*,
    sales::*,
};
fn irr(v: i64) -> Money {
    Money::from_rials(v)
}
fn fixture() -> AccountingState {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/assets/accounting-m3/restore/core-v4.json"
    ))
    .unwrap();
    let s: AccountingState = serde_json::from_value(v["state"]["accounting"].clone()).unwrap();
    s.validate().unwrap();
    s
}
fn context(s: &AccountingState, source: &str, date: &str) -> ProcurementContext {
    let o = &s.core.openings[0].command;
    ProcurementContext {
        company_id: o.company_id.clone(),
        period_id: o.period_id.clone(),
        setup_id: o.setup_id.clone(),
        source_id: source.into(),
        source_revision: 1,
        date: date.into(),
        request_key: format!("key-{source}"),
        actor: "procurement-fixture".into(),
        reason: "independent artificial amounts; no live books".into(),
    }
}
fn ids(s: &AccountingState) -> (String, String, String, String, String) {
    let o = &s.core.openings[0].command;
    (
        o.items[0].id.clone(),
        o.items[0].base_unit_id.clone(),
        o.stock
            .iter()
            .find(|x| x.value == irr(240))
            .unwrap()
            .warehouse_id
            .clone(),
        o.stock
            .iter()
            .find(|x| x.value == irr(60))
            .unwrap()
            .warehouse_id
            .clone(),
        o.grni[0].party_id.clone(),
    )
}
fn order(s: &AccountingState) -> ProcurementCommand {
    let (item, unit, w, _, party) = ids(s);
    ProcurementCommand::Order(PurchaseOrder {
        context: context(s, "PO", "2026-03-21"),
        supplier_id: party,
        reference: "PO-12".into(),
        document_date: "2026-03-21".into(),
        lines: vec![OrderLine {
            id: "PO-L".into(),
            item_id: item,
            warehouse_id: w,
            unit_id: unit,
            quantity_thousandths: 12000,
            unit_price: irr(15),
            line_discount: Money::ZERO,
            price_reference: "independent piece price".into(),
            price_date: "2026-03-21".into(),
            override_reason: None,
        }],
        document_discount: Money::ZERO,
        supplied_total: irr(180),
    })
}
fn receive(
    s: &AccountingState,
    order: &str,
    source: &str,
    date: &str,
    q: i64,
    v: i64,
) -> ProcurementCommand {
    let (_, unit, w, _, _) = ids(s);
    ProcurementCommand::Receive(ReceiveOrder {
        context: context(s, source, date),
        order_id: order.into(),
        reference: source.into(),
        document_date: date.into(),
        lines: vec![ReceiveLine {
            id: format!("{source}-L"),
            order_line_id: "PO-L".into(),
            warehouse_id: w,
            unit_id: unit,
            quantity_thousandths: q,
        }],
        supplied_total: irr(v),
    })
}
fn bill(
    s: &AccountingState,
    source: &str,
    date: &str,
    lines: &[(&str, i64, i64)],
) -> ProcurementCommand {
    let (_, _, _, _, party) = ids(s);
    ProcurementCommand::Bill(MatchBill {
        context: context(s, source, date),
        supplier_id: party,
        reference: source.into(),
        document_date: date.into(),
        due_date: "2026-04-30".into(),
        lines: lines
            .iter()
            .enumerate()
            .map(|(i, (id, q, v))| BillLine {
                id: format!("{source}-{i}"),
                receipt_line_id: (*id).into(),
                quantity_thousandths: *q,
                supplied_value: irr(*v),
            })
            .collect(),
        supplied_total: irr(lines.iter().map(|x| x.2).sum()),
    })
}
fn transfer(s: &AccountingState, source: &str, date: &str, q: i64) -> ProcurementCommand {
    let (item, unit, w1, w2, _) = ids(s);
    ProcurementCommand::Transfer(TransferStock {
        context: context(s, source, date),
        item_id: item,
        source_warehouse_id: w1,
        target_warehouse_id: w2,
        unit_id: unit,
        quantity_thousandths: q,
    })
}
fn ret(
    s: &AccountingState,
    source: &str,
    date: &str,
    receipt: &str,
    bill: Option<&str>,
    q: i64,
    value: i64,
) -> ProcurementCommand {
    ProcurementCommand::Return(ReturnPurchase {
        context: context(s, source, date),
        supplier_id: ids(s).4,
        return_id: format!("{source}-return"),
        return_reference: format!("{source}-return-ref"),
        credit_id: bill.map(|_| format!("{source}-credit")),
        credit_reference: bill.map(|_| format!("{source}-credit-ref")),
        document_date: date.into(),
        lines: vec![PurchaseReturnLine {
            id: format!("{source}-line"),
            receipt_line_id: receipt.into(),
            bill_line_id: bill.map(str::to_owned),
            quantity_thousandths: q,
            supplied_value: irr(value),
        }],
        supplied_total: irr(value),
    })
}
fn received() -> (AccountingState, String) {
    let mut s = fixture();
    let command = order(&s);
    let po = s.post_procurement_command(command).unwrap().record_id;
    let command = receive(&s, &po, "R", "2026-03-22", 12000, 180);
    s.post_procurement_command(command).unwrap();
    (s, po)
}
fn assert_report(s: &AccountingState, date: &str, inv: i64, ap: i64, grni: i64) {
    let r = s
        .procurement_reconcile(&s.core.openings[0].command.company_id, date)
        .unwrap();
    assert!(r.reconciled);
    for (role, value) in [
        (CoreRole::Inventory, inv),
        (CoreRole::Payables, ap),
        (CoreRole::Grni, grni),
    ] {
        let control = r.controls.iter().find(|c| c.role == role).unwrap();
        assert_eq!(
            (control.ledger, control.detail, control.difference),
            (irr(value), irr(value), irr(0))
        );
    }
}
#[test]
fn unbilled_return_then_bill_and_linked_partial_final_credits_preserve_historical_controls_and_po()
{
    let (mut s, po) = received();
    let original = s.procurement.records.clone();
    let command = ret(&s, "U", "2026-03-23", "R-L", None, 2000, 30);
    s.post_procurement_command(command).unwrap();
    let command = bill(&s, "B", "2026-03-24", &[("R-L", 10000, 150)]);
    let bid = s.post_procurement_command(command).unwrap().record_id;
    for (source, date, q, v) in [
        ("C1", "2026-03-25", 2000, 30),
        ("C2", "2026-03-26", 8000, 120),
    ] {
        let command = ret(&s, source, date, "R-L", Some("B-0"), q, v);
        let result = s.post_procurement_command(command).unwrap();
        let record = s.procurement_document(&result.record_id).unwrap();
        let ProcurementData::Return(posted) = &record.data else {
            panic!()
        };
        assert_eq!(posted.bill_id.as_ref(), Some(&bid));
        assert_eq!(posted.return_state, PurchaseReturnState::Returned);
        assert_eq!(posted.credit_state, Some(SupplierCreditState::Posted));
    }
    for (date, inv, ap, grni) in [
        ("2026-03-21", 300, 149, 50),
        ("2026-03-22", 480, 149, 230),
        ("2026-03-23", 450, 149, 200),
        ("2026-03-24", 450, 299, 50),
        ("2026-03-25", 420, 269, 50),
        ("2026-03-26", 300, 149, 50),
    ] {
        assert_report(&s, date, inv, ap, grni);
    }
    assert!(s.procurement.records.starts_with(&original));
    let before = s.clone();
    let command = receive(&s, &po, "REOPEN", "2026-03-27", 1000, 15);
    assert!(s.post_procurement_command(command).is_err());
    assert_eq!(s, before);
    let company = s.core.openings[0].command.company_id.clone();
    assert_eq!(
        s.procurement_receipt_returnable(&company, "R-L")
            .unwrap()
            .remaining_quantity_thousandths,
        0
    );
    assert_eq!(
        s.procurement_bill_returnable(&company, "B-0")
            .unwrap()
            .remaining_value,
        irr(0)
    );
    s.validate().unwrap();
}
fn tiny(value: i64) -> (AccountingState, String, String) {
    let mut s = fixture();
    let company = s.core.openings[0].command.company_id.clone();
    let mut item = s.core.openings[0].command.items[0].clone();
    item.id = "tiny-return-item".into();
    item.sku = "tiny-return-sku".into();
    let command = ProcurementCommand::Resources(RegisterResources {
        context: context(&s, "TINY-MASTER", "2026-03-21"),
        parties: vec![],
        units: vec![],
        items: vec![item.clone()],
        warehouses: vec![],
    });
    s.post_procurement_command(command).unwrap();
    let mut command = order(&s);
    if let ProcurementCommand::Order(x) = &mut command {
        x.lines[0].item_id = item.id.clone();
        x.lines[0].quantity_thousandths = 3000;
        x.lines[0].unit_price = irr(1);
        x.lines[0].line_discount = irr(3 - value);
        x.supplied_total = irr(value);
    }
    let po = s.post_procurement_command(command).unwrap().record_id;
    let command = receive(&s, &po, "R", "2026-03-22", 3000, value);
    s.post_procurement_command(command).unwrap();
    (s, company, item.id)
}
#[test]
fn tiny_shared_receipt_cursor_is_distinct_from_specific_bill_credit_cursor() {
    let (mut s, company, item) = tiny(1);
    let command = bill(&s, "B1", "2026-03-23", &[("R-L", 1000, 0)]);
    assert!(
        s.post_procurement_command(command)
            .unwrap()
            .journal_id
            .is_none()
    );
    let command = ret(&s, "U", "2026-03-24", "R-L", None, 1000, 1);
    s.post_procurement_command(command).unwrap();
    let e = s.procurement_receipt_returnable(&company, "R-L").unwrap();
    assert_eq!(
        (e.remaining_quantity_thousandths, e.remaining_value),
        (1000, irr(0))
    );
    let command = ret(&s, "C1", "2026-03-25", "R-L", Some("B1-0"), 1000, 0);
    let result = s.post_procurement_command(command).unwrap();
    assert!(result.journal_id.is_none());
    let ProcurementData::Return(p) = &s.procurement_document(&result.record_id).unwrap().data
    else {
        panic!()
    };
    assert_eq!(p.credit_state, Some(SupplierCreditState::Posted));
    assert!(p.credit_id.is_some());
    let command = bill(&s, "B2", "2026-03-25", &[("R-L", 1000, 0)]);
    assert!(
        s.post_procurement_command(command)
            .unwrap()
            .journal_id
            .is_none()
    );
    let command = ret(&s, "C2", "2026-03-25", "R-L", Some("B2-0"), 1000, 0);
    assert!(
        s.post_procurement_command(command)
            .unwrap()
            .journal_id
            .is_none()
    );
    let r = s.procurement_reconcile(&company, "2026-03-25").unwrap();
    let pool = r.stock.iter().find(|p| p.item_id == item).unwrap();
    assert_eq!((pool.quantity_thousandths, pool.value), (0, irr(0)));
    assert_report(&s, "2026-03-25", 300, 149, 50);
    s.validate().unwrap();
}
#[test]
fn two_rial_interleaving_finishes_exactly_without_denominator_rebasing() {
    let (mut s, company, _) = tiny(2);
    for command in [
        bill(&s, "B1", "2026-03-23", &[("R-L", 1000, 1)]),
        ret(&s, "U", "2026-03-23", "R-L", None, 1000, 0),
    ] {
        s.post_procurement_command(command).unwrap();
    }
    let command = ret(&s, "C1", "2026-03-24", "R-L", Some("B1-0"), 1000, 1);
    s.post_procurement_command(command).unwrap();
    let command = bill(&s, "B2", "2026-03-24", &[("R-L", 1000, 1)]);
    s.post_procurement_command(command).unwrap();
    let command = ret(&s, "C2", "2026-03-24", "R-L", Some("B2-0"), 1000, 1);
    s.post_procurement_command(command).unwrap();
    assert_eq!(
        s.procurement_receipt_returnable(&company, "R-L")
            .unwrap()
            .remaining_value,
        irr(0)
    );
    assert_report(&s, "2026-03-24", 300, 149, 50);
    s.validate().unwrap();
}
#[test]
fn unchanged_same_and_different_key_retries_work_after_lock_but_changed_identity_never_does() {
    let (mut s, _) = received();
    let command = ret(&s, "U", "2026-03-23", "R-L", None, 2000, 30);
    let first = s.post_procurement_command(command.clone()).unwrap();
    let period = s.core.openings[0].command.period_id.clone();
    s.set_period_lock(&period, true, "owner", "lock fixture")
        .unwrap();
    assert_eq!(s.post_procurement_command(command.clone()).unwrap(), first);
    let count = (s.journals.len(), s.audit.len(), s.procurement.records.len());
    let mut alias = command.clone();
    if let ProcurementCommand::Return(x) = &mut alias {
        x.context.request_key = "another-key".into();
    }
    assert_eq!(s.post_procurement_command(alias.clone()).unwrap(), first);
    assert_eq!(
        (s.journals.len(), s.audit.len(), s.procurement.records.len()),
        count
    );
    for change in 0..5 {
        let mut bad = alias.clone();
        if let ProcurementCommand::Return(x) = &mut bad {
            match change {
                0 => x.context.source_revision += 1,
                1 => x.context.reason = "different provenance".into(),
                2 => x.lines[0].quantity_thousandths = 1000,
                3 => x.return_id = "other-return".into(),
                _ => x.document_date = "2026-03-22".into(),
            }
            x.context.request_key = format!("changed-{change}");
        }
        let before = s.clone();
        assert!(s.post_procurement_command(bad).is_err());
        assert_eq!(s, before);
    }
    s.validate().unwrap();
}
#[test]
fn invalid_source_mode_identity_quantity_cost_and_dates_are_failure_atomic() {
    let (mut s, _) = received();
    let command = bill(&s, "B", "2026-03-23", &[("R-L", 5000, 75)]);
    s.post_procurement_command(command).unwrap();
    let mut invalid = vec![
        ret(&s, "OVER-U", "2026-03-24", "R-L", None, 8000, 120),
        ret(&s, "OVER-C", "2026-03-24", "R-L", Some("B-0"), 6000, 90),
        ret(&s, "VARIANCE", "2026-03-24", "R-L", None, 1000, 14),
        ret(&s, "NEGATIVE", "2026-03-24", "R-L", None, -1000, 0),
        ret(&s, "FRACTION", "2026-03-24", "R-L", None, 1, 0),
        ret(&s, "BACKDATE", "2026-03-22", "R-L", None, 1000, 15),
        ret(
            &s,
            "OPENING",
            "2026-03-24",
            &s.core.openings[0].command.grni[0].id,
            None,
            1000,
            10,
        ),
    ];
    for mode in 0..5 {
        let mut bad = ret(
            &s,
            &format!("BAD-{mode}"),
            "2026-03-24",
            "R-L",
            Some("B-0"),
            1000,
            15,
        );
        if let ProcurementCommand::Return(x) = &mut bad {
            match mode {
                0 => x.credit_id = None,
                1 => x.lines[0].receipt_line_id = "other-receipt".into(),
                2 => x.return_id = x.credit_id.clone().unwrap(),
                3 => x.return_id = s.core.openings[0].command.items[0].id.clone(),
                _ => x.document_date = "2026-03-25".into(),
            }
        }
        invalid.push(bad);
    }
    for bad in invalid {
        let before = s.clone();
        assert!(s.post_procurement_command(bad).is_err());
        assert_eq!(s, before);
    }
    let command = ret(&s, "OK", "2026-03-24", "R-L", Some("B-0"), 1000, 15);
    s.post_procurement_command(command.clone()).unwrap();
    for kind in 0..2 {
        let mut duplicate = ret(
            &s,
            &format!("PAPER-DUP-{kind}"),
            "2026-03-24",
            "R-L",
            Some("B-0"),
            1000,
            15,
        );
        if let (ProcurementCommand::Return(x), ProcurementCommand::Return(old)) =
            (&mut duplicate, &command)
        {
            if kind == 0 {
                x.return_reference = format!("  {}  ", old.return_reference)
            } else {
                x.credit_reference = old.credit_reference.clone()
            }
        }
        let before = s.clone();
        assert!(s.post_procurement_command(duplicate).is_err());
        assert_eq!(s, before);
    }
    s.validate().unwrap();
}
fn sales_order(s: &AccountingState) -> SalesCommand {
    let (i, u, w, _, _) = ids(s);
    let customer = s.core.openings[0]
        .command
        .open_items
        .iter()
        .find(|x| x.side == sarv_quote::core::OpenItemSide::Receivable)
        .unwrap()
        .party_id
        .clone();
    SalesCommand::Order(SalesOrder {
        context: context(s, "SO", "2026-03-26"),
        customer_id: customer,
        reference: "SO-199".into(),
        document_date: "2026-03-26".into(),
        lines: vec![SalesOrderLine {
            id: "SO-L".into(),
            item_id: i,
            warehouse_id: w,
            unit_id: u,
            quantity_thousandths: 10000,
            unit_price: irr(20),
            line_discount: irr(0),
            price_reference: "independent20".into(),
            price_date: "2026-03-26".into(),
            override_reason: None,
        }],
        document_discount: irr(1),
        supplied_total: irr(199),
        quotation: None,
    })
}
fn sales_fulfill(
    s: &AccountingState,
    oid: &str,
    source: &str,
    date: &str,
    q: i64,
    v: i64,
) -> SalesCommand {
    let (_, u, w, _, _) = ids(s);
    SalesCommand::Fulfill(FulfillOrder {
        context: context(s, source, date),
        order_id: oid.into(),
        delivery_id: format!("{source}-delivery"),
        invoice_id: format!("{source}-invoice"),
        delivery_reference: format!("{source}-delivery"),
        invoice_reference: format!("{source}-invoice"),
        document_date: date.into(),
        due_date: "2026-04-30".into(),
        lines: vec![FulfillLine {
            id: format!("{source}-L"),
            order_line_id: "SO-L".into(),
            warehouse_id: w,
            unit_id: u,
            quantity_thousandths: q,
        }],
        supplied_total: irr(v),
    })
}
fn sales_ret(
    s: &AccountingState,
    fid: &str,
    source: &str,
    date: &str,
    q: i64,
    credit: i64,
    cost: i64,
) -> SalesCommand {
    SalesCommand::Return(ReturnSale {
        context: context(s, source, date),
        fulfillment_id: fid.into(),
        return_id: format!("{source}-return"),
        credit_id: format!("{source}-credit"),
        return_reference: format!("{source}-return"),
        credit_reference: format!("{source}-credit"),
        document_date: date.into(),
        lines: vec![ReturnLine {
            id: format!("{source}-L"),
            fulfillment_line_id: "F1-L".into(),
            quantity_thousandths: q,
            supplied_credit: irr(credit),
            supplied_cost: irr(cost),
        }],
        supplied_credit: irr(credit),
        supplied_cost: irr(cost),
    })
}
#[test]
fn m5_literal_original_rb_cost_reduces_only_stock_and_linked_bill_ap() {
    let mut s = fixture();
    let command = order(&s);
    let po = s.post_procurement_command(command).unwrap().record_id;
    for (id, date, q, v) in [
        ("RA", "2026-03-22", 5000, 75),
        ("RB", "2026-03-23", 7000, 105),
    ] {
        let command = receive(&s, &po, id, date, q, v);
        s.post_procurement_command(command).unwrap();
    }
    let command = bill(&s, "BA", "2026-03-24", &[("RA-L", 2000, 30)]);
    s.post_procurement_command(command).unwrap();
    let command = bill(
        &s,
        "BB",
        "2026-03-25",
        &[("RA-L", 3000, 45), ("RB-L", 7000, 105)],
    );
    let bill_id = s.post_procurement_command(command).unwrap().record_id;
    let command = transfer(&s, "TX", "2026-03-26", 6000);
    s.post_procurement_command(command).unwrap();
    let command = sales_order(&s);
    let so = s.post_sales_command(command).unwrap().record_id;
    let command = sales_fulfill(&s, &so, "F1", "2026-03-27", 5000, 100);
    let f = s.post_sales_command(command).unwrap().record_id;
    for (id, date, q, credit, cost) in [
        ("SR1", "2026-03-28", 2000, 40, 23),
        ("SR2", "2026-03-29", 3000, 60, 35),
    ] {
        let command = sales_ret(&s, &f, id, date, q, credit, cost);
        s.post_sales_command(command).unwrap();
    }
    let command = sales_fulfill(&s, &so, "F2", "2026-03-31", 5000, 99);
    s.post_sales_command(command).unwrap();
    let sales_original = s.sales.clone();
    let company = s.core.openings[0].command.company_id.clone();
    let warehouse = ids(&s).2;
    for (id, date, q, v, inv, ap, balance, wq, wv, tb) in [
        (
            "PR1",
            "2026-04-01",
            3000,
            45,
            377,
            284,
            105,
            22000,
            247,
            1585,
        ),
        (
            "PR2",
            "2026-04-02",
            4000,
            60,
            317,
            224,
            45,
            18000,
            187,
            1525,
        ),
    ] {
        let command = ret(&s, id, date, "RB-L", Some("BB-1"), q, v);
        s.post_procurement_command(command).unwrap();
        assert_report(&s, date, inv, ap, 50);
        let report = s.sales_reconcile(&company, date).unwrap();
        assert!(report.reconciled);
        for (role, v) in [
            (CoreRole::Receivables, 299),
            (CoreRole::Sales, 199),
            (CoreRole::SalesReturns, 100),
            (CoreRole::Cogs, 58),
        ] {
            assert_eq!(
                report
                    .controls
                    .iter()
                    .find(|c| c.role == role)
                    .unwrap()
                    .detail,
                irr(v)
            );
        }
        assert_eq!(
            (report.total_debit, report.total_credit),
            (irr(tb), irr(tb))
        );
        let pool = report
            .stock
            .iter()
            .find(|p| p.warehouse_id == warehouse)
            .unwrap();
        assert_eq!((pool.quantity_thousandths, pool.value), (wq, irr(wv)));
        assert_eq!(
            report
                .payables
                .iter()
                .find(|p| p.id == bill_id)
                .unwrap()
                .remaining,
            irr(balance)
        );
    }
    assert_eq!(s.sales, sales_original);
    assert_report(&s, "2026-03-31", 422, 329, 50);
    s.validate().unwrap();
    let r = s.procurement.records.last().unwrap().clone();
    let mut forged = s.clone();
    if let ProcurementData::Return(x) = &mut forged.procurement.records.last_mut().unwrap().data {
        x.lines[0].value = irr(59);
    }
    assert!(forged.validate().is_err());
    let mut forged = s.clone();
    forged.procurement.records.last_mut().unwrap().movements[0].source_line_id = "unrelated".into();
    assert!(forged.validate().is_err());
    let before = s.clone();
    assert!(
        s.reverse(
            &r.journal_id.unwrap(),
            &s.core.openings[0].command.period_id.clone(),
            "2026-04-03",
            "generic-return-reversal",
            "owner",
            "unsupported generic reversal"
        )
        .is_err()
    );
    assert_eq!(s, before);
}
#[test]
fn incompatible_original_cost_quantity_value_and_zero_quantity_orphan_are_atomic() {
    let (mut s, _, item) = tiny(0);
    let mut command = order(&s);
    if let ProcurementCommand::Order(x) = &mut command {
        x.context = context(&s, "EXP-PO", "2026-03-23");
        x.reference = "EXP-PO".into();
        x.lines[0].id = "EXP-PO-L".into();
        x.lines[0].item_id = item.clone();
        x.lines[0].quantity_thousandths = 1000;
        x.lines[0].unit_price = irr(100);
        x.supplied_total = irr(100);
    }
    let po = s.post_procurement_command(command).unwrap().record_id;
    let mut command = receive(&s, &po, "EXP-R", "2026-03-23", 1000, 100);
    if let ProcurementCommand::Receive(x) = &mut command {
        x.lines[0].order_line_id = "EXP-PO-L".into();
    }
    s.post_procurement_command(command).unwrap();
    let mut command = transfer(&s, "MIX-TX", "2026-03-24", 3000);
    if let ProcurementCommand::Transfer(x) = &mut command {
        x.item_id = item;
    }
    s.post_procurement_command(command).unwrap(); // Original warehouse is one piece / 25 IRR.
    for command in [
        ret(&s, "VALUE", "2026-03-25", "EXP-R-L", None, 1000, 100),
        ret(&s, "ORPHAN", "2026-03-25", "R-L", None, 1000, 0),
        ret(&s, "QUANTITY", "2026-03-25", "R-L", None, 2000, 0),
    ] {
        let before = s.clone();
        let error = s.post_procurement_command(command).unwrap_err();
        assert!(error.to_string().contains("اصلاح بهای"));
        assert_eq!(s, before);
    }
    s.validate().unwrap();
}
#[test]
fn billed_multiple_receipts_keep_each_original_warehouse_and_reject_mixed_bills_or_unbilled_receipts()
 {
    let mut s = fixture();
    let command = order(&s);
    let po = s.post_procurement_command(command).unwrap().record_id;
    let command = receive(&s, &po, "RA", "2026-03-22", 5000, 75);
    s.post_procurement_command(command).unwrap();
    let mut command = receive(&s, &po, "RB", "2026-03-23", 7000, 105);
    if let ProcurementCommand::Receive(x) = &mut command {
        x.lines[0].warehouse_id = ids(&s).3;
    }
    s.post_procurement_command(command).unwrap();
    let mut bad = ret(&s, "MIX-U", "2026-03-24", "RA-L", None, 1000, 15);
    if let ProcurementCommand::Return(x) = &mut bad {
        x.lines.push(PurchaseReturnLine {
            id: "MIX-U-2".into(),
            receipt_line_id: "RB-L".into(),
            bill_line_id: None,
            quantity_thousandths: 1000,
            supplied_value: irr(15),
        });
        x.supplied_total = irr(30);
    }
    let before = s.clone();
    assert!(s.post_procurement_command(bad).is_err());
    assert_eq!(s, before);
    let command = bill(
        &s,
        "B",
        "2026-03-24",
        &[("RA-L", 3000, 45), ("RB-L", 5000, 75)],
    );
    s.post_procurement_command(command).unwrap();
    let command = bill(&s, "OTHER", "2026-03-24", &[("RB-L", 2000, 30)]);
    s.post_procurement_command(command).unwrap();
    let mut command = ret(&s, "C", "2026-03-25", "RA-L", Some("B-0"), 1000, 15);
    if let ProcurementCommand::Return(x) = &mut command {
        x.lines.push(PurchaseReturnLine {
            id: "C-L2".into(),
            receipt_line_id: "RB-L".into(),
            bill_line_id: Some("B-1".into()),
            quantity_thousandths: 2000,
            supplied_value: irr(30),
        });
        x.supplied_total = irr(45);
    }
    let mut bad = command.clone();
    if let ProcurementCommand::Return(x) = &mut bad {
        x.lines[1].bill_line_id = Some("OTHER-0".into());
    }
    let before = s.clone();
    assert!(s.post_procurement_command(bad).is_err());
    assert_eq!(s, before);
    let posted = s.post_procurement_command(command).unwrap();
    let r = s.procurement_document(&posted.record_id).unwrap();
    assert_eq!(r.movements.len(), 2);
    assert_eq!(r.movements[0].warehouse_id, ids(&s).2);
    assert_eq!(r.movements[1].warehouse_id, ids(&s).3);
    assert_report(&s, "2026-03-25", 435, 254, 80);
    s.validate().unwrap();
}
#[test]
fn free_unbilled_return_retains_source_state_and_blocks_an_earlier_bill_allocation() {
    let (mut s, company, item) = tiny(0);
    let command = ret(&s, "FREE", "2026-03-24", "R-L", None, 3000, 0);
    let before = (s.journals.len(), s.audit.len());
    let posted = s.post_procurement_command(command).unwrap();
    assert!(posted.journal_id.is_none());
    assert_eq!((s.journals.len(), s.audit.len()), (before.0, before.1 + 1));
    let r = s.procurement_document(&posted.record_id).unwrap();
    let ProcurementData::Return(v) = &r.data else {
        panic!()
    };
    assert_eq!(v.return_state, PurchaseReturnState::Returned);
    assert!(v.credit_id.is_none());
    assert!(v.receipt_id.is_some());
    let report = s.procurement_reconcile(&company, "2026-03-24").unwrap();
    let pool = report.stock.iter().find(|p| p.item_id == item).unwrap();
    assert_eq!((pool.quantity_thousandths, pool.value), (0, irr(0)));
    let command = bill(&s, "LATE-OLD-BILL", "2026-03-23", &[("R-L", 1000, 0)]);
    let before = s.clone();
    assert!(s.post_procurement_command(command).is_err());
    assert_eq!(s, before);
    s.validate().unwrap();
}
