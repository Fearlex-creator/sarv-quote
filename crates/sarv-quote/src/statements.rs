//! Read-only financial statements derived from the posted books: the balance sheet and
//! receivable/payable aging. Nothing here writes to the books, and every figure is exact
//! integer rial arithmetic.
use crate::{
    accounting::{AccountClass, AccountingState, Money},
    payments::PaymentSide,
};
use anyhow::{Result, ensure};
use chrono::NaiveDate;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatementRow {
    pub account_id: String,
    pub code: String,
    pub name: String,
    pub amount: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BalanceSheet {
    pub company_id: String,
    pub as_of: String,
    pub assets: Vec<StatementRow>,
    pub liabilities: Vec<StatementRow>,
    pub equity: Vec<StatementRow>,
    /// Revenue minus expense not yet closed into equity by a year-end entry.
    pub current_earnings: Money,
    pub total_assets: Money,
    pub total_liabilities: Money,
    /// Equity accounts plus current earnings.
    pub total_equity: Money,
    /// Assets = liabilities + equity. False means the books are out of balance.
    pub balanced: bool,
}

/// Balance sheet as of a date, cumulative through it (same basis as the trial balance).
pub fn balance_sheet(books: &AccountingState, company: &str, as_of: &str) -> Result<BalanceSheet> {
    let tb = books.trial_balance(company, as_of)?;
    let mut sheet = BalanceSheet {
        company_id: company.into(),
        as_of: as_of.into(),
        assets: vec![],
        liabilities: vec![],
        equity: vec![],
        current_earnings: Money::ZERO,
        total_assets: Money::ZERO,
        total_liabilities: Money::ZERO,
        total_equity: Money::ZERO,
        balanced: false,
    };
    let (mut earnings_credit, mut equity_total) = (Money::ZERO, Money::ZERO);
    for row in &tb.rows {
        let account = books.accounts.iter().find(|a| a.id == row.account_id && a.company_id == company);
        let Some(account) = account else { continue };
        let debit_minus_credit = row.debit.checked_sub(row.credit)?;
        let credit_minus_debit = row.credit.checked_sub(row.debit)?;
        let line = |amount: Money| StatementRow {
            account_id: row.account_id.clone(),
            code: row.code.clone(),
            name: row.name.clone(),
            amount,
        };
        match account.classification {
            AccountClass::Asset => {
                sheet.total_assets = sheet.total_assets.checked_add(debit_minus_credit)?;
                if debit_minus_credit != Money::ZERO {
                    sheet.assets.push(line(debit_minus_credit));
                }
            }
            AccountClass::Liability => {
                sheet.total_liabilities = sheet.total_liabilities.checked_add(credit_minus_debit)?;
                if credit_minus_debit != Money::ZERO {
                    sheet.liabilities.push(line(credit_minus_debit));
                }
            }
            AccountClass::Equity => {
                equity_total = equity_total.checked_add(credit_minus_debit)?;
                if credit_minus_debit != Money::ZERO {
                    sheet.equity.push(line(credit_minus_debit));
                }
            }
            // Income statement accounts: revenue is credit-normal, expense debit-normal.
            AccountClass::Revenue | AccountClass::Expense => {
                earnings_credit = earnings_credit.checked_add(credit_minus_debit)?;
            }
        }
    }
    sheet.current_earnings = earnings_credit;
    sheet.total_equity = equity_total.checked_add(earnings_credit)?;
    sheet.balanced =
        sheet.total_assets == sheet.total_liabilities.checked_add(sheet.total_equity)?;
    Ok(sheet)
}

/// One party's open amounts split by how overdue they are.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AgingRow {
    pub party_id: String,
    pub not_due: Money,
    pub days_1_30: Money,
    pub days_31_60: Money,
    pub days_61_90: Money,
    pub days_over_90: Money,
    pub total: Money,
}
impl AgingRow {
    fn add(&mut self, days_overdue: i64, amount: Money) -> Result<()> {
        let bucket = match days_overdue {
            i64::MIN..=0 => &mut self.not_due,
            1..=30 => &mut self.days_1_30,
            31..=60 => &mut self.days_31_60,
            61..=90 => &mut self.days_61_90,
            _ => &mut self.days_over_90,
        };
        *bucket = bucket.checked_add(amount)?;
        self.total = self.total.checked_add(amount)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Aging {
    pub company_id: String,
    /// `receivable` (customers owe us) or `payable` (we owe suppliers).
    pub side: &'static str,
    pub as_of: String,
    pub rows: Vec<AgingRow>,
    pub totals: AgingRow,
}

/// Open invoices or bills grouped by party and days past due as of a date. An item that is not
/// yet due (or due today) is "not due"; overdue days are counted from the due date.
pub fn aging(books: &AccountingState, company: &str, side: PaymentSide, as_of: &str) -> Result<Aging> {
    let as_of_date = NaiveDate::parse_from_str(as_of, "%Y-%m-%d")?;
    let items = books.payment_open_items(company, None, side, as_of)?;
    let mut by_party: BTreeMap<String, AgingRow> = BTreeMap::new();
    let mut totals = AgingRow { party_id: "total".into(), ..AgingRow::default() };
    for item in items.iter().filter(|i| i.remaining.rials() != 0) {
        ensure!(item.remaining.rials() > 0, "Open item with a negative remainder");
        let due = NaiveDate::parse_from_str(&item.due_date, "%Y-%m-%d")?;
        let days = (as_of_date - due).num_days();
        by_party
            .entry(item.party_id.clone())
            .or_insert_with(|| AgingRow { party_id: item.party_id.clone(), ..AgingRow::default() })
            .add(days, item.remaining)?;
        totals.add(days, item.remaining)?;
    }
    Ok(Aging {
        company_id: company.into(),
        side: match side {
            PaymentSide::Receive => "receivable",
            PaymentSide::Make => "payable",
        },
        as_of: as_of.into(),
        rows: by_party.into_values().collect(),
        totals,
    })
}
