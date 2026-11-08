use super::*;
use std::collections::BTreeMap;

pub(super) fn party(s: &State, p: &Value) -> Result<CoreParty> {
    let c = company(s, p)?;
    s.accounting
        .procurement_resources(&c.id)?
        .parties
        .into_iter()
        .find(|party| party.id == p["party_id"])
        .ok_or_else(|| anyhow!("Contact not found in this business"))
}
pub(super) fn contact_detail(s: &State, p: &Value) -> Result<Value> {
    let party = party(s, p)?;
    let asof = string(p, "as_of")?;
    chrono::NaiveDate::parse_from_str(asof, "%Y-%m-%d")?;
    let profile = s.party_profiles.get(&party.id).cloned().unwrap_or_else(|| {
        crate::contact_profile::PartyProfile {
            company_id: party.company_id.clone(),
            display_name: party.name.clone(),
            ..Default::default()
        }
    });
    let mut records = Vec::new();
    let mut orders = 0;
    let mut invoices = 0;
    let mut returns_count = 0;
    let mut gross = Money::ZERO;
    let mut returned = Money::ZERO;
    let mut cost = Money::ZERO;
    let mut monthly = BTreeMap::<String, Money>::new();
    let mut products = BTreeMap::<String, (String, i64, Money)>::new();
    let mut last_activity: Option<String> = None;
    for r in s.accounting.sales.records.iter().filter(|r| {
        r.command.context().company_id == party.company_id
            && r.command.context().date.as_str() <= asof
    }) {
        let owner = match &r.data {
            SalesData::Order(d) => &d.customer.id,
            SalesData::Fulfill(d) => &d.customer.id,
            SalesData::Return(d) => &d.customer_id,
        };
        if owner != &party.id {
            continue;
        }
        let date = r.command.context().date.clone();
        let calendar = if s.settings.date_display == DateDisplay::Jalali {
            crate::calendar::Calendar::Jalali
        } else {
            crate::calendar::Calendar::Gregorian
        };
        let displayed = crate::calendar::format_date(
            chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")?,
            calendar,
        )?;
        let month = displayed[..7].to_owned();
        last_activity = Some(last_activity.map_or(date.clone(), |d| d.max(date.clone())));
        match &r.data {
            SalesData::Order(_) => orders += 1,
            SalesData::Fulfill(d) => {
                invoices += 1;
                gross = gross.checked_add(d.total)?;
                cost = cost.checked_add(d.cost)?;
                let prior = monthly.entry(month.clone()).or_insert(Money::ZERO);
                *prior = prior.checked_add(d.total)?;
                for line in &d.lines {
                    let prior = products.entry(line.item.id.clone()).or_insert((
                        line.item.name.clone(),
                        0,
                        Money::ZERO,
                    ));
                    prior.1 = prior
                        .1
                        .checked_add(line.quantity_thousandths)
                        .ok_or_else(|| anyhow!("Quantity overflow"))?;
                    prior.2 = prior.2.checked_add(line.net)?;
                }
            }
            SalesData::Return(d) => {
                returns_count += 1;
                returned = returned.checked_add(d.total)?;
                cost = cost.checked_sub(d.cost)?;
                let prior = monthly.entry(month).or_insert(Money::ZERO);
                *prior = prior.checked_sub(d.total)?;
            }
        }
        records.push(json!(r));
    }
    let receivables: Vec<_> = s
        .accounting
        .sales_receivables(&party.company_id, asof)?
        .into_iter()
        .filter(|r| r.party_id == party.id)
        .collect();
    let payables: Vec<_> = s
        .accounting
        .procurement_reconcile(&party.company_id, asof)?
        .payables
        .into_iter()
        .filter(|r| r.party_id == party.id)
        .collect();
    let mut outstanding = Money::ZERO;
    let mut overdue = Money::ZERO;
    for item in &receivables {
        outstanding = outstanding.checked_add(item.remaining)?;
        if item.due_date.as_str() < asof {
            overdue = overdue.checked_add(item.remaining)?;
        }
    }
    let net = gross.checked_sub(returned)?;
    records.reverse();
    let offset = p["offset"].as_str().unwrap_or("0").parse::<usize>()?;
    Ok(
        json!({"party":party,"profile":profile,"as_of":asof,"stats":{"orders":orders,"invoices":invoices,"returns":returns_count,"gross_sales":gross,"returns_value":returned,"net_sales":net,"gross_profit":net.checked_sub(cost)?,"outstanding":outstanding,"overdue":overdue,"last_activity":last_activity},"records":records.iter().skip(offset).take(100).collect::<Vec<_>>(),"record_count":records.len(),"monthly":monthly.into_iter().map(|(month,net)|json!({"month":month,"net":net})).collect::<Vec<_>>(),"products":products.into_iter().map(|(id,(name,quantity,revenue))|json!({"id":id,"name":name,"quantity":quantity,"revenue":revenue})).collect::<Vec<_>>(),"receivables":receivables,"payables":payables}),
    )
}
pub(super) fn banking(s: &State, p: &Value) -> Result<Value> {
    let c = company(s, p)?;
    let from = string(p, "from")?;
    let to = string(p, "to")?;
    chrono::NaiveDate::parse_from_str(from, "%Y-%m-%d")?;
    chrono::NaiveDate::parse_from_str(to, "%Y-%m-%d")?;
    ensure!(from <= to, "Invalid banking date range");
    let setup = s
        .accounting
        .core
        .setups
        .iter()
        .find(|x| x.command.company_id == c.id)
        .ok_or_else(|| anyhow!("Company chart required"))?;
    let tb = s.accounting.trial_balance(&c.id, to)?;
    let mut accounts = Vec::new();
    for role in [CoreRole::Cash, CoreRole::Bank] {
        let mapping = setup
            .command
            .mappings
            .iter()
            .find(|m| m.role == role)
            .ok_or_else(|| anyhow!("Cash/bank mapping required"))?;
        let account = s
            .accounting
            .accounts
            .iter()
            .find(|a| a.id == mapping.account_id)
            .ok_or_else(|| anyhow!("Bank account missing"))?;
        let closing = tb
            .rows
            .iter()
            .find(|r| r.account_id == account.id)
            .map(|r| r.debit.checked_sub(r.credit))
            .transpose()?
            .unwrap_or(Money::ZERO);
        let mut opening = Money::ZERO;
        let mut incoming = Money::ZERO;
        let mut outgoing = Money::ZERO;
        for j in s
            .accounting
            .journals
            .iter()
            .filter(|j| j.company_id == c.id && j.date.as_str() <= to)
        {
            for line in j.lines.iter().filter(|l| l.account.id == account.id) {
                if j.date.as_str() < from {
                    opening = opening.checked_add(line.debit)?.checked_sub(line.credit)?;
                } else {
                    incoming = incoming.checked_add(line.debit)?;
                    outgoing = outgoing.checked_add(line.credit)?;
                }
            }
        }
        accounts.push(json!({"account":account,"role":role,"opening":opening,"incoming":incoming,"outgoing":outgoing,"balance":closing}));
    }
    let ids: Vec<_> = accounts
        .iter()
        .filter_map(|a| a["account"]["id"].as_str())
        .collect();
    let mut movements: Vec<_> = s
        .accounting
        .journals
        .iter()
        .filter(|j| {
            j.company_id == c.id
                && j.date.as_str() >= from
                && j.date.as_str() <= to
                && j.lines.iter().any(|l| ids.contains(&l.account.id.as_str()))
        })
        .collect();
    movements.reverse();
    let offset = p["offset"].as_str().unwrap_or("0").parse::<usize>()?;
    Ok(
        json!({"accounts":accounts,"movements":movements.iter().skip(offset).take(100).collect::<Vec<_>>(),"movement_count":movements.len(),"counterparts":s.accounting.accounts.iter().filter(|a|a.company_id==c.id&&a.postable&&!a.archived&&!a.control).collect::<Vec<_>>()}),
    )
}
pub(super) fn movement(s: &State, p: &Value) -> Result<PostCommand> {
    let ctx = context(s, p)?;
    let amount = amount(p, "amount")?;
    ensure!(amount.rials() > 0, "Amount must be positive");
    let setup = s
        .accounting
        .core
        .setups
        .iter()
        .find(|x| x.id == ctx.setup_id)
        .ok_or_else(|| anyhow!("Company chart required"))?;
    let funds: Vec<_> = setup
        .command
        .mappings
        .iter()
        .filter(|m| matches!(m.role, CoreRole::Cash | CoreRole::Bank))
        .map(|m| m.account_id.as_str())
        .collect();
    let source = string(p, "account_id")?;
    let target = string(p, "counterpart_id")?;
    ensure!(funds.contains(&source), "Select cash or bank");
    let kind = string(p, "kind")?;
    let (debit, credit) = match kind {
        "deposit" => (source, target),
        "withdrawal" => (target, source),
        "transfer" => {
            ensure!(
                funds.contains(&target),
                "Transfer target must be cash or bank"
            );
            (target, source)
        }
        _ => return Err(anyhow!("Invalid banking movement")),
    };
    let reference = string(p, "reference")?;
    crate::domain::text(reference)?;
    Ok(PostCommand {
        company_id: ctx.company_id,
        period_id: ctx.period_id,
        date: ctx.date,
        source_id: ctx.source_id,
        source_revision: 1,
        event: format!("banking.{kind}.v1"),
        request_key: ctx.request_key,
        actor: ctx.actor,
        reason: ctx.reason,
        kind: JournalKind::Manual,
        lines: vec![
            JournalLineInput {
                account_id: debit.into(),
                debit: amount,
                credit: Money::ZERO,
                description: reference.into(),

                dimension: String::new(),
            },
            JournalLineInput {
                account_id: credit.into(),
                debit: Money::ZERO,
                credit: amount,
                description: reference.into(),

                dimension: String::new(),
            },
        ],
    })
}
