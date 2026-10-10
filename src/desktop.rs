//! Shared desktop boundary. Both Tauri and the opt-in local QA adapter call this
//! dispatcher; all validation and durable writes still use Store and the kernel.
use crate::{
    accounting::*,
    core::*,
    domain::{Settings, State, id, quantity},
    procurement::*,
    sales::*,
    sales_draft::RawSalesDraft,
    store::Store,
};
use anyhow::{Result, anyhow, ensure};
use serde_json::{Value, json};
use std::path::Path;

fn actor(p: &Value) -> &str {
    if p["_actor"] == "local-agent" {
        "local-agent"
    } else {
        "local-desktop"
    }
}
fn string<'a>(p: &'a Value, key: &str) -> Result<&'a str> {
    p.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing field: {key}"))
}
fn optional<'a>(p: &'a Value, key: &str, default: &'a str) -> &'a str {
    p.get(key).and_then(Value::as_str).unwrap_or(default)
}
fn integer(p: &Value, key: &str) -> Result<u64> {
    Ok(string(p, key)?.parse()?)
}
fn amount(p: &Value, key: &str) -> Result<Money> {
    Money::parse(string(p, key)?, MoneyUnit::Rial)
}
/// Every integer crossing IPC is a string. No rounding through JavaScript f64.
pub fn wire(mut value: Value) -> Value {
    fn visit(v: &mut Value) {
        match v {
            Value::Number(n) => *v = Value::String(n.to_string()),
            Value::Array(a) => a.iter_mut().for_each(visit),
            Value::Object(o) => o.values_mut().for_each(visit),
            _ => {}
        }
    }
    visit(&mut value);
    value
}
fn journal_totals(value: &mut Value) -> Result<()> {
    match value {
        Value::Array(rows) => {
            for row in rows {
                journal_totals(row)?;
            }
        }
        Value::Object(object) => {
            for child in object.values_mut() {
                journal_totals(child)?;
            }
            if object.contains_key("kind")
                && object.contains_key("period_id")
                && let Some(lines) = object.get("lines").and_then(Value::as_array)
            {
                let mut debit = Money::ZERO;
                let mut credit = Money::ZERO;
                for line in lines {
                    debit = debit.checked_add(Money::from_rials(
                        line["debit"]
                            .as_i64()
                            .ok_or_else(|| anyhow!("Invalid journal debit"))?,
                    ))?;
                    credit = credit.checked_add(Money::from_rials(
                        line["credit"]
                            .as_i64()
                            .ok_or_else(|| anyhow!("Invalid journal credit"))?,
                    ))?;
                }
                object.insert("total_debit".into(), json!(debit));
                object.insert("total_credit".into(), json!(credit));
            }
        }
        _ => {}
    }
    Ok(())
}
fn raw_draft(p: &Value) -> Result<RawSalesDraft> {
    let mut v = p.clone();
    v["revision"] = json!(integer(p, "revision")?);
    Ok(serde_json::from_value(v)?)
}
fn company<'a>(s: &'a State, p: &Value) -> Result<&'a Company> {
    let c = string(p, "company_id")?;
    s.accounting
        .companies
        .iter()
        .find(|x| x.id == c)
        .ok_or_else(|| anyhow!("Company not found"))
}
fn context(s: &State, p: &Value) -> Result<ProcurementContext> {
    let c = company(s, p)?;
    let period = string(p, "period_id")?;
    ensure!(
        s.accounting
            .periods
            .iter()
            .any(|x| x.id == period && x.company_id == c.id),
        "Period does not belong to company"
    );
    let setup = s
        .accounting
        .core
        .setups
        .iter()
        .find(|x| x.command.company_id == c.id)
        .ok_or_else(|| anyhow!("Configure the core chart and opening first"))?;
    Ok(ProcurementContext {
        company_id: c.id.clone(),
        period_id: period.into(),
        setup_id: setup.id.clone(),
        source_id: string(p, "source_id")?.into(),
        source_revision: 1,
        date: string(p, "date")?.into(),
        request_key: string(p, "request_key")?.into(),
        actor: actor(p).into(),
        reason: string(p, "reason")?.into(),
    })
}
fn opening_zero(s: &State, p: &Value) -> Result<OpeningCommand> {
    let c = company(s, p)?;
    let setup = s
        .accounting
        .core
        .setups
        .iter()
        .find(|x| x.command.company_id == c.id)
        .ok_or_else(|| anyhow!("Core setup not found"))?;
    Ok(OpeningCommand {
        company_id: c.id.clone(),
        period_id: setup.command.period_id.clone(),
        setup_id: setup.id.clone(),
        as_of: setup.command.policy.effective_date.clone(),
        source_id: string(p, "source_id")?.into(),
        request_key: string(p, "request_key")?.into(),
        actor: actor(p).into(),
        reason: "Explicitly confirmed fresh company with zero balances".into(),
        balances: setup
            .command
            .mappings
            .iter()
            .map(|m| OpeningBalance {
                account_id: m.account_id.clone(),
                debit: Money::ZERO,
                credit: Money::ZERO,
                source_reference: "Explicit fresh-company zero declaration".into(),
            })
            .collect(),
        parties: vec![],
        units: vec![],
        items: vec![],
        warehouses: vec![],
        cash: vec![],
        banks: vec![],
        open_items: vec![],
        advances: vec![],
        cheques: vec![],
        stock: vec![],
        grni: vec![],
        import_source: None,
    })
}
fn manual(s: &State, p: &Value) -> Result<PostCommand> {
    let c = company(s, p)?;
    let lines = p["lines"]
        .as_array()
        .ok_or_else(|| anyhow!("Journal lines required"))?
        .iter()
        .map(|l| {
            Ok(JournalLineInput {
                account_id: string(l, "account_id")?.into(),
                debit: amount(l, "debit")?,
                credit: amount(l, "credit")?,
                description: optional(l, "description", "").into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(PostCommand {
        company_id: c.id.clone(),
        period_id: string(p, "period_id")?.into(),
        date: string(p, "date")?.into(),
        source_id: string(p, "source_id")?.into(),
        source_revision: 1,
        event: "manual.desktop.v1".into(),
        request_key: string(p, "request_key")?.into(),
        actor: actor(p).into(),
        reason: string(p, "reason")?.into(),
        kind: JournalKind::Manual,
        lines,
    })
}
fn purchase_order(s: &State, p: &Value) -> Result<ProcurementCommand> {
    let mut total = Money::ZERO;
    let lines = p["lines"]
        .as_array()
        .ok_or_else(|| anyhow!("Order lines required"))?
        .iter()
        .map(|l| {
            let q = quantity(string(l, "quantity")?)?;
            let price = amount(l, "unit_price")?;
            let discount = Money::parse(optional(l, "line_discount", "0"), MoneyUnit::Rial)?;
            let gross = Money::from_rials(positive_half_up(
                i128::from(price.rials()) * i128::from(q),
                1000,
            )?);
            total = total.checked_add(gross.checked_sub(discount)?)?;
            Ok(OrderLine {
                id: string(l, "id")?.into(),
                item_id: string(l, "item_id")?.into(),
                warehouse_id: string(l, "warehouse_id")?.into(),
                unit_id: string(l, "unit_id")?.into(),
                quantity_thousandths: q,
                unit_price: price,
                line_discount: discount,
                price_reference: string(p, "reference")?.into(),
                price_date: string(p, "date")?.into(),
                override_reason: None,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let discount = Money::parse(optional(p, "discount", "0"), MoneyUnit::Rial)?;
    total = total.checked_sub(discount)?;
    Ok(ProcurementCommand::Order(PurchaseOrder {
        context: context(s, p)?,
        supplier_id: string(p, "supplier_id")?.into(),
        reference: string(p, "reference")?.into(),
        document_date: string(p, "date")?.into(),
        lines,
        document_discount: discount,
        supplied_total: total,
    }))
}
// Quantities are base units. The immutable kernel cursors allocate every rial.
fn selections(p: &Value) -> Result<Option<std::collections::BTreeMap<String, i64>>> {
    let Some(value) = p.get("lines") else {
        return Ok(None);
    };
    let rows = value
        .as_array()
        .ok_or_else(|| anyhow!("Selected lines required"))?;
    let mut out = std::collections::BTreeMap::new();
    for row in rows {
        let raw = string(row, "quantity")?;
        let q = if raw == "0" { 0 } else { quantity(raw)? };
        ensure!(
            out.insert(string(row, "id")?.into(), q).is_none(),
            "Duplicate source line"
        );
    }
    ensure!(
        out.values().any(|q| *q > 0),
        "Select at least one positive quantity"
    );
    Ok(Some(out))
}
fn retry_selection<'a>(p: &Value, rows: impl Iterator<Item = (&'a str, i64)>) -> Result<()> {
    if let Some(selected) = selections(p)? {
        let selected: std::collections::BTreeMap<_, _> =
            selected.into_iter().filter(|(_, q)| *q > 0).collect();
        let prior: std::collections::BTreeMap<_, _> =
            rows.map(|(id, q)| (id.to_owned(), q)).collect();
        ensure!(
            selected == prior,
            "Posted retry has different selected quantities"
        );
    }
    Ok(())
}
#[derive(serde::Serialize)]
struct OperationLine {
    id: String,
    item_id: String,
    warehouse_id: String,
    unit_id: String,
    original_quantity: i64,
    remaining_quantity: i64,
    original_value: Money,
    remaining_value: Money,
    original_cost: Money,
    remaining_cost: Money,
    receipt_line_id: Option<String>,
    bill_line_id: Option<String>,
}
impl OperationLine {
    fn value(&self, q: i64, sales: bool) -> Result<Money> {
        let f = if sales {
            crate::sales::allocation
        } else {
            crate::procurement::allocation
        };
        f(
            self.original_value,
            self.original_quantity,
            self.original_quantity - self.remaining_quantity,
            self.original_value.checked_sub(self.remaining_value)?,
            q,
        )
    }
    fn cost(&self, q: i64) -> Result<Money> {
        crate::sales::allocation(
            self.original_cost,
            self.original_quantity,
            self.original_quantity - self.remaining_quantity,
            self.original_cost.checked_sub(self.remaining_cost)?,
            q,
        )
    }
}
fn operation_lines(
    s: &State,
    company: &str,
    record: &str,
    action: &str,
) -> Result<Vec<OperationLine>> {
    let mut out = vec![];
    if action == "fulfill" || action == "sale_return" {
        let r = s.accounting.sales_document(record)?;
        ensure!(r.command.context().company_id == company, "Wrong company");
        if action == "fulfill" {
            for l in s.accounting.sales_order_remaining(record)? {
                out.push(OperationLine {
                    id: l.id,
                    item_id: l.item.id,
                    warehouse_id: l.warehouse_id,
                    unit_id: l.item.base_unit_id,
                    original_quantity: l.original_quantity_thousandths,
                    remaining_quantity: l.remaining_quantity_thousandths,
                    original_value: l.original_net,
                    remaining_value: l.remaining_net,
                    original_cost: Money::ZERO,
                    remaining_cost: Money::ZERO,
                    receipt_line_id: None,
                    bill_line_id: None,
                });
            }
        } else {
            let SalesData::Fulfill(f) = &r.data else {
                return Err(anyhow!("Not an invoice"));
            };
            for l in s.accounting.sales_invoice_returnable(record)? {
                let orig = f.lines.iter().find(|x| x.id == l.id).unwrap();
                out.push(OperationLine {
                    id: l.id,
                    item_id: l.item.id,
                    warehouse_id: l.warehouse_id,
                    unit_id: l.item.base_unit_id,
                    original_quantity: l.original_quantity_thousandths,
                    remaining_quantity: l.quantity_thousandths,
                    original_value: orig.net,
                    remaining_value: l.net,
                    original_cost: orig.cost,
                    remaining_cost: l.cost,
                    receipt_line_id: None,
                    bill_line_id: None,
                });
            }
        }
    } else {
        let r = s.accounting.procurement_document(record)?;
        ensure!(r.command.context().company_id == company, "Wrong company");
        let resources = s.accounting.procurement_resources(company)?;
        let unit = |item: &str| -> Result<String> {
            Ok(resources
                .items
                .iter()
                .find(|x| x.id == item)
                .ok_or_else(|| anyhow!("Product not found"))?
                .base_unit_id
                .clone())
        };
        if action == "receive" {
            let ProcurementData::Order(order) = &r.data else {
                return Err(anyhow!("Not an order"));
            };
            for l in &order.lines {
                let (q, v) = s
                    .accounting
                    .procurement
                    .records
                    .iter()
                    .filter_map(|x| match &x.data {
                        ProcurementData::Receive(x) if x.order_id == record => Some(x),
                        _ => None,
                    })
                    .flat_map(|x| &x.lines)
                    .filter(|x| x.order_line_id == l.input.id)
                    .try_fold((0i64, Money::ZERO), |(q, v), x| -> Result<_> {
                        Ok((
                            q.checked_add(x.quantity_thousandths)
                                .ok_or_else(|| anyhow!("Quantity overflow"))?,
                            v.checked_add(x.value)?,
                        ))
                    })?;
                out.push(OperationLine {
                    id: l.input.id.clone(),
                    item_id: l.item.id.clone(),
                    warehouse_id: l.input.warehouse_id.clone(),
                    unit_id: l.item.base_unit_id.clone(),
                    original_quantity: l.base_quantity_thousandths,
                    remaining_quantity: l
                        .base_quantity_thousandths
                        .checked_sub(q)
                        .ok_or_else(|| anyhow!("Quantity overflow"))?,
                    original_value: l.net,
                    remaining_value: l.net.checked_sub(v)?,
                    original_cost: Money::ZERO,
                    remaining_cost: Money::ZERO,
                    receipt_line_id: None,
                    bill_line_id: None,
                });
            }
        } else {
            let eligible = match (&r.data, action) {
                (ProcurementData::Receive(receipt), "bill" | "purchase_return") => receipt
                    .lines
                    .iter()
                    .map(|l| s.accounting.procurement_receipt_returnable(company, &l.id))
                    .collect::<Result<Vec<_>>>()?,
                (ProcurementData::Bill(bill), "purchase_return") => bill
                    .lines
                    .iter()
                    .map(|l| s.accounting.procurement_bill_returnable(company, &l.id))
                    .collect::<Result<Vec<_>>>()?,
                _ => return Err(anyhow!("Unsupported source operation")),
            };
            for l in eligible {
                out.push(OperationLine {
                    id: l
                        .bill_line_id
                        .clone()
                        .unwrap_or_else(|| l.receipt_line_id.clone()),
                    unit_id: unit(&l.item_id)?,
                    item_id: l.item_id,
                    warehouse_id: l.warehouse_id,
                    original_quantity: l.original_quantity_thousandths,
                    remaining_quantity: l.remaining_quantity_thousandths,
                    original_value: l.original_value,
                    remaining_value: l.remaining_value,
                    original_cost: Money::ZERO,
                    remaining_cost: Money::ZERO,
                    receipt_line_id: Some(l.receipt_line_id),
                    bill_line_id: l.bill_line_id,
                });
            }
        }
    }
    Ok(out)
}
fn selected_lines(
    s: &State,
    p: &Value,
    record: &str,
    action: &str,
) -> Result<Vec<(OperationLine, i64)>> {
    let options = operation_lines(s, string(p, "company_id")?, record, action)?;
    let selection = selections(p)?;
    if let Some(selection) = &selection {
        ensure!(
            selection
                .keys()
                .all(|id| options.iter().any(|x| &x.id == id)),
            "Source line does not belong to this document"
        );
    }
    let mut out = vec![];
    for l in options {
        let q = selection
            .as_ref()
            .map_or(l.remaining_quantity, |m| *m.get(&l.id).unwrap_or(&0));
        ensure!(
            q >= 0 && q <= l.remaining_quantity,
            "Quantity exceeds the remaining source quantity"
        );
        if q > 0 {
            out.push((l, q));
        }
    }
    ensure!(!out.is_empty(), "No eligible quantities remain");
    Ok(out)
}
fn sale_return(s: &State, p: &Value) -> Result<SalesCommand> {
    let ctx = context(s, p)?;
    if let Some(prior) = s
        .accounting
        .sales
        .records
        .iter()
        .find_map(|r| match &r.command {
            SalesCommand::Return(x)
                if x.context.company_id == ctx.company_id
                    && x.context.source_id == ctx.source_id =>
            {
                Some(x)
            }
            _ => None,
        })
    {
        let mut previous = prior.context.clone();
        previous.request_key = ctx.request_key.clone();
        ensure!(
            previous == ctx
                && prior.fulfillment_id == p["record_id"]
                && prior.return_reference == p["reference"],
            "Posted return retry has different contents"
        );
        retry_selection(
            p,
            prior
                .lines
                .iter()
                .map(|l| (l.fulfillment_line_id.as_str(), l.quantity_thousandths)),
        )?;
        let mut repeat = prior.clone();
        repeat.context = ctx;
        return Ok(SalesCommand::Return(repeat));
    }
    let mut credit = Money::ZERO;
    let mut cost = Money::ZERO;
    let lines = selected_lines(s, p, string(p, "record_id")?, "sale_return")?
        .into_iter()
        .map(|(l, q)| -> Result<_> {
            let value = l.value(q, true)?;
            let c = l.cost(q)?;
            credit = credit.checked_add(value)?;
            cost = cost.checked_add(c)?;
            Ok(ReturnLine {
                id: format!("{}-{}", ctx.source_id, l.id),
                fulfillment_line_id: l.id,
                quantity_thousandths: q,
                supplied_credit: value,
                supplied_cost: c,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(SalesCommand::Return(ReturnSale {
        return_id: format!("{}-return", ctx.source_id),
        credit_id: format!("{}-credit", ctx.source_id),
        context: ctx,
        fulfillment_id: string(p, "record_id")?.into(),
        return_reference: string(p, "reference")?.into(),
        credit_reference: format!("C-{}", string(p, "reference")?),
        document_date: string(p, "date")?.into(),
        lines,
        supplied_credit: credit,
        supplied_cost: cost,
    }))
}
fn purchase_return(s: &State, p: &Value) -> Result<ProcurementCommand> {
    let reference = string(p, "reference")?;
    let ctx = context(s, p)?;
    let r = s.accounting.procurement_document(string(p, "record_id")?)?;
    ensure!(
        r.command.context().company_id == ctx.company_id,
        "Wrong company"
    );
    let (supplier, billed, ids) = match &r.data {
        ProcurementData::Receive(x) => (
            x.supplier_id.clone(),
            false,
            x.lines.iter().map(|l| l.id.clone()).collect::<Vec<_>>(),
        ),
        ProcurementData::Bill(x) => (
            x.supplier_id.clone(),
            true,
            x.lines.iter().map(|l| l.id.clone()).collect(),
        ),
        _ => return Err(anyhow!("Return source must be a receipt or bill")),
    };
    if let Some(prior) = s
        .accounting
        .procurement
        .records
        .iter()
        .find_map(|r| match &r.command {
            ProcurementCommand::Return(x)
                if x.context.company_id == ctx.company_id
                    && x.context.source_id == ctx.source_id =>
            {
                Some(x)
            }
            _ => None,
        })
    {
        let mut previous = prior.context.clone();
        previous.request_key = ctx.request_key.clone();
        ensure!(
            previous == ctx
                && prior.return_reference == p["reference"]
                && prior.supplier_id == supplier
                && prior
                    .lines
                    .iter()
                    .all(|l| ids.contains(l.bill_line_id.as_ref().unwrap_or(&l.receipt_line_id))),
            "Posted return retry has different contents"
        );
        retry_selection(
            p,
            prior.lines.iter().map(|l| {
                (
                    l.bill_line_id.as_deref().unwrap_or(&l.receipt_line_id),
                    l.quantity_thousandths,
                )
            }),
        )?;
        let mut repeat = prior.clone();
        repeat.context = ctx;
        return Ok(ProcurementCommand::Return(repeat));
    }
    let mut total = Money::ZERO;
    let lines = selected_lines(s, p, &r.id, "purchase_return")?
        .into_iter()
        .map(|(l, q)| -> Result<_> {
            let value = l.value(q, false)?;
            total = total.checked_add(value)?;
            Ok(PurchaseReturnLine {
                id: format!("{}-{}", ctx.source_id, l.id),
                receipt_line_id: l.receipt_line_id.unwrap(),
                bill_line_id: l.bill_line_id,
                quantity_thousandths: q,
                supplied_value: value,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ProcurementCommand::Return(ReturnPurchase {
        return_id: format!("{}-return", ctx.source_id),
        credit_id: billed.then(|| format!("{}-credit", ctx.source_id)),
        credit_reference: billed.then(|| format!("C-{reference}")),
        context: ctx,
        supplier_id: supplier,
        return_reference: string(p, "reference")?.into(),
        document_date: string(p, "date")?.into(),
        lines,
        supplied_total: total,
    }))
}

fn fulfill(s: &State, p: &Value) -> Result<SalesCommand> {
    if let Some(prior) = s
        .accounting
        .sales
        .records
        .iter()
        .find_map(|r| match &r.command {
            SalesCommand::Fulfill(x)
                if x.context.company_id == p["company_id"]
                    && x.context.source_id == p["source_id"] =>
            {
                Some(x)
            }
            _ => None,
        })
    {
        let ctx = context(s, p)?;
        let mut previous = prior.context.clone();
        previous.request_key = ctx.request_key.clone();
        ensure!(
            previous == ctx
                && prior.order_id == p["order_id"]
                && prior.invoice_reference == p["reference"]
                && prior.due_date == p["due_date"],
            "Posted invoice retry has different contents"
        );
        retry_selection(
            p,
            prior
                .lines
                .iter()
                .map(|l| (l.order_line_id.as_str(), l.quantity_thousandths)),
        )?;
        let mut repeat = prior.clone();
        repeat.context = ctx;
        return Ok(SalesCommand::Fulfill(repeat));
    }
    let order = s.accounting.sales_document(string(p, "order_id")?)?;
    ensure!(
        order.command.context().company_id == string(p, "company_id")?,
        "Wrong company"
    );
    let mut total = Money::ZERO;
    let lines = selected_lines(s, p, &order.id, "fulfill")?
        .into_iter()
        .map(|(l, q)| -> Result<_> {
            total = total.checked_add(l.value(q, true)?)?;
            Ok(FulfillLine {
                id: format!("{}-{}", string(p, "source_id")?, l.id),
                order_line_id: l.id,
                warehouse_id: l.warehouse_id,
                unit_id: l.unit_id,
                quantity_thousandths: q,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(SalesCommand::Fulfill(FulfillOrder {
        context: context(s, p)?,
        order_id: order.id.clone(),
        delivery_id: format!("{}-delivery", string(p, "source_id")?),
        invoice_id: format!("{}-invoice", string(p, "source_id")?),
        delivery_reference: format!("D-{}", string(p, "reference")?),
        invoice_reference: string(p, "reference")?.into(),
        document_date: string(p, "date")?.into(),
        due_date: string(p, "due_date")?.into(),
        lines,
        supplied_total: total,
    }))
}
fn receive(s: &State, p: &Value) -> Result<ProcurementCommand> {
    if let Some(prior) = s
        .accounting
        .procurement
        .records
        .iter()
        .find_map(|r| match &r.command {
            ProcurementCommand::Receive(x)
                if x.context.company_id == p["company_id"]
                    && x.context.source_id == p["source_id"] =>
            {
                Some(x)
            }
            _ => None,
        })
    {
        let ctx = context(s, p)?;
        let mut previous = prior.context.clone();
        previous.request_key = ctx.request_key.clone();
        ensure!(
            previous == ctx && prior.order_id == p["order_id"] && prior.reference == p["reference"],
            "Posted receipt retry has different contents"
        );
        retry_selection(
            p,
            prior
                .lines
                .iter()
                .map(|l| (l.order_line_id.as_str(), l.quantity_thousandths)),
        )?;
        let mut repeat = prior.clone();
        repeat.context = ctx;
        return Ok(ProcurementCommand::Receive(repeat));
    }
    let r = s.accounting.procurement_document(string(p, "order_id")?)?;
    let ProcurementCommand::Order(o) = &r.command else {
        return Err(anyhow!("Not a purchase order"));
    };
    ensure!(
        o.context.company_id == string(p, "company_id")?,
        "Wrong company"
    );
    let mut total = Money::ZERO;
    let lines = selected_lines(s, p, &r.id, "receive")?
        .into_iter()
        .map(|(l, q)| -> Result<_> {
            total = total.checked_add(l.value(q, false)?)?;
            Ok(ReceiveLine {
                id: format!("{}-{}", string(p, "source_id")?, l.id),
                order_line_id: l.id,
                warehouse_id: l.warehouse_id,
                unit_id: l.unit_id,
                quantity_thousandths: q,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ProcurementCommand::Receive(ReceiveOrder {
        context: context(s, p)?,
        order_id: r.id.clone(),
        reference: string(p, "reference")?.into(),
        document_date: string(p, "date")?.into(),
        lines,
        supplied_total: total,
    }))
}
fn bill(s: &State, p: &Value) -> Result<ProcurementCommand> {
    if let Some(prior) = s
        .accounting
        .procurement
        .records
        .iter()
        .find_map(|r| match &r.command {
            ProcurementCommand::Bill(x)
                if x.context.company_id == p["company_id"]
                    && x.context.source_id == p["source_id"] =>
            {
                Some(x)
            }
            _ => None,
        })
    {
        let ctx = context(s, p)?;
        let mut previous = prior.context.clone();
        previous.request_key = ctx.request_key.clone();
        let receipt = s
            .accounting
            .procurement_document(string(p, "receipt_id")?)?;
        let ProcurementData::Receive(source) = &receipt.data else {
            return Err(anyhow!("Not a receipt"));
        };
        ensure!(
            previous == ctx
                && prior.reference == p["reference"]
                && prior.due_date == p["due_date"]
                && prior
                    .lines
                    .iter()
                    .all(|l| source.lines.iter().any(|x| x.id == l.receipt_line_id)),
            "Posted bill retry has different contents"
        );
        retry_selection(
            p,
            prior
                .lines
                .iter()
                .map(|l| (l.receipt_line_id.as_str(), l.quantity_thousandths)),
        )?;
        let mut repeat = prior.clone();
        repeat.context = ctx;
        return Ok(ProcurementCommand::Bill(repeat));
    }
    let r = s
        .accounting
        .procurement_document(string(p, "receipt_id")?)?;
    let ProcurementData::Receive(receipt) = &r.data else {
        return Err(anyhow!("Not a receipt"));
    };
    ensure!(
        r.command.context().company_id == string(p, "company_id")?,
        "Wrong company"
    );
    let mut total = Money::ZERO;
    let lines = selected_lines(s, p, &r.id, "bill")?
        .into_iter()
        .map(|(l, q)| -> Result<_> {
            let value = l.value(q, false)?;
            total = total.checked_add(value)?;
            Ok(BillLine {
                id: format!("{}-{}", string(p, "source_id")?, l.id),
                receipt_line_id: l.id,
                quantity_thousandths: q,
                supplied_value: value,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ProcurementCommand::Bill(MatchBill {
        context: context(s, p)?,
        supplier_id: receipt.supplier_id.clone(),
        reference: string(p, "reference")?.into(),
        document_date: string(p, "date")?.into(),
        due_date: string(p, "due_date")?.into(),
        lines,
        supplied_total: total,
    }))
}
fn report(s: &State, p: &Value) -> Result<Value> {
    let c = company(s, p)?;
    let from = string(p, "from")?;
    let to = string(p, "to")?;
    civil_date(from)?;
    civil_date(to)?;
    ensure!(from <= to, "Start date must precede end date");
    let tb = s.accounting.trial_balance(&c.id, to)?;
    let mut rows = vec![];
    let mut revenue = Money::ZERO;
    let mut expense = Money::ZERO;
    for account in s
        .accounting
        .accounts
        .iter()
        .filter(|a| a.company_id == c.id)
    {
        let mut net = Money::ZERO;
        for line in s
            .accounting
            .journals
            .iter()
            .filter(|j| j.company_id == c.id && j.date.as_str() >= from && j.date.as_str() <= to)
            .flat_map(|j| &j.lines)
            .filter(|l| l.account.id == account.id)
        {
            net = net.checked_add(line.debit.checked_sub(line.credit)?)?;
        }
        if account.classification == AccountClass::Revenue {
            let v = Money::ZERO.checked_sub(net)?;
            revenue = revenue.checked_add(v)?;
            rows.push(json!({"account_id":account.id,"code":account.code,"name":account.name,"classification":"Revenue","amount":v}));
        }
        if account.classification == AccountClass::Expense {
            expense = expense.checked_add(net)?;
            rows.push(json!({"account_id":account.id,"code":account.code,"name":account.name,"classification":"Expense","amount":net}));
        }
    }
    let reconciliation = if s
        .accounting
        .core
        .openings
        .iter()
        .any(|o| o.command.company_id == c.id && o.command.as_of.as_str() <= to)
    {
        Some(s.accounting.sales_reconcile(&c.id, to)?)
    } else {
        None
    };
    let balance = |role: CoreRole| -> Result<Money> {
        let account = s
            .accounting
            .core
            .setups
            .iter()
            .find(|x| x.command.company_id == c.id)
            .and_then(|x| x.account_id(role).ok());
        if let Some(row) = tb
            .rows
            .iter()
            .find(|r| Some(r.account_id.as_str()) == account)
        {
            if role.normal_balance() == BalanceSide::Debit {
                row.debit.checked_sub(row.credit)
            } else {
                row.credit.checked_sub(row.debit)
            }
        } else {
            Ok(Money::ZERO)
        }
    };
    let mut party_balances = std::collections::BTreeMap::<String, Money>::new();
    if let Some(detail) = &reconciliation {
        for item in &detail.receivables {
            let entry = party_balances
                .entry(format!("customer:{}", item.party_id))
                .or_insert(Money::ZERO);
            *entry = entry.checked_add(item.remaining)?;
        }
        for item in &detail.payables {
            let entry = party_balances
                .entry(format!("vendor:{}", item.party_id))
                .or_insert(Money::ZERO);
            *entry = entry.checked_add(item.remaining)?;
        }
    }
    Ok(
        json!({"company_id":c.id,"from":from,"to":to,"trial":tb,"profit_loss":{"rows":rows,"revenue":revenue,"expenses":expense,"profit":revenue.checked_sub(expense)?},"position":{"cash":balance(CoreRole::Cash)?.checked_add(balance(CoreRole::Bank)?)?,"receivables":balance(CoreRole::Receivables)?,"payables":balance(CoreRole::Payables)?},"party_balances":party_balances,"reconciliation":reconciliation}),
    )
}
fn workspace(s: &State, p: &Value) -> Result<Value> {
    let c = p["company_id"].as_str().unwrap_or("");
    if !c.is_empty() {
        company(s, p)?;
    }
    let a = &s.accounting;
    let resources = if a.core.openings.iter().any(|o| o.command.company_id == c) {
        Some(a.procurement_resources(c)?)
    } else {
        None
    };
    let journal_count = a.journals.iter().filter(|j| j.company_id == c).count();
    let sales_count = a
        .sales
        .records
        .iter()
        .filter(|r| r.command.context().company_id == c)
        .count();
    let purchase_count = a
        .procurement
        .records
        .iter()
        .filter(|r| r.command.context().company_id == c)
        .count();
    let offset = p["offset"]
        .as_str()
        .unwrap_or("0")
        .parse::<usize>()?
        .min(1_000_000);
    Ok(
        json!({"companies":a.companies,"periods":a.periods.iter().filter(|x|x.company_id==c).collect::<Vec<_>>(),"accounts":a.accounts.iter().filter(|x|x.company_id==c).collect::<Vec<_>>(),"setups":a.core.setups.iter().filter(|x|x.command.company_id==c).collect::<Vec<_>>(),"initialized":a.core.openings.iter().any(|o|o.command.company_id==c),"resources":resources,"journals":a.journals.iter().filter(|j|j.company_id==c).rev().skip(offset).take(100).collect::<Vec<_>>(),"journal_count":journal_count,"sales":a.sales.records.iter().filter(|r|r.command.context().company_id==c).rev().skip(offset).take(100).collect::<Vec<_>>(),"sales_count":sales_count,"purchases":a.procurement.records.iter().filter(|r|r.command.context().company_id==c).rev().skip(offset).take(100).collect::<Vec<_>>(),"purchase_count":purchase_count,"drafts":s.sales_drafts.iter().filter(|d|d.company_id==c).take(100).collect::<Vec<_>>(),"audit":a.audit.iter().filter(|x|x.company_id==c).rev().take(100).collect::<Vec<_>>(),"settings":s.settings,"capabilities":{"payments":false,"banking":false,"attachments":false,"service_invoices":false}}),
    )
}

pub fn dispatch(directory: &Path, action: &str, p: Value) -> Result<Value> {
    let mut store = Store::open(directory)?;
    let key = p["request_key"].as_str().unwrap_or("");
    let replay = store
        .state
        .accounting
        .requests
        .iter()
        .any(|r| r.command.request_key == key)
        || store
            .state
            .accounting
            .sales
            .requests
            .iter()
            .any(|r| r.request_key == key)
        || store
            .state
            .accounting
            .procurement
            .requests
            .iter()
            .any(|r| r.request_key == key)
        || store
            .state
            .accounting
            .core
            .zero_requests
            .iter()
            .any(|r| r.request_key == key);
    if (action.ends_with("_post") || action == "journal_reverse")
        && !replay
        && p.get("expected_generation").is_some()
    {
        let expected: i64 = string(&p, "expected_generation")?.parse()?;
        ensure!(
            expected == store.local_generation(),
            "Books changed since review. Refresh and review the financial effect again."
        );
    }
    let mut out = match action {
        "workspace" => workspace(&store.state, &p)?,
        "report" => report(&store.state, &p)?,
        "setup_company" => {
            let result = store.change(|s| {
                s.accounting.setup_core_company(NewCoreCompany {
                    name: string(&p, "name")?.into(),
                    fiscal_name: string(&p, "fiscal_name")?.into(),
                    start_date: string(&p, "start_date")?.into(),
                    end_date: string(&p, "end_date")?.into(),
                    timezone: if optional(&p, "timezone", "AsiaTehran") == "Utc" {
                        CompanyTimezone::Utc
                    } else {
                        CompanyTimezone::AsiaTehran
                    },
                    chart: default_chart(),
                    actor: actor(&p).into(),
                })
            })?;
            json!(result)
        }
        "initialize_zero" => {
            ensure!(
                p["confirmed"] == true,
                "Zero balances require explicit confirmation"
            );
            let cmd = opening_zero(&store.state, &p)?;
            json!(store.change(|s| s.accounting.post_opening(cmd))?)
        }
        "opening_csv_preview" | "opening_csv_post" => {
            let cmd = crate::core_import::opening_csv(
                &store.state.accounting,
                string(&p, "company_id")?,
                string(&p, "csv")?,
                actor(&p),
                string(&p, "request_key")?,
            )?;
            if action.ends_with("preview") {
                json!(store.state.accounting.opening_preview(&cmd)?)
            } else {
                json!(store.change(|s| s.accounting.post_opening(cmd))?)
            }
        }
        "resource" => {
            let ctx = context(&store.state, &p)?;
            let company = ctx.company_id.clone();
            let rid = string(&p, "id")?.to_owned();
            let name = string(&p, "name")?.to_owned();
            let mut cmd = RegisterResources {
                context: ctx,
                parties: vec![],
                units: vec![],
                items: vec![],
                warehouses: vec![],
            };
            match string(&p, "kind")? {
                "customer" | "vendor" => cmd.parties.push(CoreParty {
                    id: rid,
                    company_id: company,
                    name,
                    customer: string(&p, "kind")? == "customer",
                    supplier: string(&p, "kind")? == "vendor",
                }),
                "unit" => cmd.units.push(CoreUnit {
                    id: rid,
                    company_id: company,
                    name,
                }),
                "warehouse" => cmd.warehouses.push(CoreWarehouse {
                    id: rid,
                    company_id: company,
                    name,
                }),
                "item" => cmd.items.push(CoreItem {
                    id: rid,
                    company_id: company,
                    sku: string(&p, "sku")?.into(),
                    name,
                    base_unit_id: string(&p, "unit_id")?.into(),
                    indivisible: p["indivisible"] == true,
                    packs: vec![],
                }),
                _ => return Err(anyhow!("Unknown resource kind")),
            }
            json!(store.change(|s| {
                s.accounting
                    .post_procurement_command(ProcurementCommand::Resources(cmd))
            })?)
        }
        "draft_save" => {
            let draft = raw_draft(&p["draft"])?;
            let expected = p["expected_revision"]
                .as_str()
                .map(str::parse)
                .transpose()?;
            json!(store.change(|s| s.save_sales_draft(draft, expected))?)
        }
        "draft_delete" => {
            store.change(|s| s.remove_sales_draft(string(&p, "id")?, integer(&p, "revision")?))?;
            json!(true)
        }
        "draft_preview" => {
            let mut s = store.state.clone();
            let mut draft = raw_draft(&p["draft"])?;
            let mut total = Money::ZERO;
            for l in &draft.lines {
                let q = quantity(&l.quantity)?;
                let price = Money::parse(&l.unit_price, draft.display_unit)?;
                let discount = Money::parse(&l.line_discount, draft.display_unit)?;
                total = total.checked_add(
                    Money::from_rials(positive_half_up(
                        i128::from(price.rials()) * i128::from(q),
                        1000,
                    )?)
                    .checked_sub(discount)?,
                )?;
            }
            total =
                total.checked_sub(Money::parse(&draft.document_discount, draft.display_unit)?)?;
            draft.supplied_total = total.display(draft.display_unit);
            let expected = s
                .sales_drafts
                .iter()
                .find(|d| d.id == draft.id)
                .map(|d| d.revision);
            let rev = s.save_sales_draft(draft.clone(), expected)?;
            let order = s.sales_order_from_draft(&draft.id, rev, "preview", actor(&p))?;
            json!({"preview":s.accounting.preview_sales(&SalesCommand::Order(order))?,"supplied_total":draft.supplied_total})
        }
        "draft_post" => json!(store.change(|s| s.post_sales_from_draft(
            string(&p, "id")?,
            integer(&p, "revision")?,
            string(&p, "request_key")?,
            actor(&p)
        ))?),
        "purchase_preview" | "purchase_post" => {
            let cmd = purchase_order(&store.state, &p)?;
            if action.ends_with("preview") {
                json!(store.state.accounting.preview_procurement(&cmd)?)
            } else {
                json!(store.change(|s| s.accounting.post_procurement_command(cmd))?)
            }
        }
        "receive_preview" | "receive_post" => {
            let cmd = receive(&store.state, &p)?;
            if action.ends_with("preview") {
                json!(store.state.accounting.preview_procurement(&cmd)?)
            } else {
                json!(store.change(|s| s.accounting.post_procurement_command(cmd))?)
            }
        }
        "bill_preview" | "bill_post" => {
            let cmd = bill(&store.state, &p)?;
            if action.ends_with("preview") {
                json!(store.state.accounting.preview_procurement(&cmd)?)
            } else {
                json!(store.change(|s| s.accounting.post_procurement_command(cmd))?)
            }
        }
        "fulfill_preview" | "fulfill_post" => {
            let cmd = fulfill(&store.state, &p)?;
            if action.ends_with("preview") {
                json!(store.state.accounting.preview_sales(&cmd)?)
            } else {
                json!(store.change(|s| s.accounting.post_sales_command(cmd))?)
            }
        }
        "operation_options" => {
            company(&store.state, &p)?;
            json!({"lines": operation_lines(&store.state,string(&p,"company_id")?,string(&p,"record_id")?,string(&p,"operation")?)?.into_iter().filter(|l| l.remaining_quantity > 0).collect::<Vec<_>>()})
        }
        "sale_return_preview" | "sale_return_post" => {
            let cmd = sale_return(&store.state, &p)?;
            if action.ends_with("preview") {
                json!(store.state.accounting.preview_sales(&cmd)?)
            } else {
                json!(store.change(|s| s.accounting.post_sales_command(cmd))?)
            }
        }
        "purchase_return_preview" | "purchase_return_post" => {
            let cmd = purchase_return(&store.state, &p)?;
            if action.ends_with("preview") {
                json!(store.state.accounting.preview_procurement(&cmd)?)
            } else {
                json!(store.change(|s| s.accounting.post_procurement_command(cmd))?)
            }
        }
        "transfer_preview" | "transfer_post" => {
            let cmd = ProcurementCommand::Transfer(TransferStock {
                context: context(&store.state, &p)?,
                item_id: string(&p, "item_id")?.into(),
                source_warehouse_id: string(&p, "source_warehouse_id")?.into(),
                target_warehouse_id: string(&p, "target_warehouse_id")?.into(),
                unit_id: string(&p, "unit_id")?.into(),
                quantity_thousandths: quantity(string(&p, "quantity")?)?,
            });
            if action.ends_with("preview") {
                json!(store.state.accounting.preview_procurement(&cmd)?)
            } else {
                json!(store.change(|s| s.accounting.post_procurement_command(cmd))?)
            }
        }
        "journal_preview" | "journal_post" => {
            let cmd = manual(&store.state, &p)?;
            if action.ends_with("preview") {
                let mut s = store.state.clone();
                let jid = s.accounting.post(cmd)?;
                json!(s.accounting.journals.iter().find(|j| j.id == jid))
            } else {
                json!(store.change(|s| s.accounting.post(cmd))?)
            }
        }
        "period_lock" => {
            company(&store.state, &p)?;
            ensure!(
                store
                    .state
                    .accounting
                    .periods
                    .iter()
                    .any(|x| x.id == p["period_id"] && x.company_id == p["company_id"]),
                "Wrong company period"
            );
            store.change(|s| {
                s.accounting.set_period_lock(
                    string(&p, "period_id")?,
                    p["locked"] == true,
                    actor(&p),
                    string(&p, "reason")?,
                )
            })?;
            json!(true)
        }
        "journal_reverse" | "journal_reverse_preview" => {
            company(&store.state, &p)?;
            ensure!(
                store
                    .state
                    .accounting
                    .journals
                    .iter()
                    .any(|x| x.id == p["journal_id"] && x.company_id == p["company_id"]),
                "Wrong company journal"
            );
            if action.ends_with("_preview") {
                let mut snapshot = store.state.clone();
                let id = snapshot.accounting.reverse(
                    string(&p, "journal_id")?,
                    string(&p, "period_id")?,
                    string(&p, "date")?,
                    string(&p, "request_key")?,
                    actor(&p),
                    string(&p, "reason")?,
                )?;
                json!(snapshot.accounting.journals.iter().find(|j| j.id == id))
            } else {
                json!(store.change(|s| s.accounting.reverse(
                    string(&p, "journal_id")?,
                    string(&p, "period_id")?,
                    string(&p, "date")?,
                    string(&p, "request_key")?,
                    actor(&p),
                    string(&p, "reason")?
                ))?)
            }
        }
        "settings" => {
            let mut v = p.clone();
            for k in [
                "next_number",
                "default_validity_days",
                "default_discount_rial",
                "tax_basis_points",
            ] {
                v[k] = serde_json::from_str(string(&p, k)?)?;
            }
            let settings: Settings = serde_json::from_value(v)?;
            store.change(|s| {
                s.settings = settings;
                Ok(())
            })?;
            json!(true)
        }
        "detail" => {
            let c = string(&p, "company_id")?;
            let rid = string(&p, "id")?;
            if let Some(j) = store
                .state
                .accounting
                .journals
                .iter()
                .find(|x| x.id == rid && x.company_id == c)
            {
                json!(j)
            } else if let Some(r) = store
                .state
                .accounting
                .sales
                .records
                .iter()
                .find(|x| x.id == rid && x.command.context().company_id == c)
            {
                json!(r)
            } else if let Some(r) = store
                .state
                .accounting
                .procurement
                .records
                .iter()
                .find(|x| x.id == rid && x.command.context().company_id == c)
            {
                json!(r)
            } else {
                return Err(anyhow!("Record not found in this company"));
            }
        }
        "ledger" => {
            let c = company(&store.state, &p)?;
            let aid = string(&p, "account_id")?;
            let from = string(&p, "from")?;
            let to = string(&p, "to")?;
            civil_date(from)?;
            civil_date(to)?;
            ensure!(from <= to, "Invalid date range");
            let offset = optional(&p, "offset", "0").parse::<usize>()?;
            let lines=store.state.accounting.journals.iter().filter(|j|j.company_id==c.id&&j.date.as_str()>=from&&j.date.as_str()<=to).flat_map(|j|j.lines.iter().filter(move|l|l.account.id==aid).map(move|l|json!({"journal_id":j.id,"date":j.date,"reference":j.source_id,"description":l.description,"debit":l.debit,"credit":l.credit}))).collect::<Vec<_>>();
            json!({"count":lines.len(),"rows":lines.into_iter().skip(offset).take(100).collect::<Vec<_>>()})
        }
        "report_csv" => {
            let r = report(&store.state, &p)?;
            let mut writer = csv::Writer::from_writer(vec![]);
            writer.write_record(["Account code", "Account name", "Debit IRR", "Credit IRR"])?;
            for row in r["trial"]["rows"]
                .as_array()
                .ok_or_else(|| anyhow!("Invalid report"))?
            {
                writer.write_record([
                    csv_safe(row["code"].as_str().unwrap_or("")),
                    csv_safe(row["name"].as_str().unwrap_or("")),
                    row["debit"].to_string(),
                    row["credit"].to_string(),
                ])?;
            }
            json!({"filename":format!("trial-balance-{}.csv",string(&p,"to")?),"content":format!("\u{feff}{}",String::from_utf8(writer.into_inner()?)?)})
        }
        "profit_csv" => {
            let r = report(&store.state, &p)?;
            let mut writer = csv::Writer::from_writer(vec![]);
            writer.write_record([
                "Account code",
                "Account name",
                "Classification",
                "Amount IRR",
            ])?;
            for row in r["profit_loss"]["rows"]
                .as_array()
                .ok_or_else(|| anyhow!("Invalid report"))?
            {
                writer.write_record([
                    csv_safe(row["code"].as_str().unwrap_or("")),
                    csv_safe(row["name"].as_str().unwrap_or("")),
                    row["classification"].as_str().unwrap_or("").into(),
                    row["amount"].to_string(),
                ])?;
            }
            writer.write_record([
                "",
                "Net profit",
                "",
                &r["profit_loss"]["profit"].to_string(),
            ])?;
            json!({"filename":format!("profit-loss-{}.csv",string(&p,"to")?),"content":format!("\u{feff}{}",String::from_utf8(writer.into_inner()?)?)})
        }
        _ => return Err(anyhow!("Unsupported command: {action}")),
    };
    if let Value::Object(ref mut object) = out {
        object.insert("_generation".into(), json!(store.local_generation()));
    }
    journal_totals(&mut out)?;
    Ok(wire(out))
}
fn csv_safe(v: &str) -> String {
    if v.trim_start_matches(|c: char| c.is_whitespace() || c.is_control() || c == '\u{feff}')
        .starts_with(['=', '+', '-', '@'])
    {
        format!("'{v}")
    } else {
        v.into()
    }
}
/// Explicit QA/demo seed, never invoked by the production dispatcher.
pub fn seed_fixture(directory: &Path) -> Result<()> {
    let mut store = Store::open(directory)?;
    ensure!(
        store.state.accounting.is_empty(),
        "Fixture requires an empty profile"
    );
    let envelope: Value = serde_json::from_str(include_str!(
        "../docs/assets/accounting-m3/restore/core-v4.json"
    ))?;
    let mut state: State = serde_json::from_value(envelope["state"].clone())?;
    state.schema = crate::domain::CURRENT_SCHEMA;
    store.change(|s| {
        *s = state;
        Ok(())
    })?;
    Ok(())
}
pub fn request_id() -> String {
    id()
}
