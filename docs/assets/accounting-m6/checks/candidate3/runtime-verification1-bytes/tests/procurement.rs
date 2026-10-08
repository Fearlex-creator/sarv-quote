fn second_zero_company(s: &mut AccountingState) -> sarv_quote::core::OpeningCommand {
    let second = s
        .setup_core_company(sarv_quote::core::NewCoreCompany {
            name: "second independent company".into(),
            fiscal_name: "1405".into(),
            start_date: "2026-03-21".into(),
            end_date: "2027-03-20".into(),
            timezone: sarv_quote::core::CompanyTimezone::Utc,
            chart: sarv_quote::core::default_chart(),
            actor: "owner".into(),
        })
        .unwrap();
    let mut empty = s.core.openings[0].command.clone();
    empty.company_id = second.company_id.clone();
    empty.period_id = second.period_id;
    empty.setup_id = second.setup_id.clone();
    empty.source_id = "second-empty-opening".into();
    empty.request_key = "second-opening-key".into();
    empty.import_source = None;
    empty.parties.clear();
    empty.units.clear();
    empty.items.clear();
    empty.warehouses.clear();
    empty.cash.clear();
    empty.banks.clear();
    empty.open_items.clear();
    empty.advances.clear();
    empty.cheques.clear();
    empty.stock.clear();
    empty.grni.clear();
    let setup = s
        .core
        .setups
        .iter()
        .find(|p| p.id == second.setup_id)
        .unwrap();
    empty.balances = CoreRole::ALL
        .into_iter()
        .filter(|r| r.is_control())
        .map(|r| sarv_quote::core::OpeningBalance {
            account_id: setup.account_id(r).unwrap().into(),
            debit: Money::ZERO,
            credit: Money::ZERO,
            source_reference: format!("independent-zero-{}", r.key()),
        })
        .collect();
    empty
}

use sarv_quote::{
    accounting::{AccountingState, AuditOperation, Money},
    core::{CoreItem, CoreParty, CoreRole, CoreUnit, CoreWarehouse, PackRatio},
    procurement::*,
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
fn main_sequence() -> (AccountingState, Vec<ProcurementCommand>) {
    let mut s = fixture();
    let po = order(&s);
    let p = s.post_procurement_command(po.clone()).unwrap();
    let ra = receive(&s, &p.record_id, "RA", "2026-03-22", 5000, 75);
    s.post_procurement_command(ra.clone()).unwrap();
    let rb = receive(&s, &p.record_id, "RB", "2026-03-23", 7000, 105);
    s.post_procurement_command(rb.clone()).unwrap();
    let ba = bill(&s, "BA", "2026-03-24", &[("RA-L", 2000, 30)]);
    s.post_procurement_command(ba.clone()).unwrap();
    let bb = bill(
        &s,
        "BB",
        "2026-03-25",
        &[("RA-L", 3000, 45), ("RB-L", 7000, 105)],
    );
    s.post_procurement_command(bb.clone()).unwrap();
    let tx = transfer(&s, "TX", "2026-03-26", 6000);
    s.post_procurement_command(tx.clone()).unwrap();
    (s, vec![po, ra, rb, ba, bb, tx])
}
fn control(report: &ProcurementReconciliation, role: CoreRole) -> i64 {
    report
        .controls
        .iter()
        .find(|r| r.role == role)
        .unwrap()
        .detail
        .rials()
}
#[test]
fn literal_partial_order_receipts_bills_and_transfer_reconcile_at_every_cutoff() {
    let (s, _) = main_sequence();
    let company = &s.core.openings[0].command.company_id;
    let (_, _, w1, w2, _) = ids(&s);
    for (date, inv, ap, grni, tb, q1, v1, q2, v2) in [
        ("2026-03-21", 300, 149, 50, 1251, 24000, 240, 6000, 60),
        ("2026-03-22", 375, 149, 125, 1326, 29000, 315, 6000, 60),
        ("2026-03-23", 480, 149, 230, 1431, 36000, 420, 6000, 60),
        ("2026-03-24", 480, 179, 200, 1431, 36000, 420, 6000, 60),
        ("2026-03-25", 480, 329, 50, 1431, 36000, 420, 6000, 60),
        ("2026-03-26", 480, 329, 50, 1431, 30000, 350, 12000, 130),
    ] {
        let r = s.procurement_reconcile(company, date).unwrap();
        assert!(r.reconciled);
        assert_eq!(r.controls.len(), 12);
        assert!(r.controls.iter().all(|r| r.difference == Money::ZERO));
        assert_eq!(
            (
                control(&r, CoreRole::Inventory),
                control(&r, CoreRole::Payables),
                control(&r, CoreRole::Grni)
            ),
            (inv, ap, grni)
        );
        assert_eq!((r.total_debit.rials(), r.total_credit.rials()), (tb, tb));
        let a = r.stock.iter().find(|p| p.warehouse_id == w1).unwrap();
        let b = r.stock.iter().find(|p| p.warehouse_id == w2).unwrap();
        assert_eq!(
            (
                a.quantity_thousandths,
                a.value.rials(),
                b.quantity_thousandths,
                b.value.rials()
            ),
            (q1, v1, q2, v2)
        );
        for (role, expected) in [
            (CoreRole::Cash, 251),
            (CoreRole::Bank, 300),
            (CoreRole::Receivables, 200),
            (CoreRole::ChequesReceivable, 200),
            (CoreRole::ChequesPayable, 100),
            (CoreRole::Cogs, 0),
            (CoreRole::InventoryLoss, 0),
        ] {
            assert_eq!(control(&r, role), expected);
        }
    }
    assert_eq!(s.journals.len(), 5);
    assert!(s.procurement.records[0].journal_id.is_none());
    let tx = &s.procurement.records[5];
    assert!(tx.journal_id.is_none());
    assert_eq!(tx.movements.len(), 2);
    assert_eq!(tx.movements[0].value, irr(70));
    assert_eq!(tx.movements[1].value, irr(70));
    assert_eq!(
        s.audit.last().unwrap().operation,
        AuditOperation::ProcurementRecorded
    );
    assert_eq!(
        s.reconcile_opening(company, "2026-03-21")
            .unwrap()
            .total_debit,
        irr(1251)
    );
    s.validate().unwrap();
    let restored: AccountingState =
        serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, s);
}
#[test]
fn opening_grni_is_matched_without_receiving_historical_stock_again() {
    let (mut s, _) = main_sequence();
    let opening = s.core.openings[0].command.grni[0].id.clone();
    let b = bill(&s, "OPENING-BILL", "2026-03-27", &[(&opening, 2000, 20)]);
    s.post_procurement_command(b).unwrap();
    let b = bill(
        &s,
        "OPENING-BILL-FINAL",
        "2026-03-28",
        &[(&opening, 3000, 30)],
    );
    s.post_procurement_command(b).unwrap();
    let r = s
        .procurement_reconcile(&s.core.openings[0].command.company_id, "2026-03-28")
        .unwrap();
    assert_eq!(
        (
            control(&r, CoreRole::Payables),
            control(&r, CoreRole::Grni),
            control(&r, CoreRole::Inventory)
        ),
        (379, 0, 480)
    );
    assert_eq!(
        r.stock.iter().map(|p| p.quantity_thousandths).sum::<i64>(),
        42000
    );
    s.validate().unwrap();
}
#[test]
fn exact_retry_aliases_and_changed_source_revision_dates_cost_or_provenance_reject() {
    let (mut s, commands) = main_sequence();
    let once = s.clone();
    for cmd in &commands {
        let result = s.post_procurement_command(cmd.clone()).unwrap();
        assert_eq!(
            s.procurement_document(&result.record_id).unwrap().command,
            *cmd
        );
    }
    assert_eq!(s, once);
    for (i, cmd) in commands.iter().enumerate() {
        let mut alias = cmd.clone();
        match &mut alias {
            ProcurementCommand::Order(x) => x.context.request_key = format!("alias-{i}"),
            ProcurementCommand::Receive(x) => x.context.request_key = format!("alias-{i}"),
            ProcurementCommand::Bill(x) => x.context.request_key = format!("alias-{i}"),
            ProcurementCommand::Transfer(x) => x.context.request_key = format!("alias-{i}"),
            _ => unreachable!(),
        };
        s.post_procurement_command(alias).unwrap();
    }
    assert_eq!(s.journals.len(), once.journals.len());
    assert_eq!(s.audit, once.audit);
    assert_eq!(s.procurement.records, once.procurement.records);
    assert_eq!(s.procurement.requests.len(), 12);
    s.validate().unwrap();
    let before = s.clone();
    let mut changed = commands[3].clone();
    if let ProcurementCommand::Bill(x) = &mut changed {
        x.due_date = "2026-05-01".into();
    }
    assert!(s.post_procurement_command(changed).is_err());
    assert_eq!(s, before);
    let mut changed = commands[1].clone();
    if let ProcurementCommand::Receive(x) = &mut changed {
        x.context.source_revision = 2;
        x.context.request_key = "rev2".into();
    }
    assert!(s.post_procurement_command(changed).is_err());
    assert_eq!(s, before);
    let mut bad = s.clone();
    bad.procurement.records[1].movements[0].value = irr(74);
    assert!(bad.validate().is_err());
    let mut bad = s.clone();
    bad.procurement.records[3].command = commands[3].clone();
    if let ProcurementCommand::Bill(x) = &mut bad.procurement.records[3].command {
        x.due_date = "2026-05-01".into();
    }
    assert!(bad.validate().is_err());
    let mut bad = s.clone();
    bad.procurement.records[5].sequence += 1;
    assert!(bad.validate().is_err());
    assert!(s.validate_transition(&bad).is_err());
}
#[test]
fn invalid_overreceipt_overbill_variance_transfer_backdate_and_lock_are_atomic() {
    let (mut s, _) = main_sequence();
    let before = s.clone();
    let order = s.procurement.records[0].id.clone();
    let mut invalid = vec![
        receive(&s, &order, "OVER", "2026-03-27", 1000, 15),
        bill(&s, "OVERBILL", "2026-03-27", &[("RA-L", 1000, 15)]),
        transfer(&s, "EXCESS", "2026-03-27", 31000),
        transfer(&s, "BACKDATE", "2026-03-25", 1000),
    ];
    let opening = s.core.openings[0].command.grni[0].id.clone();
    invalid.push(bill(&s, "VARIANCE", "2026-03-27", &[(&opening, 1000, 11)]));
    for cmd in invalid {
        assert!(s.post_procurement_command(cmd).is_err());
        assert_eq!(s, before);
    }
    let period = s.core.openings[0].command.period_id.clone();
    s.set_period_lock(&period, true, "owner", "period lock")
        .unwrap();
    let locked = s.clone();
    assert!(
        s.post_procurement_command(transfer(&s, "LOCKED", "2026-03-27", 1000))
            .is_err()
    );
    assert_eq!(s, locked);
    s.validate().unwrap();
}
#[test]
fn zero_and_residual_costs_preserve_full_original_allocation_and_empty_pool() {
    let mut s = fixture();
    let (_, _, w1, w2, supplier) = ids(&s);
    let company = s.core.openings[0].command.company_id.clone();
    let resources = RegisterResources {
        context: context(&s, "NEW-MASTERS", "2026-03-21"),
        parties: vec![CoreParty {
            id: "new-supplier".into(),
            company_id: company.clone(),
            name: "new dual role".into(),
            customer: true,
            supplier: true,
        }],
        units: vec![
            CoreUnit {
                id: "tiny-piece".into(),
                company_id: company.clone(),
                name: "tiny piece".into(),
            },
            CoreUnit {
                id: "tiny-carton".into(),
                company_id: company.clone(),
                name: "tiny carton".into(),
            },
        ],
        items: vec![CoreItem {
            id: "tiny-item".into(),
            company_id: company.clone(),
            sku: "tiny-sku".into(),
            name: "tiny".into(),
            base_unit_id: "tiny-piece".into(),
            indivisible: true,
            packs: vec![PackRatio {
                unit_id: "tiny-carton".into(),
                numerator: 12,
                denominator: 1,
            }],
        }],
        warehouses: vec![CoreWarehouse {
            id: "new-warehouse".into(),
            company_id: company.clone(),
            name: "new warehouse".into(),
        }],
    };
    s.post_procurement_command(ProcurementCommand::Resources(resources))
        .unwrap();
    assert!(
        s.procurement_resources(&company)
            .unwrap()
            .parties
            .iter()
            .any(|p| p.id == "new-supplier")
    );
    let po = PurchaseOrder {
        context: context(&s, "ONE-RIAL-PO", "2026-03-21"),
        supplier_id: supplier,
        reference: "one-rial".into(),
        document_date: "2026-03-21".into(),
        lines: vec![OrderLine {
            id: "tiny-PO-L".into(),
            item_id: "tiny-item".into(),
            warehouse_id: w1.clone(),
            unit_id: "tiny-carton".into(),
            quantity_thousandths: 1000,
            unit_price: irr(1),
            line_discount: Money::ZERO,
            price_reference: "one-rial-carton".into(),
            price_date: "2026-03-21".into(),
            override_reason: None,
        }],
        document_discount: Money::ZERO,
        supplied_total: irr(1),
    };
    let p = s
        .post_procurement_command(ProcurementCommand::Order(po))
        .unwrap();
    for (source, value, date) in [("TINY-A", 1, "2026-03-22"), ("TINY-B", 0, "2026-03-23")] {
        let cmd = ProcurementCommand::Receive(ReceiveOrder {
            context: context(&s, source, date),
            order_id: p.record_id.clone(),
            reference: source.into(),
            document_date: date.into(),
            lines: vec![ReceiveLine {
                id: format!("{source}-L"),
                order_line_id: "tiny-PO-L".into(),
                warehouse_id: w1.clone(),
                unit_id: "tiny-piece".into(),
                quantity_thousandths: 6000,
            }],
            supplied_total: irr(value),
        });
        let posted = s.post_procurement_command(cmd).unwrap();
        assert_eq!(posted.journal_id.is_some(), value > 0);
    }
    let mut first = bill(&s, "TINY-BILL-A", "2026-03-24", &[("TINY-A-L", 3000, 1)]);
    if let ProcurementCommand::Bill(x) = &mut first {
        x.supplier_id = ids(&s).4;
    }
    s.post_procurement_command(first).unwrap();
    let next = bill(
        &s,
        "TINY-BILL-B",
        "2026-03-25",
        &[("TINY-A-L", 3000, 0), ("TINY-B-L", 6000, 0)],
    );
    let zero = s.post_procurement_command(next).unwrap();
    assert!(zero.journal_id.is_none());
    let tx = ProcurementCommand::Transfer(TransferStock {
        context: context(&s, "TINY-TRANSFER", "2026-03-26"),
        item_id: "tiny-item".into(),
        source_warehouse_id: w1.clone(),
        target_warehouse_id: w2,
        unit_id: "tiny-piece".into(),
        quantity_thousandths: 12000,
    });
    let tx = s.post_procurement_command(tx).unwrap();
    assert!(tx.journal_id.is_none());
    let report = s.procurement_reconcile(&company, "2026-03-26").unwrap();
    let pool = report
        .stock
        .iter()
        .find(|p| p.item_id == "tiny-item" && p.warehouse_id == w1)
        .unwrap();
    assert_eq!((pool.quantity_thousandths, pool.value), (0, Money::ZERO));
    assert!(report.reconciled);
    s.validate().unwrap();
}

#[test]
fn document_discount_uses_independent_largest_remainder_and_receipt_cost_is_frozen() {
    let mut s = fixture();
    let (item, _, warehouse, _, supplier) = ids(&s);
    let carton = s.core.openings[0].command.items[0].packs[0].unit_id.clone();
    let input = PurchaseOrder {
        context: context(&s, "DISCOUNT-PO", "2026-03-21"),
        supplier_id: supplier,
        reference: "discount-fixture".into(),
        document_date: "2026-03-21".into(),
        lines: [("DISCOUNT-A", 1251), ("DISCOUNT-B", 349)]
            .into_iter()
            .map(|(id, price)| OrderLine {
                id: id.into(),
                item_id: item.clone(),
                warehouse_id: warehouse.clone(),
                unit_id: carton.clone(),
                quantity_thousandths: 1000,
                unit_price: irr(price),
                line_discount: Money::ZERO,
                price_reference: "independent carton source".into(),
                price_date: "2026-03-21".into(),
                override_reason: None,
            })
            .collect(),
        document_discount: irr(101),
        supplied_total: irr(1499),
    };
    let p = s
        .post_procurement_command(ProcurementCommand::Order(input))
        .unwrap();
    let ProcurementData::Order(order) = &s.procurement_document(&p.record_id).unwrap().data else {
        panic!()
    };
    assert_eq!(
        (
            order.lines[0].document_discount.rials(),
            order.lines[1].document_discount.rials()
        ),
        (79, 22)
    );
    assert_eq!(
        (order.lines[0].net.rials(), order.lines[1].net.rials()),
        (1172, 327)
    );
    let baseunit = s.core.openings[0].command.items[0].base_unit_id.clone();
    for (source, date, q, v) in [
        ("DISC-R1", "2026-03-22", 3000, 293),
        ("DISC-R2", "2026-03-23", 9000, 879),
    ] {
        let r = ReceiveOrder {
            context: context(&s, source, date),
            order_id: p.record_id.clone(),
            reference: source.into(),
            document_date: date.into(),
            lines: vec![ReceiveLine {
                id: format!("{source}-L"),
                order_line_id: "DISCOUNT-A".into(),
                warehouse_id: warehouse.clone(),
                unit_id: baseunit.clone(),
                quantity_thousandths: q,
            }],
            supplied_total: irr(v),
        };
        s.post_procurement_command(ProcurementCommand::Receive(r))
            .unwrap();
    }
    s.validate().unwrap();
}
#[test]
fn namespace_and_malformed_source_failures_leave_every_record_unchanged() {
    let mut s = fixture();
    let original = s.clone();
    let mut variants = Vec::new();
    let mut bad = order(&s);
    if let ProcurementCommand::Order(x) = &mut bad {
        x.context.request_key = s.core.openings[0].command.request_key.clone();
    }
    variants.push(bad);
    let mut bad = order(&s);
    if let ProcurementCommand::Order(x) = &mut bad {
        x.lines[0].unit_price = irr(-1);
    }
    variants.push(bad);
    let mut bad = order(&s);
    if let ProcurementCommand::Order(x) = &mut bad {
        x.lines[0].quantity_thousandths = i64::MAX;
    }
    variants.push(bad);
    let mut bad = order(&s);
    if let ProcurementCommand::Order(x) = &mut bad {
        x.lines[0].unit_price = irr(9_000_000_000_000_001);
    }
    variants.push(bad);
    let mut bad = order(&s);
    if let ProcurementCommand::Order(x) = &mut bad {
        x.lines[0].unit_id = "missing-pack".into();
    }
    variants.push(bad);
    let mut bad = order(&s);
    if let ProcurementCommand::Order(x) = &mut bad {
        x.lines[0].quantity_thousandths = 500;
    }
    variants.push(bad);
    let mut bad = order(&s);
    if let ProcurementCommand::Order(x) = &mut bad {
        x.context.company_id = "foreign".into();
    }
    variants.push(bad);
    let mut bad = order(&s);
    if let ProcurementCommand::Order(x) = &mut bad {
        x.lines[0].id = s.core.openings[0].command.items[0].id.clone();
    }
    variants.push(bad);
    for command in variants {
        assert!(s.post_procurement_command(command).is_err());
        assert_eq!(s, original);
    }
    let resources = RegisterResources {
        context: context(&s, "FOREIGN-RESOURCES", "2026-03-21"),
        parties: vec![CoreParty {
            id: "foreign-party".into(),
            company_id: "foreign".into(),
            name: "bad company".into(),
            customer: true,
            supplier: true,
        }],
        units: vec![],
        items: vec![],
        warehouses: vec![],
    };
    assert!(
        s.post_procurement_command(ProcurementCommand::Resources(resources))
            .is_err()
    );
    assert_eq!(s, original);
}

#[test]
fn restored_event_identity_cannot_alias_its_own_source_line_or_resource() {
    let mut s = fixture();
    s.post_procurement_command(order(&s)).unwrap();
    let old = s.procurement.records[0].id.clone();
    let mut bad = s.clone();
    bad.procurement.records[0].id = "PO-L".into();
    for r in &mut bad.procurement.requests {
        if r.record_id == old {
            r.record_id = "PO-L".into();
        }
    }
    for a in &mut bad.audit {
        if a.entity_id == old {
            a.entity_id = "PO-L".into();
        }
    }
    assert!(bad.validate().is_err());
    assert!(s.validate_transition(&bad).is_err());
}

#[test]
fn new_company_opening_cannot_reuse_operational_line_movement_or_resource_id() {
    let (mut s, _) = main_sequence();
    s.post_procurement_command(ProcurementCommand::Resources(RegisterResources {
        context: context(&s, "EXTRA-RESOURCE", "2026-03-26"),
        parties: vec![CoreParty {
            id: "operational-new-party".into(),
            company_id: s.core.openings[0].command.company_id.clone(),
            name: "new operational supplier".into(),
            customer: false,
            supplier: true,
        }],
        units: vec![],
        items: vec![],
        warehouses: vec![],
    }))
    .unwrap();
    let empty = second_zero_company(&mut s);
    let unchanged = s.clone();
    for collision in [
        "PO-L".to_string(),
        "operational-new-party".to_string(),
        s.procurement.records[1].movements[0].id.clone(),
        s.procurement.records[0].id.clone(),
    ] {
        let mut bad = empty.clone();
        bad.parties.push(CoreParty {
            id: collision,
            company_id: empty.company_id.clone(),
            name: "attempted reserved operational identity".into(),
            customer: true,
            supplier: true,
        });
        assert!(s.post_opening(bad).is_err());
        assert_eq!(s, unchanged);
    }
    s.post_opening(empty).unwrap();
    s.validate().unwrap();
}
#[test]
fn opening_invoice_and_receipt_sources_cannot_be_recreated_with_new_operational_ids() {
    let mut s = fixture();
    let p = s.post_procurement_command(order(&s)).unwrap();
    let old = s.core.openings[0].command.grni[0].clone();
    let mut duplicate = receive(&s, &p.record_id, "DUP-OPEN-RECEIPT", "2026-03-22", 1000, 15);
    if let ProcurementCommand::Receive(x) = &mut duplicate {
        x.reference = old.receipt_reference.clone();
        x.document_date = old.received_date.clone();
    }
    let before = s.clone();
    assert!(s.post_procurement_command(duplicate).is_err());
    assert_eq!(s, before);
    let payable = s.core.openings[0]
        .command
        .open_items
        .iter()
        .find(|p| p.side == sarv_quote::core::OpenItemSide::Payable && p.party_id == old.party_id)
        .unwrap();
    let mut duplicate = bill(&s, "DUP-OPEN-INVOICE", "2026-03-22", &[(&old.id, 1000, 10)]);
    if let ProcurementCommand::Bill(x) = &mut duplicate {
        x.reference = payable.document_reference.clone();
        x.document_date = payable.document_date.clone();
    }
    assert!(s.post_procurement_command(duplicate).is_err());
    assert_eq!(s, before);
}
#[test]
fn matched_frozen_receipt_from_locked_prior_period_posts_only_in_current_unlocked_period() {
    let mut s = fixture();
    let p = s.post_procurement_command(order(&s)).unwrap();
    let r = receive(&s, &p.record_id, "RA", "2026-03-22", 5000, 75);
    s.post_procurement_command(r).unwrap();
    let company = s.core.openings[0].command.company_id.clone();
    let oldperiod = s.core.openings[0].command.period_id.clone();
    s.set_period_lock(&oldperiod, true, "owner", "completed prior financial year")
        .unwrap();
    let newperiod = s
        .add_period(&company, "next year", "2027-03-21", "2028-03-19", "owner")
        .unwrap();
    let history = s.procurement.records.clone();
    let mut b = bill(&s, "NEXT-YEAR-BILL", "2027-03-21", &[("RA-L", 5000, 75)]);
    if let ProcurementCommand::Bill(x) = &mut b {
        x.context.period_id = newperiod;
        x.due_date = "2027-04-30".into();
    }
    s.post_procurement_command(b).unwrap();
    assert!(s.procurement.records.starts_with(&history));
    let old = s.procurement_reconcile(&company, "2026-03-22").unwrap();
    assert_eq!(
        (
            control(&old, CoreRole::Payables),
            control(&old, CoreRole::Grni)
        ),
        (149, 125)
    );
    let current = s.procurement_reconcile(&company, "2027-03-21").unwrap();
    assert_eq!(
        (
            control(&current, CoreRole::Payables),
            control(&current, CoreRole::Grni)
        ),
        (224, 50)
    );
    s.validate().unwrap();
}

#[test]
fn recomputed_backup_checksum_cannot_hide_zero_initialization_procurement_key_collisions() {
    use sarv_quote::{
        domain::State,
        imports::hash,
        store::{Store, backup_state},
    };
    let (mut s, _) = main_sequence();
    let zero = second_zero_company(&mut s);
    s.post_opening(zero).unwrap();
    s.validate().unwrap();
    let state = State {
        accounting: s.clone(),
        ..State::default()
    };
    let dir = tempfile::tempdir().unwrap();
    let valid_path = dir.path().join("valid-cross-module.json");
    backup_state(&state, &valid_path).unwrap();
    assert!(Store::restore_preview(&valid_path).is_ok());
    let zero_key = s.core.zero_requests[0].request_key.clone();
    let mut variants = Vec::new();
    for key in ["key-PO", "key-RA"] {
        let mut bad = s.clone();
        let mut alias = bad.core.zero_requests[0].clone();
        alias.request_key = key.into();
        bad.core.zero_requests.push(alias);
        variants.push(bad);
    }
    let mut bad = s.clone();
    let mut alias = bad.procurement.requests[0].clone();
    alias.request_key = zero_key;
    bad.procurement.requests.push(alias);
    variants.push(bad);
    for (index, bad) in variants.into_iter().enumerate() {
        assert!(bad.validate().is_err());
        assert!(s.validate_transition(&bad).is_err());
        let bad_state = State {
            accounting: bad,
            ..state.clone()
        };
        let checksum = hash(&serde_json::to_vec(&bad_state).unwrap());
        let envelope = serde_json::json!({
            "format": "sarv-accounting-backup-v7", "checksum": checksum, "state": bad_state
        });
        let path = dir
            .path()
            .join(format!("invalid-cross-module-{index}.json"));
        std::fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
        // The outer envelope is internally consistent. Complete domain replay
        // must reject the conflicting alias in either direction.
        let parsed: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let typed: State = serde_json::from_value(parsed["state"].clone()).unwrap();
        assert_eq!(
            parsed["checksum"].as_str().unwrap(),
            hash(&serde_json::to_vec(&typed).unwrap())
        );
        assert!(Store::restore_preview(&path).is_err());
    }
}
