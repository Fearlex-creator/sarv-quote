use sarv_quote::{
    accounting::{AccountingState, Money},
    core::CoreRole,
    procurement::*,
    sales::*,
};
fn irr(v: i64) -> Money {
    Money::from_rials(v)
}
fn opening() -> AccountingState {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/assets/accounting-m3/restore/core-v4.json"
    ))
    .unwrap();
    let s: AccountingState = serde_json::from_value(v["state"]["accounting"].clone()).unwrap();
    s.validate().unwrap();
    s
}
fn cx(s: &AccountingState, id: &str, date: &str) -> SalesContext {
    let c = &s.core.openings[0].command;
    SalesContext {
        company_id: c.company_id.clone(),
        period_id: c.period_id.clone(),
        setup_id: c.setup_id.clone(),
        source_id: id.into(),
        source_revision: 1,
        date: date.into(),
        request_key: format!("key-{id}"),
        actor: "sales-fixture".into(),
        reason: "independent synthetic fixture".into(),
    }
}
fn ids(s: &AccountingState) -> (String, String, String, String, String, String) {
    let c = &s.core.openings[0].command;
    (
        c.items[0].id.clone(),
        c.items[0].base_unit_id.clone(),
        c.stock
            .iter()
            .find(|x| x.value == irr(240))
            .unwrap()
            .warehouse_id
            .clone(),
        c.stock
            .iter()
            .find(|x| x.value == irr(60))
            .unwrap()
            .warehouse_id
            .clone(),
        c.grni[0].party_id.clone(),
        c.open_items
            .iter()
            .find(|x| x.side == sarv_quote::core::OpenItemSide::Receivable)
            .unwrap()
            .party_id
            .clone(),
    )
}
fn transfer(s: &AccountingState, source: &str, date: &str, q: i64) -> ProcurementCommand {
    let (i, u, w1, w2, _, _) = ids(s);
    ProcurementCommand::Transfer(TransferStock {
        context: cx(s, source, date),
        item_id: i,
        source_warehouse_id: w1,
        target_warehouse_id: w2,
        unit_id: u,
        quantity_thousandths: q,
    })
}
fn m4() -> AccountingState {
    let mut s = opening();
    let (i, u, w, _, supplier, _) = ids(&s);
    let p = s
        .post_procurement_command(ProcurementCommand::Order(PurchaseOrder {
            context: cx(&s, "PO", "2026-03-21"),
            supplier_id: supplier.clone(),
            reference: "PO".into(),
            document_date: "2026-03-21".into(),
            lines: vec![OrderLine {
                id: "PO-L".into(),
                item_id: i,
                warehouse_id: w.clone(),
                unit_id: u.clone(),
                quantity_thousandths: 12000,
                unit_price: irr(15),
                line_discount: irr(0),
                price_reference: "independent price".into(),
                price_date: "2026-03-21".into(),
                override_reason: None,
            }],
            document_discount: irr(0),
            supplied_total: irr(180),
        }))
        .unwrap();
    s.post_procurement_command(ProcurementCommand::Receive(ReceiveOrder {
        context: cx(&s, "RECEIVE", "2026-03-22"),
        order_id: p.record_id,
        reference: "RECEIVE".into(),
        document_date: "2026-03-22".into(),
        lines: vec![ReceiveLine {
            id: "RECEIVE-L".into(),
            order_line_id: "PO-L".into(),
            warehouse_id: w,
            unit_id: u,
            quantity_thousandths: 12000,
        }],
        supplied_total: irr(180),
    }))
    .unwrap();
    s.post_procurement_command(ProcurementCommand::Bill(MatchBill {
        context: cx(&s, "BILL", "2026-03-23"),
        supplier_id: supplier,
        reference: "BILL".into(),
        document_date: "2026-03-23".into(),
        due_date: "2026-04-30".into(),
        lines: vec![BillLine {
            id: "BILL-L".into(),
            receipt_line_id: "RECEIVE-L".into(),
            quantity_thousandths: 12000,
            supplied_value: irr(180),
        }],
        supplied_total: irr(180),
    }))
    .unwrap();
    s.post_procurement_command(transfer(&s, "TRANSFER", "2026-03-26", 6000))
        .unwrap();
    s.validate().unwrap();
    s
}
fn order(s: &AccountingState) -> SalesCommand {
    let (i, u, w, _, _, customer) = ids(s);
    SalesCommand::Order(SalesOrder {
        context: cx(s, "SO", "2026-03-26"),
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
fn fulfill(
    s: &AccountingState,
    oid: &str,
    source: &str,
    date: &str,
    q: i64,
    v: i64,
) -> SalesCommand {
    let (_, u, w, _, _, _) = ids(s);
    SalesCommand::Fulfill(FulfillOrder {
        context: cx(s, source, date),
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
fn ret(
    s: &AccountingState,
    fid: &str,
    source: &str,
    date: &str,
    q: i64,
    credit: i64,
    cost: i64,
) -> SalesCommand {
    SalesCommand::Return(ReturnSale {
        context: cx(s, source, date),
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
fn sequence() -> (AccountingState, Vec<SalesCommand>) {
    let mut s = m4();
    let o = order(&s);
    let oid = s.post_sales_command(o.clone()).unwrap().record_id;
    let f = fulfill(&s, &oid, "F1", "2026-03-27", 5000, 100);
    let fid = s.post_sales_command(f.clone()).unwrap().record_id;
    let a = ret(&s, &fid, "R1", "2026-03-28", 2000, 40, 23);
    s.post_sales_command(a.clone()).unwrap();
    let b = ret(&s, &fid, "R2", "2026-03-29", 3000, 60, 35);
    s.post_sales_command(b.clone()).unwrap();
    let last = fulfill(&s, &oid, "F2", "2026-03-30", 5000, 99);
    s.post_sales_command(last.clone()).unwrap();
    (s, vec![o, f, a, b, last])
}
fn detail(s: &AccountingState, date: &str, role: CoreRole) -> i64 {
    s.sales_reconcile(&s.core.openings[0].command.company_id, date)
        .unwrap()
        .controls
        .iter()
        .find(|r| r.role == role)
        .unwrap()
        .detail
        .rials()
}
#[test]
fn partial_sale_and_original_cost_returns_match_literal_controls_and_cutoffs() {
    let (s, _) = sequence();
    let company = &s.core.openings[0].command.company_id;
    let (_, _, w1, w2, _, _) = ids(&s);
    for (date, inv, ar, sales, returns, cogs, profit, tb, q, v) in [
        ("2026-03-26", 480, 200, 0, 0, 0, 0, 1431, 30000, 350),
        ("2026-03-27", 422, 300, 100, 0, 58, 42, 1531, 25000, 292),
        ("2026-03-28", 445, 260, 100, 40, 35, 25, 1531, 27000, 315),
        ("2026-03-29", 480, 200, 100, 100, 0, 0, 1531, 30000, 350),
        ("2026-03-30", 422, 299, 199, 100, 58, 41, 1630, 25000, 292),
    ] {
        let r = s.sales_reconcile(company, date).unwrap();
        assert!(r.reconciled);
        assert!(r.controls.iter().all(|r| r.difference == irr(0)));
        assert_eq!(detail(&s, date, CoreRole::Inventory), inv);
        assert_eq!(detail(&s, date, CoreRole::Receivables), ar);
        assert_eq!(detail(&s, date, CoreRole::Payables), 329);
        assert_eq!(detail(&s, date, CoreRole::Grni), 50);
        assert_eq!(
            (
                r.sales.rials(),
                r.sales_returns.rials(),
                r.cogs.rials(),
                r.profit.rials(),
                r.total_debit.rials(),
                r.total_credit.rials()
            ),
            (sales, returns, cogs, profit, tb, tb)
        );
        let a = r.stock.iter().find(|p| p.warehouse_id == w1).unwrap();
        let b = r.stock.iter().find(|p| p.warehouse_id == w2).unwrap();
        assert_eq!(
            (
                a.quantity_thousandths,
                a.value.rials(),
                b.quantity_thousandths,
                b.value.rials()
            ),
            (q, v, 12000, 130)
        );
    }
    let o = &s.sales.records[0];
    assert!(o.journal_id.is_none());
    let remaining = s.sales_order_remaining(&o.id).unwrap();
    assert_eq!(remaining[0].remaining_quantity_thousandths, 0);
    assert_eq!(remaining[0].remaining_net, irr(0));
    let SalesData::Fulfill(f) = &s.sales.records[1].data else {
        panic!()
    };
    assert_ne!(f.delivery_id, f.invoice_id);
    assert_eq!(f.lines[0].gross, irr(101));
    assert_eq!(f.lines[0].document_discount, irr(1));
    assert_eq!(
        s.sales_invoice_returnable(&s.sales.records[1].id).unwrap()[0].quantity_thousandths,
        0
    );
    s.validate().unwrap();
    let restored: AccountingState =
        serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, s);
}
#[test]
fn full_content_aliases_retry_after_lock_and_changed_revision_are_safe() {
    let (mut s, commands) = sequence();
    let once = s.clone();
    for c in &commands {
        s.post_sales_command(c.clone()).unwrap();
    }
    assert_eq!(once, s);
    for (i, c) in commands.iter().enumerate() {
        let mut c = c.clone();
        match &mut c {
            SalesCommand::Order(x) => x.context.request_key = format!("alias{i}"),
            SalesCommand::Fulfill(x) => x.context.request_key = format!("alias{i}"),
            SalesCommand::Return(x) => x.context.request_key = format!("alias{i}"),
        };
        s.post_sales_command(c).unwrap();
    }
    assert_eq!(s.sales.records, once.sales.records);
    assert_eq!(s.audit, once.audit);
    assert_eq!(s.journals, once.journals);
    s.validate().unwrap();
    let period = s.core.openings[0].command.period_id.clone();
    s.set_period_lock(&period, true, "owner", "closed").unwrap();
    for c in &commands {
        s.post_sales_command(c.clone()).unwrap();
    }
    let original = s.clone();
    for change in 0..4 {
        let mut c = commands[1].clone();
        if let SalesCommand::Fulfill(x) = &mut c {
            x.context.request_key = format!("changed{change}");
            match change {
                0 => x.context.source_revision = 2,
                1 => x.due_date = "2026-05-01".into(),
                2 => x.delivery_reference = "other".into(),
                _ => x.context.actor = "other".into(),
            }
        }
        assert!(s.post_sales_command(c).is_err());
        assert_eq!(s, original);
    }
    s.validate().unwrap();
}
#[test]
fn invalid_quantities_dates_prices_duplicates_and_locks_are_atomic() {
    let mut s = m4();
    let original = s.clone();
    for n in 0..6 {
        let mut c = order(&s);
        if let SalesCommand::Order(x) = &mut c {
            match n {
                0 => x.lines[0].quantity_thousandths = 0,
                1 => x.lines[0].quantity_thousandths = 500,
                2 => x.lines[0].unit_price = irr(-1),
                3 => x.lines[0].unit_price = irr(9_000_000_000_000_001),
                4 => x.lines[0].id = s.procurement.records[0].id.clone(),
                _ => x.context.request_key = s.procurement.requests[0].request_key.clone(),
            }
        }
        assert!(s.post_sales_command(c).is_err());
        assert_eq!(s, original);
    }
    let oid = s.post_sales_command(order(&s)).unwrap().record_id;
    let f = fulfill(&s, &oid, "F1", "2026-03-27", 5000, 100);
    let fid = s.post_sales_command(f).unwrap().record_id;
    let before = s.clone();
    for c in [
        fulfill(&s, &oid, "OVER", "2026-03-28", 6000, 119),
        fulfill(&s, &oid, "BACK", "2026-03-25", 1000, 20),
        ret(&s, &fid, "OVERRET", "2026-03-28", 6000, 120, 70),
        ret(&s, &fid, "BADPRICE", "2026-03-28", 2000, 41, 23),
        ret(&s, &fid, "BADCOST", "2026-03-28", 2000, 40, 24),
    ] {
        assert!(s.post_sales_command(c).is_err());
        assert_eq!(s, before);
    }
    let mut collision = fulfill(&s, &oid, "IDS", "2026-03-28", 1000, 20);
    if let SalesCommand::Fulfill(x) = &mut collision {
        x.invoice_id = x.delivery_id.clone();
    }
    assert!(s.post_sales_command(collision).is_err());
    assert_eq!(s, before);
    let mut duplicate = fulfill(&s, &oid, "DUP", "2026-03-28", 1000, 20);
    if let SalesCommand::Fulfill(x) = &mut duplicate {
        x.invoice_reference = "F1-invoice".into();
        x.document_date = "2026-03-27".into();
    }
    assert!(s.post_sales_command(duplicate).is_err());
    assert_eq!(s, before);
    let period = s.core.openings[0].command.period_id.clone();
    s.set_period_lock(&period, true, "owner", "closed").unwrap();
    let before = s.clone();
    assert!(
        s.post_sales_command(ret(&s, &fid, "LOCK", "2026-03-28", 2000, 40, 23))
            .is_err()
    );
    assert_eq!(s, before);
    s.validate().unwrap();
}
#[test]
fn sales_and_procurement_share_audit_stock_order_and_original_return_cost() {
    let mut s = m4();
    let oid = s.post_sales_command(order(&s)).unwrap().record_id;
    let fid = s
        .post_sales_command(fulfill(&s, &oid, "F1", "2026-03-27", 5000, 100))
        .unwrap()
        .record_id;
    s.post_procurement_command(transfer(&s, "AFTER-SALE", "2026-03-27", 5000))
        .unwrap();
    let (_, _, w1, w2, _, _) = ids(&s);
    let tx = &s.procurement.records.last().unwrap().movements;
    assert_eq!(tx[0].before.value, irr(292));
    assert_eq!(tx[0].value, irr(58));
    assert_eq!(tx[0].after.value, irr(234));
    assert_eq!(tx[1].after.value, irr(188));
    s.post_sales_command(ret(&s, &fid, "R1", "2026-03-27", 2000, 40, 23))
        .unwrap();
    let company = &s.core.openings[0].command.company_id;
    let r = s.sales_reconcile(company, "2026-03-27").unwrap();
    let a = r.stock.iter().find(|p| p.warehouse_id == w1).unwrap();
    let b = r.stock.iter().find(|p| p.warehouse_id == w2).unwrap();
    assert_eq!(
        (
            a.quantity_thousandths,
            a.value.rials(),
            b.quantity_thousandths,
            b.value.rials()
        ),
        (22000, 257, 17000, 188)
    );
    assert_eq!(r.cogs, irr(35));
    assert!(r.reconciled);
    s.validate().unwrap();
    let before = s.clone();
    assert!(
        s.post_procurement_command(transfer(&s, "BACK-PUR", "2026-03-26", 1000))
            .is_err()
    );
    assert_eq!(s, before);
}
#[test]
fn tampered_source_cost_state_lineage_and_duplicate_requests_fail_complete_replay() {
    let (s, _) = sequence();
    for n in 0..7 {
        let mut bad = s.clone();
        match n {
            0 => bad.sales.records[1].movements[0].value = irr(57),
            1 => bad.sales.records[1].sequence += 1,
            2 => {
                if let SalesData::Fulfill(f) = &mut bad.sales.records[1].data {
                    f.lines[0].cost = irr(57)
                }
            }
            3 => {
                if let SalesCommand::Fulfill(f) = &mut bad.sales.records[1].command {
                    f.due_date = "2026-05-01".into()
                }
            }
            4 => bad.sales.requests.push(bad.sales.requests[0].clone()),
            5 => {
                let mut r = bad.sales.requests[0].clone();
                r.request_key = bad.procurement.requests[0].request_key.clone();
                bad.sales.requests.push(r)
            }
            _ => {
                if let SalesData::Return(r) = &mut bad.sales.records[2].data {
                    r.credit_id = r.return_id.clone()
                }
            }
        }
        assert!(bad.validate().is_err(), "tamper variant{n}");
        assert!(s.validate_transition(&bad).is_err());
    }
}
#[test]
fn one_rial_discount_and_zero_cost_quantities_conserve_original_components_without_empty_journals()
{
    let mut s = m4();
    let (_, _, w, _, supplier, customer) = ids(&s);
    let company = s.core.openings[0].command.company_id.clone();
    s.post_procurement_command(ProcurementCommand::Resources(RegisterResources {
        context: cx(&s, "TINY-MASTERS", "2026-03-26"),
        parties: Vec::new(),
        units: vec![
            sarv_quote::core::CoreUnit {
                id: "tiny-unit".into(),
                company_id: company.clone(),
                name: "tiny-unit".into(),
            },
            sarv_quote::core::CoreUnit {
                id: "tiny-pack".into(),
                company_id: company.clone(),
                name: "tiny-pack".into(),
            },
        ],
        items: vec![sarv_quote::core::CoreItem {
            id: "tiny-item".into(),
            company_id: company.clone(),
            sku: "tiny".into(),
            name: "tiny".into(),
            base_unit_id: "tiny-unit".into(),
            indivisible: false,
            packs: vec![sarv_quote::core::PackRatio {
                unit_id: "tiny-pack".into(),
                numerator: 3,
                denominator: 1,
            }],
        }],
        warehouses: Vec::new(),
    }))
    .unwrap();
    let p = s
        .post_procurement_command(ProcurementCommand::Order(PurchaseOrder {
            context: cx(&s, "TINY-PO", "2026-03-26"),
            supplier_id: supplier,
            reference: "TINY-PO".into(),
            document_date: "2026-03-26".into(),
            lines: vec![OrderLine {
                id: "tiny-PO-L".into(),
                item_id: "tiny-item".into(),
                warehouse_id: w.clone(),
                unit_id: "tiny-pack".into(),
                quantity_thousandths: 1000,
                unit_price: irr(1),
                line_discount: irr(0),
                price_reference: "one-rial-pack".into(),
                price_date: "2026-03-26".into(),
                override_reason: None,
            }],
            document_discount: irr(0),
            supplied_total: irr(1),
        }))
        .unwrap();
    s.post_procurement_command(ProcurementCommand::Receive(ReceiveOrder {
        context: cx(&s, "TINY-RECEIVE", "2026-03-26"),
        order_id: p.record_id,
        reference: "TINY-RECEIVE".into(),
        document_date: "2026-03-26".into(),
        lines: vec![ReceiveLine {
            id: "tiny-receive-L".into(),
            order_line_id: "tiny-PO-L".into(),
            warehouse_id: w.clone(),
            unit_id: "tiny-unit".into(),
            quantity_thousandths: 3000,
        }],
        supplied_total: irr(1),
    }))
    .unwrap();
    let oid = s
        .post_sales_command(SalesCommand::Order(SalesOrder {
            context: cx(&s, "TINY-SO", "2026-03-26"),
            customer_id: customer,
            reference: "TINY-SO".into(),
            document_date: "2026-03-26".into(),
            lines: vec![OrderLine {
                id: "tiny-SO-L".into(),
                item_id: "tiny-item".into(),
                warehouse_id: w.clone(),
                unit_id: "tiny-pack".into(),
                quantity_thousandths: 1000,
                unit_price: irr(3),
                line_discount: irr(1),
                price_reference: "three-rial-pack".into(),
                price_date: "2026-03-26".into(),
                override_reason: None,
            }],
            document_discount: irr(1),
            supplied_total: irr(1),
            quotation: None,
        }))
        .unwrap()
        .record_id;
    let mut fids = Vec::new();
    for (i, net, cost, gross, ld, dd) in
        [(0, 0, 0, 0, 0, 0), (1, 1, 1, 3, 1, 1), (2, 0, 0, 0, 0, 0)]
    {
        let source = format!("tinyF{i}");
        let posted = s
            .post_sales_command(SalesCommand::Fulfill(FulfillOrder {
                context: cx(&s, &source, "2026-03-27"),
                order_id: oid.clone(),
                delivery_id: format!("tinyD{i}"),
                invoice_id: format!("tinyI{i}"),
                delivery_reference: format!("tinyD{i}"),
                invoice_reference: format!("tinyI{i}"),
                document_date: "2026-03-27".into(),
                due_date: "2026-04-30".into(),
                lines: vec![FulfillLine {
                    id: format!("tinyFL{i}"),
                    order_line_id: "tiny-SO-L".into(),
                    warehouse_id: w.clone(),
                    unit_id: "tiny-unit".into(),
                    quantity_thousandths: 1000,
                }],
                supplied_total: irr(net),
            }))
            .unwrap();
        assert_eq!(posted.journal_id.is_some(), net > 0 || cost > 0);
        let SalesData::Fulfill(f) = &s.sales_document(&posted.record_id).unwrap().data else {
            panic!()
        };
        assert_eq!(
            (
                f.lines[0].net.rials(),
                f.lines[0].cost.rials(),
                f.lines[0].gross.rials(),
                f.lines[0].line_discount.rials(),
                f.lines[0].document_discount.rials()
            ),
            (net, cost, gross, ld, dd)
        );
        fids.push(posted.record_id);
    }
    for (source, index, q, net, cost) in [
        ("zero-return0", 0, 1000, 0, 0),
        ("tiny-return1", 1, 500, 1, 1),
        ("tiny-return2", 1, 500, 0, 0),
        ("zero-return3", 2, 1000, 0, 0),
    ] {
        let posted = s
            .post_sales_command(SalesCommand::Return(ReturnSale {
                context: cx(&s, source, "2026-03-28"),
                fulfillment_id: fids[index].clone(),
                return_id: format!("{source}-R"),
                credit_id: format!("{source}-C"),
                return_reference: format!("{source}-R"),
                credit_reference: format!("{source}-C"),
                document_date: "2026-03-28".into(),
                lines: vec![ReturnLine {
                    id: format!("{source}-L"),
                    fulfillment_line_id: format!("tinyFL{index}"),
                    quantity_thousandths: q,
                    supplied_credit: irr(net),
                    supplied_cost: irr(cost),
                }],
                supplied_credit: irr(net),
                supplied_cost: irr(cost),
            }))
            .unwrap();
        assert_eq!(posted.journal_id.is_some(), net > 0 || cost > 0);
    }
    let r = s.sales_reconcile(&company, "2026-03-28").unwrap();
    let tiny = r.stock.iter().find(|p| p.item_id == "tiny-item").unwrap();
    assert_eq!((tiny.quantity_thousandths, tiny.value.rials()), (3000, 1));
    assert_eq!(
        (
            r.typed_sales.rials(),
            r.typed_sales_returns.rials(),
            r.cogs.rials()
        ),
        (1, 1, 0)
    );
    assert!(r.reconciled);
    let returning = s.sales_invoice_returnable(&fids[1]).unwrap();
    assert_eq!(
        (
            returning[0].net,
            returning[0].gross,
            returning[0].line_discount,
            returning[0].document_discount,
            returning[0].cost
        ),
        (irr(0), irr(0), irr(0), irr(0), irr(0))
    );
    s.validate().unwrap();
}
#[test]
fn ordinary_sales_adjustments_remain_distinct_from_invoice_provenance_and_reversible() {
    use sarv_quote::accounting::{JournalKind, JournalLineInput, PostCommand};
    let mut s = m4();
    let company = s.core.openings[0].command.company_id.clone();
    let period = s.core.openings[0].command.period_id.clone();
    let setup = s.core.openings[0].frozen_setup.clone();
    let journal = s
        .post(PostCommand {
            company_id: company.clone(),
            period_id: period.clone(),
            date: "2026-03-26".into(),
            source_id: "historical-ordinary-sale".into(),
            source_revision: 1,
            event: "ordinary-sale-adjustment".into(),
            request_key: "ordinary-key".into(),
            actor: "owner".into(),
            reason: "valid ordinary journal".into(),
            kind: JournalKind::Manual,
            lines: vec![
                JournalLineInput {
                    account_id: setup.account_id(CoreRole::OperatingExpense).unwrap().into(),
                    debit: irr(7),
                    credit: irr(0),
                    description: "ordinary".into(),

                    dimension: String::new(),
                },
                JournalLineInput {
                    account_id: setup.account_id(CoreRole::Sales).unwrap().into(),
                    debit: irr(0),
                    credit: irr(7),
                    description: "ordinary".into(),

                    dimension: String::new(),
                },
            ],
        })
        .unwrap();
    let oid = s.post_sales_command(order(&s)).unwrap().record_id;
    s.post_sales_command(fulfill(&s, &oid, "F1", "2026-03-27", 5000, 100))
        .unwrap();
    let r = s.sales_reconcile(&company, "2026-03-27").unwrap();
    assert!(r.reconciled);
    assert_eq!((r.typed_sales, r.sales), (irr(100), irr(107)));
    assert_eq!(r.adjustments.len(), 1);
    assert_eq!(r.adjustments[0].journal_id, journal);
    assert_eq!(r.receivables.len(), 3);
    s.reverse(
        &journal,
        &period,
        "2026-03-28",
        "ordinary-reversal",
        "owner",
        "ordinary correction",
    )
    .unwrap();
    let old = s.sales_reconcile(&company, "2026-03-27").unwrap();
    let current = s.sales_reconcile(&company, "2026-03-28").unwrap();
    assert_eq!(old.sales, irr(107));
    assert_eq!(current.sales, irr(100));
    assert_eq!(current.adjustments.len(), 2);
    assert!(current.reconciled);
    s.validate().unwrap();
}
#[test]
fn oversell_cross_module_keys_and_original_opening_invoice_duplicates_reject() {
    let mut s = m4();
    let mut oversell = order(&s);
    if let SalesCommand::Order(x) = &mut oversell {
        x.lines[0].quantity_thousandths = 31000;
        x.supplied_total = irr(619);
    }
    let oid = s.post_sales_command(oversell).unwrap().record_id;
    let before = s.clone();
    assert!(
        s.post_sales_command(fulfill(&s, &oid, "OVERSTOCK", "2026-03-27", 31000, 619))
            .is_err()
    );
    assert_eq!(s, before);
    let oid = s
        .post_sales_command({
            let mut c = order(&s);
            if let SalesCommand::Order(x) = &mut c {
                x.context.source_id = "SO2".into();
                x.context.request_key = "SO2-key".into();
                x.reference = "SO2".into();
                x.lines[0].id = "SO2-L".into();
            }
            c
        })
        .unwrap()
        .record_id;
    let mut f = fulfill(&s, &oid, "F1", "2026-03-27", 5000, 100);
    if let SalesCommand::Fulfill(x) = &mut f {
        x.lines[0].order_line_id = "SO2-L".into();
    }
    let posted = s.post_sales_command(f.clone()).unwrap();
    let opening = s.core.openings[0]
        .command
        .open_items
        .iter()
        .find(|x| x.side == sarv_quote::core::OpenItemSide::Receivable)
        .unwrap()
        .clone();
    let before = s.clone();
    let mut dupe = fulfill(&s, &oid, "OPEN-DUP", "2026-03-28", 1000, 20);
    if let SalesCommand::Fulfill(x) = &mut dupe {
        x.lines[0].order_line_id = "SO2-L".into();
        x.invoice_reference = opening.document_reference;
        x.document_date = opening.document_date;
    }
    assert!(s.post_sales_command(dupe).is_err());
    assert_eq!(s, before);
    for key in ["key-SO", "key-F1"] {
        let mut c = transfer(&s, &format!("key-collision-{key}"), "2026-03-28", 1000);
        if let ProcurementCommand::Transfer(x) = &mut c {
            x.context.request_key = key.into();
        }
        assert!(s.post_procurement_command(c).is_err());
        assert_eq!(s, before);
    }
    let journal = posted.journal_id.unwrap();
    let period = s.core.openings[0].command.period_id.clone();
    assert!(
        s.reverse(
            &journal,
            &period,
            "2026-03-28",
            "typed-reverse",
            "owner",
            "attempt generic correction"
        )
        .is_err()
    );
    assert_eq!(s, before);
    s.validate().unwrap();
}
#[test]
fn stable_discount_line_id_tiebreak_and_checked_limits() {
    let mut s = m4();
    let mut command = order(&s);
    if let SalesCommand::Order(x) = &mut command {
        let mut a = x.lines[0].clone();
        a.id = "B".into();
        a.quantity_thousandths = 1000;
        a.unit_price = irr(100);
        let mut b = a.clone();
        b.id = "A".into();
        let mut c = a.clone();
        c.id = "C".into();
        x.lines = vec![a, b, c];
        x.document_discount = irr(2);
        x.supplied_total = irr(298);
    }
    let p = s.post_sales_command(command).unwrap();
    let SalesData::Order(o) = &s.sales_document(&p.record_id).unwrap().data else {
        panic!()
    };
    assert_eq!(
        o.lines
            .iter()
            .map(|l| l.document_discount.rials())
            .collect::<Vec<_>>(),
        vec![1, 1, 0]
    );
    let before = s.clone();
    for n in 0..4 {
        let mut c = order(&s);
        if let SalesCommand::Order(x) = &mut c {
            match n {
                0 => x.lines[0].quantity_thousandths = i64::MAX,
                1 => {
                    x.lines[0].unit_price = irr(9_000_000_000_000_000);
                    x.lines[0].quantity_thousandths = 1000000000;
                }
                2 => x.lines[0].unit_id = "missing-pack".into(),
                _ => x.context.company_id = "foreign-company".into(),
            }
        }
        assert!(s.post_sales_command(c).is_err());
        assert_eq!(s, before);
    }
    s.validate().unwrap();
}

#[test]
fn divisible_quantity_uses_original_net_and_discount_before_derived_gross() {
    let mut s = m4();
    let (_, unit, warehouse, _, supplier, customer) = ids(&s);
    let company = s.core.openings[0].command.company_id.clone();
    s.post_procurement_command(ProcurementCommand::Resources(RegisterResources {
        context: cx(&s, "FRACTION-MASTERS", "2026-03-26"),
        parties: vec![],
        units: vec![],
        items: vec![sarv_quote::core::CoreItem {
            id: "fraction-item".into(),
            company_id: company.clone(),
            sku: "fraction-item".into(),
            name: "divisible fixture item".into(),
            base_unit_id: unit.clone(),
            indivisible: false,
            packs: vec![],
        }],
        warehouses: vec![],
    }))
    .unwrap();
    let purchase = s
        .post_procurement_command(ProcurementCommand::Order(PurchaseOrder {
            context: cx(&s, "FRACTION-PO", "2026-03-26"),
            supplier_id: supplier,
            reference: "FRACTION-PO".into(),
            document_date: "2026-03-26".into(),
            lines: vec![OrderLine {
                id: "fraction-PO-L".into(),
                item_id: "fraction-item".into(),
                warehouse_id: warehouse.clone(),
                unit_id: unit.clone(),
                quantity_thousandths: 1500,
                unit_price: irr(0),
                line_discount: irr(0),
                price_reference: "explicit free stock fixture".into(),
                price_date: "2026-03-26".into(),
                override_reason: None,
            }],
            document_discount: irr(0),
            supplied_total: irr(0),
        }))
        .unwrap();
    s.post_procurement_command(ProcurementCommand::Receive(ReceiveOrder {
        context: cx(&s, "FRACTION-RECEIVE", "2026-03-26"),
        order_id: purchase.record_id,
        reference: "FRACTION-RECEIVE".into(),
        document_date: "2026-03-26".into(),
        lines: vec![ReceiveLine {
            id: "fraction-receive-L".into(),
            order_line_id: "fraction-PO-L".into(),
            warehouse_id: warehouse.clone(),
            unit_id: unit.clone(),
            quantity_thousandths: 1500,
        }],
        supplied_total: irr(0),
    }))
    .unwrap();
    let order = s
        .post_sales_command(SalesCommand::Order(SalesOrder {
            context: cx(&s, "FRACTION-SO", "2026-03-26"),
            customer_id: customer,
            reference: "FRACTION-SO".into(),
            document_date: "2026-03-26".into(),
            lines: vec![OrderLine {
                id: "fraction-SO-L".into(),
                item_id: "fraction-item".into(),
                warehouse_id: warehouse.clone(),
                unit_id: unit.clone(),
                quantity_thousandths: 1500,
                unit_price: irr(1),
                line_discount: irr(0),
                price_reference: "one rial per original selected unit".into(),
                price_date: "2026-03-26".into(),
                override_reason: None,
            }],
            document_discount: irr(1),
            supplied_total: irr(1),
            quotation: None,
        }))
        .unwrap();
    let SalesData::Order(frozen) = &s.sales_document(&order.record_id).unwrap().data else {
        panic!()
    };
    assert_eq!(frozen.lines[0].input.unit_price, irr(1));
    assert_eq!(
        (
            frozen.lines[0].gross,
            frozen.lines[0].document_discount,
            frozen.lines[0].net
        ),
        (irr(2), irr(1), irr(1))
    );
    let mut net = 0;
    let mut discount = 0;
    let mut gross = 0;
    let journals_before = s.journals.len();
    // Literal cumulative targets are net [0,1,1], discount [0,1,1].
    // Deriving gross from these components gives increments [0,2,0].
    // Independently rounding original gross and subtracting discount would
    // instead create a negative second net increment, which is forbidden.
    for (index, next_net, next_discount, next_gross, cumulative_net, cumulative_discount) in
        [(0, 0, 0, 0, 0, 0), (1, 1, 1, 2, 1, 1), (2, 0, 0, 0, 1, 1)]
    {
        let source = format!("fraction-F{index}");
        let result = s
            .post_sales_command(SalesCommand::Fulfill(FulfillOrder {
                context: cx(&s, &source, "2026-03-27"),
                order_id: order.record_id.clone(),
                delivery_id: format!("fraction-D{index}"),
                invoice_id: format!("fraction-I{index}"),
                delivery_reference: format!("fraction-D{index}"),
                invoice_reference: format!("fraction-I{index}"),
                document_date: "2026-03-27".into(),
                due_date: "2026-04-30".into(),
                lines: vec![FulfillLine {
                    id: format!("fraction-FL{index}"),
                    order_line_id: "fraction-SO-L".into(),
                    warehouse_id: warehouse.clone(),
                    unit_id: unit.clone(),
                    quantity_thousandths: 500,
                }],
                supplied_total: irr(next_net),
            }))
            .unwrap();
        assert_eq!(result.journal_id.is_some(), index == 1);
        let SalesData::Fulfill(f) = &s.sales_document(&result.record_id).unwrap().data else {
            panic!()
        };
        let line = &f.lines[0];
        assert_eq!(
            (line.net, line.document_discount, line.gross, line.cost),
            (irr(next_net), irr(next_discount), irr(next_gross), irr(0))
        );
        assert_eq!(line.quantity_thousandths, 500);
        assert_eq!(line.item.id, "fraction-item");
        net += line.net.rials();
        discount += line.document_discount.rials();
        gross += line.gross.rials();
        assert_eq!((net, discount), (cumulative_net, cumulative_discount));
    }
    assert_eq!((net, discount, gross), (1, 1, 2));
    assert_eq!(s.journals.len(), journals_before + 1);
    let remaining = s.sales_order_remaining(&order.record_id).unwrap();
    assert_eq!(remaining[0].remaining_quantity_thousandths, 0);
    assert_eq!(remaining[0].remaining_net, irr(0));
    let report = s.sales_reconcile(&company, "2026-03-27").unwrap();
    let pool = report
        .stock
        .iter()
        .find(|p| p.item_id == "fraction-item")
        .unwrap();
    assert_eq!((pool.quantity_thousandths, pool.value), (0, irr(0)));
    assert_eq!((report.typed_sales, report.typed_cogs), (irr(1), irr(0)));
    assert!(report.reconciled);
    s.validate().unwrap();
}
