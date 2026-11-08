//! Till shifts and held POS carts. These records are operational.
//! Closing a shift stores the counted cash and the expected cash from posted
//! POS receipts. It does not invent a journal; an accountant posts any
//! over/short from Banking when they mean to change the books.
use crate::accounting::Money;
use crate::core::CoreRole;
use crate::domain::{State, text};
use crate::payments::{PaymentData, PaymentSide};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetailState {
    pub shifts: Vec<Shift>,
    pub held: Vec<HeldCart>,
    pub imports: Vec<ImportMark>,
}
impl RetailState {
    pub fn is_empty(&self) -> bool {
        self.shifts.is_empty() && self.held.is_empty() && self.imports.is_empty()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shift {
    pub id: String,
    pub company_id: String,
    pub cashier: String,
    pub opened_on: String,
    pub opening_cash_rial: i64,
    pub request_key: String,
    pub closed_on: String,
    pub counted_cash_rial: Option<i64>,
    pub expected_cash_rial: Option<i64>,
    pub variance_rial: Option<i64>,
    pub note: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeldCart {
    pub id: String,
    pub company_id: String,
    pub label: String,
    pub at: String,
    pub payload: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportMark {
    pub request_key: String,
    pub sha256: String,
    pub kind: String,
    pub summary: String,
}

pub fn check(state: &State) -> Result<()> {
    let companies: Vec<&str> = state
        .accounting
        .companies
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    let retail = &state.retail;
    ensure!(retail.shifts.len() <= 10_000, "Too many till shifts");
    ensure!(retail.held.len() <= 50, "Too many held sales");
    ensure!(retail.imports.len() <= 10_000, "Too many import marks");
    let mut shift_ids = std::collections::BTreeSet::new();
    let mut open = std::collections::BTreeSet::new();
    for shift in &retail.shifts {
        ensure!(shift_ids.insert(&shift.id), "Duplicate till shift");
        ensure!(
            companies.contains(&shift.company_id.as_str()),
            "Till shift is for an unknown company"
        );
        text(&shift.cashier)?;
        text(&shift.request_key)?;
        crate::core::civil_date(&shift.opened_on)?;
        if !shift.closed_on.is_empty() {
            crate::core::civil_date(&shift.closed_on)?;
        }
        ensure!(
            (0..=crate::domain::MAX_MONEY).contains(&shift.opening_cash_rial),
            "Opening till cash is out of range"
        );
        for amount in [
            shift.counted_cash_rial,
            shift.expected_cash_rial,
            shift.variance_rial,
        ]
        .into_iter()
        .flatten()
        {
            ensure!(
                (-crate::domain::MAX_MONEY..=crate::domain::MAX_MONEY).contains(&amount),
                "Till amount is out of range"
            );
        }
        ensure!(shift.note.chars().count() <= 500, "Till note is too long");
        if shift.closed_on.is_empty() {
            ensure!(
                open.insert(&shift.company_id),
                "A company has two open tills"
            );
        }
    }
    let mut held_ids = std::collections::BTreeSet::new();
    for cart in &retail.held {
        ensure!(held_ids.insert(&cart.id), "Duplicate held sale");
        ensure!(
            companies.contains(&cart.company_id.as_str()),
            "Held sale is for an unknown company"
        );
        text(&cart.label)?;
        ensure!(
            cart.payload.len() <= 20_000 && !cart.payload.chars().any(char::is_control),
            "Held sale is too large"
        );
    }
    let mut keys = std::collections::BTreeSet::new();
    for mark in &retail.imports {
        ensure!(keys.insert(&mark.request_key), "Duplicate import mark");
        text(&mark.request_key)?;
        ensure!(
            mark.sha256.len() == 64 && mark.sha256.chars().all(|c| c.is_ascii_hexdigit()),
            "Import fingerprint is invalid"
        );
        ensure!(
            matches!(
                mark.kind.as_str(),
                "products" | "customers" | "suppliers" | "opening_stock"
            ),
            "Unknown import kind"
        );
        ensure!(mark.summary.len() <= 20_000, "Import summary is too large");
    }
    Ok(())
}

pub fn view(state: &State, company_id: &str) -> Value {
    let shifts: Vec<_> = state
        .retail
        .shifts
        .iter()
        .filter(|s| s.company_id == company_id)
        .cloned()
        .collect();
    let held: Vec<_> = state
        .retail
        .held
        .iter()
        .filter(|s| s.company_id == company_id)
        .cloned()
        .collect();
    json!({
        "shifts": shifts,
        "held": held,
        "open_shift": shifts.iter().find(|s| s.closed_on.is_empty()),
        "negative_stock": "rejected",
        "offline": "online_only",
    })
}

pub fn command(store: &mut crate::store::Store, action: &str, p: &Value) -> Result<Value> {
    let company = p["company_id"].as_str().unwrap_or("").to_owned();
    ensure!(
        store
            .state
            .accounting
            .companies
            .iter()
            .any(|c| c.id == company),
        "Company not found"
    );
    match action {
        "pos_shift_open" => {
            let key = required(p, "request_key")?;
            if let Some(existing) = store
                .state
                .retail
                .shifts
                .iter()
                .find(|s| s.request_key == key)
            {
                return Ok(json!({"shift": existing, "replayed": true}));
            }
            ensure!(
                !store
                    .state
                    .retail
                    .shifts
                    .iter()
                    .any(|s| s.company_id == company && s.closed_on.is_empty()),
                "Close the open till before opening another"
            );
            let opening = money(p, "opening_cash_rial")?;
            ensure!(opening >= 0, "Opening cash cannot be negative");
            let cashier = required(p, "cashier")?;
            text(&cashier)?;
            let opened_on = required(p, "date")?;
            crate::core::civil_date(&opened_on)?;
            let shift = Shift {
                id: format!("shift-{key}"),
                company_id: company,
                cashier,
                opened_on,
                opening_cash_rial: opening,
                request_key: key,
                closed_on: String::new(),
                counted_cash_rial: None,
                expected_cash_rial: None,
                variance_rial: None,
                note: String::new(),
            };
            store.change(|s| {
                s.retail.shifts.push(shift.clone());
                Ok(())
            })?;
            Ok(json!({"shift": shift, "replayed": false}))
        }
        "pos_shift_close" => {
            let id = required(p, "shift_id")?;
            let counted = money(p, "counted_cash_rial")?;
            ensure!(counted >= 0, "Counted cash cannot be negative");
            let note = p["note"].as_str().unwrap_or("").to_owned();
            ensure!(note.chars().count() <= 500, "Till note is too long");
            let closed_on = required(p, "date")?;
            crate::core::civil_date(&closed_on)?;
            let shift = store.change(|s| {
                let expected = expected_cash(s, &company, &id)?;
                let shift = s
                    .retail
                    .shifts
                    .iter_mut()
                    .find(|s| s.id == id && s.company_id == company)
                    .ok_or_else(|| anyhow!("Open this company's till first"))?;
                ensure!(shift.closed_on.is_empty(), "This till is already closed");
                ensure!(
                    closed_on.as_str() >= shift.opened_on.as_str(),
                    "Close date is before the till opened"
                );
                let variance = counted
                    .checked_sub(expected)
                    .ok_or_else(|| anyhow!("Till variance overflow"))?;
                shift.closed_on = closed_on;
                shift.counted_cash_rial = Some(counted);
                shift.expected_cash_rial = Some(expected);
                shift.variance_rial = Some(variance);
                shift.note = note;
                Ok(shift.clone())
            })?;
            Ok(json!({
                "shift": shift,
                "ledger_posted": false,
                "notice": "Counted cash is stored on the till. The difference is not a journal entry until someone posts it from Banking.",
            }))
        }
        "pos_hold" => {
            let id = required(p, "id")?;
            text(&id)?;
            let label = required(p, "label")?;
            text(&label)?;
            let payload = required(p, "payload")?;
            ensure!(
                payload.len() <= 20_000 && !payload.chars().any(char::is_control),
                "Held sale is too large"
            );
            serde_json::from_str::<Value>(&payload).map_err(|_| anyhow!("Held sale must be JSON"))?;
            let cart = HeldCart {
                id,
                company_id: company,
                label,
                at: required(p, "date")?,
                payload,
            };
            crate::core::civil_date(&cart.at)?;
            store.change(|s| {
                if let Some(existing) = s.retail.held.iter_mut().find(|c| c.id == cart.id) {
                    ensure!(
                        existing.company_id == cart.company_id,
                        "Held sale belongs to another company"
                    );
                    *existing = cart.clone();
                } else {
                    ensure!(s.retail.held.len() < 50, "Too many held sales");
                    s.retail.held.push(cart.clone());
                }
                Ok(())
            })?;
            Ok(json!({"held": cart}))
        }
        "pos_hold_delete" => {
            let id = required(p, "id")?;
            store.change(|s| {
                let before = s.retail.held.len();
                s.retail
                    .held
                    .retain(|c| !(c.id == id && c.company_id == company));
                ensure!(s.retail.held.len() != before, "Held sale not found");
                Ok(())
            })?;
            Ok(json!({"deleted": true}))
        }
        _ => Err(anyhow!("Unsupported till command")),
    }
}

fn expected_cash(state: &State, company: &str, shift_id: &str) -> Result<i64> {
    let shift = state
        .retail
        .shifts
        .iter()
        .find(|s| s.id == shift_id && s.company_id == company)
        .ok_or_else(|| anyhow!("Till not found"))?;
    let prefix = format!("POS:{shift_id}:");
    let mut net: i64 = shift.opening_cash_rial;
    for record in &state.accounting.payments.records {
        if record.command.context().company_id != company || record.funds_role() != CoreRole::Cash
        {
            continue;
        }
        let (reference, side, total, date) = match &record.data {
            PaymentData::Payment(p) => (
                p.reference.as_str(),
                p.side,
                p.total,
                record.command.context().date.as_str(),
            ),
            PaymentData::Reversal(p) => (
                p.reference.as_str(),
                p.side,
                p.total,
                record.command.context().date.as_str(),
            ),
        };
        if date < shift.opened_on.as_str() || !reference.starts_with(&prefix) {
            continue;
        }
        let signed = match (&record.data, side) {
            (PaymentData::Payment(_), PaymentSide::Receive) => total.rials(),
            (PaymentData::Payment(_), PaymentSide::Make) => -total.rials(),
            (PaymentData::Reversal(_), PaymentSide::Receive) => -total.rials(),
            (PaymentData::Reversal(_), PaymentSide::Make) => total.rials(),
        };
        net = net
            .checked_add(signed)
            .ok_or_else(|| anyhow!("Till total overflow"))?;
    }
    let _ = Money::from_rials(net.abs());
    Ok(net)
}

fn required(p: &Value, key: &str) -> Result<String> {
    p.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("Missing {key}"))
}
fn money(p: &Value, key: &str) -> Result<i64> {
    let raw = required(p, key)?;
    let amount = Money::parse(&raw, crate::accounting::MoneyUnit::Rial)?;
    Ok(amount.rials())
}
