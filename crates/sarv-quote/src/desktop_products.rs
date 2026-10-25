use super::*;
use crate::product_profile::ProductProfile;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn item(s: &State, p: &Value) -> Result<CoreItem> {
    let c = company(s, p)?;
    s.accounting
        .procurement_resources(&c.id)?
        .items
        .into_iter()
        .find(|i| i.id == p["item_id"])
        .ok_or_else(|| anyhow!("Product not found in this business"))
}
pub(super) fn profile(raw: &Value, company: &str) -> Result<ProductProfile> {
    let mut p = raw.clone();
    let object = p
        .as_object_mut()
        .ok_or_else(|| anyhow!("Invalid product profile"))?;
    ensure!(
        object
            .get("company_id")
            .and_then(Value::as_str)
            .is_none_or(|id| id.is_empty() || id == company),
        "Wrong product company"
    );
    object.insert("company_id".into(), json!(company));
    for key in ["purchase_price", "sale_price"] {
        if let Some(value) = object.get(key) {
            let parsed = if value.is_null() || value.as_str() == Some("") {
                None
            } else {
                Some(Money::parse(
                    value
                        .as_str()
                        .ok_or_else(|| anyhow!("Price must be an exact string"))?,
                    MoneyUnit::Rial,
                )?)
            };
            object.insert(key.into(), json!(parsed));
        }
    }
    if let Some(value) = object.remove("minimum_stock") {
        let raw = value
            .as_str()
            .ok_or_else(|| anyhow!("Invalid minimum stock"))?;
        let normalized = crate::domain::digits(raw);
        let parts: Vec<_> = normalized.trim().split('.').collect();
        let zero = parts.len() <= 2
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|c| c == '0'))
            && parts.get(1).is_none_or(|p| p.len() <= 3);
        let q = if normalized.trim().is_empty() || zero {
            0
        } else {
            quantity(raw)?
        };
        object.insert("minimum_stock_thousandths".into(), json!(q));
    } else if let Some(value) = object
        .get("minimum_stock_thousandths")
        .and_then(Value::as_str)
    {
        let q = value.parse::<i64>()?;
        object.insert("minimum_stock_thousandths".into(), json!(q));
    }
    let profile: ProductProfile = serde_json::from_value(p)?;
    profile.validate()?;
    Ok(profile)
}
fn per_unit(value: Money, quantity: i64) -> Result<Option<String>> {
    if quantity <= 0 {
        return Ok(None);
    }
    Ok(Some(
        ((i128::from(value.rials()) * 1000 + i128::from(quantity) / 2) / i128::from(quantity))
            .to_string(),
    ))
}
fn add_quantity(a: &mut i64, b: i64) -> Result<()> {
    *a = a
        .checked_add(b)
        .ok_or_else(|| anyhow!("Product quantity overflow"))?;
    Ok(())
}
fn month(s: &State, date: &str) -> Result<String> {
    let calendar = if s.settings.date_display == DateDisplay::Jalali {
        crate::calendar::Calendar::Jalali
    } else {
        crate::calendar::Calendar::Gregorian
    };
    Ok(crate::calendar::format_date(
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")?,
        calendar,
    )?[..7]
        .to_owned())
}
pub(super) fn detail(s: &State, p: &Value) -> Result<Value> {
    let item = item(s, p)?;
    let asof = string(p, "as_of")?;
    chrono::NaiveDate::parse_from_str(asof, "%Y-%m-%d")?;
    let profile = s
        .product_profiles
        .get(&item.id)
        .cloned()
        .unwrap_or(ProductProfile {
            company_id: item.company_id.clone(),
            ..Default::default()
        });
    let stock: Vec<_> = s
        .accounting
        .procurement_reconcile(&item.company_id, asof)?
        .stock
        .into_iter()
        .filter(|pool| pool.item_id == item.id)
        .collect();
    let mut stock_quantity = 0;
    let mut stock_value = Money::ZERO;
    for pool in &stock {
        add_quantity(&mut stock_quantity, pool.quantity_thousandths)?;
        stock_value = stock_value.checked_add(pool.value)?;
    }
    let (mut net_sales, mut cost, mut returns_value, mut purchase_value) =
        (Money::ZERO, Money::ZERO, Money::ZERO, Money::ZERO);
    let (mut sold, mut returned, mut purchased, mut purchase_returns) = (0, 0, 0, 0);
    let mut monthly = BTreeMap::<String, (Money, Money)>::new();
    let mut customers = BTreeMap::<String, (i64, Money, Money)>::new();
    let mut suppliers = BTreeMap::<String, (i64, Money)>::new();
    let mut records = Vec::<(String, u64, Value)>::new();
    let mut last_sale: Option<(String, u64, Money, i64)> = None;
    let mut last_purchase: Option<(String, u64, Money, i64)> = None;
    for record in s.accounting.sales.records.iter().filter(|r| {
        r.command.context().company_id == item.company_id
            && r.command.context().date.as_str() <= asof
    }) {
        let date = &record.command.context().date;
        let mut revenue = Money::ZERO;
        let mut carried = Money::ZERO;
        let mut qty = 0;
        let mut customer = None;
        let matched = match &record.data {
            SalesData::Order(d) => d.lines.iter().any(|l| l.item.id == item.id),
            SalesData::Fulfill(d) => {
                for l in d.lines.iter().filter(|l| l.item.id == item.id) {
                    revenue = revenue.checked_add(l.net)?;
                    carried = carried.checked_add(l.cost)?;
                    add_quantity(&mut qty, l.quantity_thousandths)?;
                }
                if qty > 0 {
                    add_quantity(&mut sold, qty)?;
                    customer = Some(d.customer.id.clone());
                    if last_sale
                        .as_ref()
                        .is_none_or(|r| (date, record.sequence) > (&r.0, r.1))
                    {
                        last_sale = Some((date.clone(), record.sequence, revenue, qty));
                    }
                }
                qty > 0
            }
            SalesData::Return(d) => {
                for l in d.lines.iter().filter(|l| l.item_id == item.id) {
                    revenue = revenue.checked_sub(l.net)?;
                    carried = carried.checked_sub(l.cost)?;
                    add_quantity(&mut qty, l.quantity_thousandths)?;
                }
                if qty > 0 {
                    add_quantity(&mut returned, qty)?;
                    returns_value = returns_value.checked_sub(revenue)?;
                    qty = -qty;
                    customer = Some(d.customer_id.clone());
                }
                qty != 0
            }
        };
        if matched {
            records.push((date.clone(), record.sequence, json!(record)));
        }
        if let Some(customer) = customer {
            net_sales = net_sales.checked_add(revenue)?;
            cost = cost.checked_add(carried)?;
            let m = monthly
                .entry(month(s, date)?)
                .or_insert((Money::ZERO, Money::ZERO));
            m.0 = m.0.checked_add(revenue)?;
            m.1 = m.1.checked_add(carried)?;
            let c = customers
                .entry(customer)
                .or_insert((0, Money::ZERO, Money::ZERO));
            add_quantity(&mut c.0, qty)?;
            c.1 = c.1.checked_add(revenue)?;
            c.2 = c.2.checked_add(carried)?;
        }
    }
    let mut receipt_lines: BTreeSet<_> = s
        .accounting
        .procurement
        .records
        .iter()
        .filter_map(|r| {
            if let ProcurementData::Receive(d) = &r.data {
                Some(d)
            } else {
                None
            }
        })
        .flat_map(|d| {
            d.lines
                .iter()
                .filter(|l| l.item_id == item.id)
                .map(|l| l.id.as_str())
        })
        .collect();
    for opening in s
        .accounting
        .core
        .openings
        .iter()
        .filter(|o| o.command.company_id == item.company_id)
    {
        receipt_lines.extend(
            opening
                .command
                .grni
                .iter()
                .filter(|g| g.item_id == item.id)
                .map(|g| g.id.as_str()),
        );
    }
    for record in s.accounting.procurement.records.iter().filter(|r| {
        r.command.context().company_id == item.company_id
            && r.command.context().date.as_str() <= asof
    }) {
        let date = &record.command.context().date;
        let mut value = Money::ZERO;
        let mut qty = 0;
        let mut supplier = None;
        let matched = match &record.data {
            ProcurementData::Resources(_) => false,
            ProcurementData::Order(d) => d.lines.iter().any(|l| l.item.id == item.id),
            ProcurementData::Receive(d) => {
                for l in d.lines.iter().filter(|l| l.item_id == item.id) {
                    value = value.checked_add(l.value)?;
                    add_quantity(&mut qty, l.quantity_thousandths)?;
                }
                if qty > 0 {
                    add_quantity(&mut purchased, qty)?;
                    supplier = Some(d.supplier_id.clone());
                    if last_purchase
                        .as_ref()
                        .is_none_or(|r| (date, record.sequence) > (&r.0, r.1))
                    {
                        last_purchase = Some((date.clone(), record.sequence, value, qty));
                    }
                }
                qty > 0
            }
            ProcurementData::Bill(d) => d
                .lines
                .iter()
                .any(|l| receipt_lines.contains(l.receipt_line_id.as_str())),
            ProcurementData::Return(d) => {
                for l in d.lines.iter().filter(|l| l.item_id == item.id) {
                    value = value.checked_sub(l.value)?;
                    add_quantity(&mut qty, l.quantity_thousandths)?;
                }
                if qty > 0 {
                    add_quantity(&mut purchase_returns, qty)?;
                    qty = -qty;
                    supplier = Some(d.supplier_id.clone());
                }
                qty != 0
            }
            ProcurementData::Transfer(d) => d.item_id == item.id,
        };
        if matched {
            records.push((date.clone(), record.sequence, json!(record)));
        }
        if let Some(supplier) = supplier {
            purchase_value = purchase_value.checked_add(value)?;
            let p = suppliers.entry(supplier).or_insert((0, Money::ZERO));
            add_quantity(&mut p.0, qty)?;
            p.1 = p.1.checked_add(value)?;
        }
    }
    records.sort_by(|a, b| (&b.0, b.1).cmp(&(&a.0, a.1)));
    let offset = p["offset"].as_str().unwrap_or("0").parse::<usize>()?;
    let estimated_profit = profile
        .sale_price
        .zip(profile.purchase_price)
        .map(|(sale, purchase)| sale.checked_sub(purchase))
        .transpose()?;
    let margin = profile
        .sale_price
        .zip(estimated_profit)
        .and_then(|(sale, profit)| {
            if sale.rials() > 0 {
                Some((i128::from(profit.rials()) * 10000 / i128::from(sale.rials())).to_string())
            } else {
                None
            }
        });
    let markup = profile
        .purchase_price
        .zip(estimated_profit)
        .and_then(|(purchase, profit)| {
            if purchase.rials() > 0 {
                Some(
                    (i128::from(profit.rials()) * 10000 / i128::from(purchase.rials())).to_string(),
                )
            } else {
                None
            }
        });
    Ok(
        json!({"item":item,"profile":profile,"as_of":asof,"stock":stock,"stats":{"stock_quantity":stock_quantity,"stock_value":stock_value,"average_stock_cost":per_unit(stock_value,stock_quantity)?,"low_stock":profile.minimum_stock_thousandths>0&&stock_quantity<=profile.minimum_stock_thousandths,"sold_quantity":sold.checked_sub(returned).ok_or_else(||anyhow!("Quantity overflow"))?,"returned_quantity":returned,"net_sales":net_sales,"cost_of_goods_sold":cost,"gross_profit":net_sales.checked_sub(cost)?,"returns_value":returns_value,"purchased_quantity":purchased.checked_sub(purchase_returns).ok_or_else(||anyhow!("Quantity overflow"))?,"purchase_returns_quantity":purchase_returns,"net_purchase_value":purchase_value,"last_purchase_price":last_purchase.map(|r|per_unit(r.2,r.3)).transpose()?.flatten(),"last_sale_price":last_sale.map(|r|per_unit(r.2,r.3)).transpose()?.flatten(),"estimated_unit_profit":estimated_profit,"margin_basis_points":margin,"markup_basis_points":markup},"monthly":monthly.into_iter().map(|(month,(sales,cost))|Ok(json!({"month":month,"net_sales":sales,"cost":cost,"profit":sales.checked_sub(cost)?}))).collect::<Result<Vec<_>>>()?,"customers":customers.into_iter().map(|(id,(quantity,sales,cost))|Ok(json!({"id":id,"quantity":quantity,"net_sales":sales,"profit":sales.checked_sub(cost)?}))).collect::<Result<Vec<_>>>()?,"suppliers":suppliers.into_iter().map(|(id,(quantity,value))|json!({"id":id,"quantity":quantity,"value":value})).collect::<Vec<_>>(),"records":records.iter().skip(offset).take(100).map(|r|&r.2).collect::<Vec<_>>(),"record_count":records.len()}),
    )
}
