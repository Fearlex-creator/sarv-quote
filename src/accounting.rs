//! Headless, policy-neutral IRR ledger. This slice does not implement subledgers,
//! tax, valuation, permissions or closing transfers. Local audit is not tamper proof.
use crate::domain::{digits, id, normalize, now, text};
use anyhow::{Result, ensure};
use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// Canonical signed integer IRR, scale zero, range i64::MIN..=i64::MAX.
/// All arithmetic and conversions fail on overflow; there is no rounding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Money(i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoneyUnit {
    Rial,
    Toman,
}
impl Money {
    pub const ZERO: Self = Self(0);
    pub fn from_rials(value: i64) -> Self {
        Self(value)
    }
    pub fn rials(self) -> i64 {
        self.0
    }
    pub fn checked_add(self, other: Self) -> Result<Self> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or_else(|| anyhow::anyhow!("سرریز مبلغ ریالی"))
    }
    pub fn checked_sub(self, other: Self) -> Result<Self> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or_else(|| anyhow::anyhow!("سرریز مبلغ ریالی"))
    }
    pub fn parse(input: &str, unit: MoneyUnit) -> Result<Self> {
        let input = digits(input.trim());
        let (negative, value) = match input.strip_prefix('-') {
            Some(s) => (true, s),
            None => (false, input.as_str()),
        };
        let mut parts = value.split('.');
        let whole = parts.next().unwrap_or_default();
        let fraction = parts.next();
        ensure!(
            !whole.is_empty()
                && whole.bytes().all(|b| b.is_ascii_digit())
                && parts.next().is_none(),
            "مبلغ نامعتبر است؛ جداکنندهٔ هزارگان وارد نکنید"
        );
        if let Some(fraction) = fraction {
            ensure!(
                !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit()),
                "اعشار مبلغ نامعتبر است"
            );
        }
        let whole: i128 = whole
            .parse()
            .map_err(|_| anyhow::anyhow!("سرریز مبلغ ریالی"))?;
        let multiplier = if unit == MoneyUnit::Toman { 10 } else { 1 };
        let fractional = match (unit, fraction) {
            (_, None) => 0,
            (MoneyUnit::Rial, Some(f)) => {
                ensure!(f.bytes().all(|b| b == b'0'), "کسری از ریال قابل ثبت نیست");
                0
            }
            (MoneyUnit::Toman, Some(f)) => {
                ensure!(
                    f[1..].bytes().all(|b| b == b'0'),
                    "کسری از ریال قابل ثبت نیست"
                );
                (f.as_bytes()[0] - b'0') as i128
            }
        };
        let unsigned = whole
            .checked_mul(multiplier)
            .and_then(|v| v.checked_add(fractional))
            .ok_or_else(|| anyhow::anyhow!("سرریز مبلغ ریالی"))?;
        let signed = if negative { -unsigned } else { unsigned };
        Ok(Self(
            i64::try_from(signed).map_err(|_| anyhow::anyhow!("سرریز مبلغ ریالی"))?,
        ))
    }
    /// Decimal toman values preserve residual rials, including negative values.
    pub fn display(self, unit: MoneyUnit) -> String {
        if unit == MoneyUnit::Rial {
            return self.0.to_string();
        }
        let value = i128::from(self.0);
        let magnitude = value.abs();
        let sign = if value < 0 { "-" } else { "" };
        if magnitude % 10 == 0 {
            format!("{sign}{}", magnitude / 10)
        } else {
            format!("{sign}{}.{}", magnitude / 10, magnitude % 10)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountClass {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BalanceSide {
    Debit,
    Credit,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Company {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiscalPeriod {
    pub id: String,
    pub company_id: String,
    pub name: String,
    pub start_date: String,
    pub end_date: String,
    pub locked: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewAccount {
    pub company_id: String,
    pub parent_id: Option<String>,
    pub code: String,
    pub name: String,
    pub classification: AccountClass,
    pub normal_balance: BalanceSide,
    pub postable: bool,
    pub control: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    pub id: String,
    pub company_id: String,
    pub parent_id: Option<String>,
    pub code: String,
    pub name: String,
    pub classification: AccountClass,
    pub normal_balance: BalanceSide,
    pub postable: bool,
    pub control: bool,
    pub archived: bool,
    pub version: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalKind {
    Opening,
    Manual,
    Reversal { journal_id: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalLineInput {
    pub account_id: String,
    pub debit: Money,
    pub credit: Money,
    pub description: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostCommand {
    pub company_id: String,
    pub period_id: String,
    pub date: String,
    pub source_id: String,
    pub source_revision: u64,
    pub event: String,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub kind: JournalKind,
    pub lines: Vec<JournalLineInput>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalLine {
    pub account: Account,
    pub debit: Money,
    pub credit: Money,
    pub description: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Journal {
    pub id: String,
    pub company_id: String,
    pub period_id: String,
    pub date: String,
    pub source_id: String,
    pub source_revision: u64,
    pub event: String,
    pub actor: String,
    pub reason: String,
    pub kind: JournalKind,
    pub lines: Vec<JournalLine>,
    pub recorded_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditOperation {
    CompanyCreated,
    PeriodCreated,
    AccountCreated,
    AccountArchived,
    JournalPosted,
    PeriodLocked,
    PeriodReopened,
    CoreConfigured,
    CoreOpeningInitialized,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEvent {
    pub id: String,
    pub company_id: String,
    pub entity_id: String,
    pub operation: AuditOperation,
    pub actor: String,
    pub reason: String,
    pub recorded_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostRequestRecord {
    pub command: PostCommand,
    pub journal_id: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountingState {
    pub companies: Vec<Company>,
    pub periods: Vec<FiscalPeriod>,
    pub accounts: Vec<Account>,
    pub journals: Vec<Journal>,
    pub requests: Vec<PostRequestRecord>,
    pub audit: Vec<AuditEvent>,
    #[serde(default, skip_serializing_if = "crate::core::CoreFoundation::is_empty")]
    pub core: crate::core::CoreFoundation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialBalanceRow {
    pub account_id: String,
    pub code: String,
    pub name: String,
    pub debit: Money,
    pub credit: Money,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialBalance {
    pub company_id: String,
    pub as_of: String,
    pub rows: Vec<TrialBalanceRow>,
    pub total_debit: Money,
    pub total_credit: Money,
}
fn iso_date(value: &str) -> Result<NaiveDate> {
    let parsed = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("تاریخ باید میلادی معتبر به شکل YYYY-MM-DD باشد"))?;
    ensure!(
        parsed.format("%Y-%m-%d").to_string() == value,
        "تاریخ باید به شکل YYYY-MM-DD باشد"
    );
    Ok(parsed)
}
fn optional_text(value: &str) -> Result<()> {
    if !value.is_empty() {
        text(value)?;
    }
    Ok(())
}
fn same_business(a: &PostCommand, b: &PostCommand) -> bool {
    a.company_id == b.company_id
        && a.source_id == b.source_id
        && a.source_revision == b.source_revision
        && a.event == b.event
}
fn same_payload(a: &PostCommand, b: &PostCommand) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.request_key.clear();
    b.request_key.clear();
    a == b
}
impl AccountingState {
    pub fn is_empty(&self) -> bool {
        self.companies.is_empty()
            && self.periods.is_empty()
            && self.accounts.is_empty()
            && self.journals.is_empty()
            && self.requests.is_empty()
            && self.audit.is_empty()
            && self.core.is_empty()
    }
    /// Validate a restored snapshot by replaying its complete command/audit history.
    /// This is integrity validation, not authentication against a file owner.
    pub fn validate(&self) -> Result<()> {
        let mut ids = HashSet::new();
        for entity_id in self
            .companies
            .iter()
            .map(|c| &c.id)
            .chain(self.periods.iter().map(|p| &p.id))
            .chain(self.accounts.iter().map(|a| &a.id))
            .chain(self.journals.iter().map(|j| &j.id))
            .chain(self.audit.iter().map(|a| &a.id))
            .chain(self.core.setups.iter().map(|s| &s.id))
            .chain(
                self.core
                    .openings
                    .iter()
                    .filter(|o| o.effect == crate::core::OpeningEffect::ZeroBalanceInitialization)
                    .map(|o| &o.journal_id),
            )
        {
            text(entity_id)?;
            ensure!(ids.insert(entity_id), "شناسهٔ حسابداری تکراری است");
        }
        let mut replay = Self::default();
        for event in &self.audit {
            text(&event.actor)?;
            text(&event.reason)?;
            DateTime::parse_from_rfc3339(&event.recorded_at)
                .map_err(|_| anyhow::anyhow!("زمان تاریخچه نامعتبر است"))?;
            let before_audit = replay.audit.len();
            match event.operation {
                AuditOperation::CoreConfigured => {
                    let setup = self
                        .core
                        .setups
                        .iter()
                        .find(|s| s.id == event.entity_id)
                        .ok_or_else(|| anyhow::anyhow!("تنظیمات تاریخچه پیدا نشد"))?;
                    ensure!(
                        setup.command.company_id == event.company_id
                            && setup.command.actor == event.actor
                            && setup.command.reason == event.reason,
                        "تنظیمات و تاریخچهٔ آن سازگار نیستند"
                    );
                    replay.replay_core_setup(setup)?;
                }
                AuditOperation::CoreOpeningInitialized => {
                    let opening = self
                        .core
                        .openings
                        .iter()
                        .find(|o| {
                            o.journal_id == event.entity_id
                                && o.effect == crate::core::OpeningEffect::ZeroBalanceInitialization
                        })
                        .ok_or_else(|| anyhow::anyhow!("رویداد شروع دفتر بدون مانده پیدا نشد"))?;
                    ensure!(
                        opening.command.company_id == event.company_id
                            && opening.command.actor == event.actor
                            && opening.command.reason == event.reason,
                        "شروع دفتر و تاریخچهٔ آن سازگار نیستند"
                    );
                    let before = replay.journals.len();
                    replay.replay_core_opening(opening.command.clone())?;
                    ensure!(
                        replay.journals.len() == before,
                        "شروع دفتر بدون مانده نباید سند مالی ایجاد کند"
                    );
                    let created = replay.core.openings.last_mut().unwrap();
                    ensure!(
                        created.effect == crate::core::OpeningEffect::ZeroBalanceInitialization,
                        "شروع دفتر بدون مانده اثر مالی دارد"
                    );
                    created.journal_id = event.entity_id.clone();
                    replay
                        .core
                        .zero_requests
                        .last_mut()
                        .unwrap()
                        .initialization_id = event.entity_id.clone();
                }
                AuditOperation::CompanyCreated => {
                    let company = self.company(&event.entity_id)?;
                    ensure!(
                        company.id == event.company_id
                            && !replay.companies.iter().any(|c| c.id == company.id),
                        "تاریخچهٔ ایجاد کسب‌وکار نامعتبر است"
                    );
                    text(&company.name)?;
                    replay.companies.push(company.clone());
                    replay.activity(
                        &company.id,
                        &company.id,
                        AuditOperation::CompanyCreated,
                        &event.actor,
                        &event.reason,
                    );
                }
                AuditOperation::PeriodCreated => {
                    let p = self.period(&event.entity_id)?;
                    ensure!(
                        p.company_id == event.company_id
                            && !replay.periods.iter().any(|old| old.id == p.id),
                        "تاریخچهٔ ایجاد سال مالی نامعتبر است"
                    );
                    let generated = replay.add_period(
                        &p.company_id,
                        &p.name,
                        &p.start_date,
                        &p.end_date,
                        &event.actor,
                    )?;
                    let created = replay
                        .periods
                        .iter_mut()
                        .find(|old| old.id == generated)
                        .unwrap();
                    created.id = p.id.clone();
                }
                AuditOperation::AccountCreated => {
                    let a = self
                        .accounts
                        .iter()
                        .find(|a| a.id == event.entity_id)
                        .ok_or_else(|| anyhow::anyhow!("حساب تاریخچه پیدا نشد"))?;
                    ensure!(
                        a.company_id == event.company_id
                            && !replay.accounts.iter().any(|old| old.id == a.id),
                        "تاریخچهٔ ایجاد حساب نامعتبر است"
                    );
                    let generated = replay.add_account(
                        NewAccount {
                            company_id: a.company_id.clone(),
                            parent_id: a.parent_id.clone(),
                            code: a.code.clone(),
                            name: a.name.clone(),
                            classification: a.classification,
                            normal_balance: a.normal_balance,
                            postable: a.postable,
                            control: a.control,
                        },
                        &event.actor,
                    )?;
                    replay
                        .accounts
                        .iter_mut()
                        .find(|old| old.id == generated)
                        .unwrap()
                        .id = a.id.clone();
                }
                AuditOperation::AccountArchived => {
                    ensure!(
                        replay
                            .accounts
                            .iter()
                            .any(|a| a.id == event.entity_id && a.company_id == event.company_id),
                        "کسب‌وکار تاریخچهٔ حساب نامعتبر است"
                    );
                    replay.archive_account(&event.entity_id, &event.actor, &event.reason)?;
                }
                AuditOperation::PeriodLocked | AuditOperation::PeriodReopened => {
                    ensure!(
                        replay.period(&event.entity_id)?.company_id == event.company_id,
                        "کسب‌وکار تاریخچهٔ قفل نامعتبر است"
                    );
                    replay.set_period_lock(
                        &event.entity_id,
                        event.operation == AuditOperation::PeriodLocked,
                        &event.actor,
                        &event.reason,
                    )?;
                }
                AuditOperation::JournalPosted => {
                    let j = self
                        .journals
                        .iter()
                        .find(|j| j.id == event.entity_id)
                        .ok_or_else(|| anyhow::anyhow!("سند تاریخچه پیدا نشد"))?;
                    ensure!(
                        j.company_id == event.company_id
                            && j.actor == event.actor
                            && j.reason == event.reason
                            && j.recorded_at == event.recorded_at,
                        "تاریخچه و سند ثبت‌شده سازگار نیستند"
                    );
                    let request = self
                        .requests
                        .iter()
                        .find(|r| r.journal_id == j.id)
                        .ok_or_else(|| anyhow::anyhow!("درخواست سند ثبت‌شده پیدا نشد"))?;
                    let opening = self.core.openings.iter().find(|o| o.journal_id == j.id);
                    let generated = if let Some(opening) = opening {
                        replay.replay_core_opening(opening.command.clone())?
                    } else {
                        replay.post(request.command.clone())?
                    };
                    let posted = replay
                        .journals
                        .iter_mut()
                        .find(|posted| posted.id == generated)
                        .ok_or_else(|| anyhow::anyhow!("تاریخچه رویداد اقتصادی تکراری دارد"))?;
                    let mut expected = j.clone();
                    expected.id = generated.clone();
                    expected.recorded_at = posted.recorded_at.clone();
                    ensure!(
                        *posted == expected,
                        "محتوا یا تصویر حساب‌های سند ثبت‌شده نامعتبر است"
                    );
                    *posted = j.clone();
                    replay.requests.last_mut().unwrap().journal_id = j.id.clone();
                    if opening.is_some() {
                        replay.core.openings.last_mut().unwrap().journal_id = j.id.clone();
                    }
                }
            }
            ensure!(
                replay.audit.len() == before_audit + 1,
                "تاریخچهٔ عملیات تکراری یا نامعتبر است"
            );
            replay.audit[before_audit] = event.clone();
        }
        // Include aliases from identical business retries. No additional financial
        // effects or audit events may be created while reconstructing those aliases.
        let journal_count = replay.journals.len();
        let audit_count = replay.audit.len();
        for request in &self.requests {
            ensure!(
                replay.post(request.command.clone())? == request.journal_id,
                "درخواست به سند دیگری اشاره می‌کند"
            );
        }
        for request in &self.core.zero_requests {
            ensure!(
                replay.replay_zero_request(request)? == request.initialization_id,
                "درخواست شروع دفتر به رویداد دیگری اشاره می‌کند"
            );
        }
        ensure!(
            replay.journals.len() == journal_count && replay.audit.len() == audit_count,
            "درخواست بدون تاریخچهٔ ثبت وجود دارد"
        );
        // Original request order includes retry aliases between journal commands.
        // Uniqueness has been checked by post; preserve that original order.
        ensure!(
            replay.requests.len() == self.requests.len(),
            "کلید درخواست تکراری است"
        );
        replay.requests = self.requests.clone();
        ensure!(
            replay.core.zero_requests.len() == self.core.zero_requests.len(),
            "کلید درخواست شروع دفتر تکراری است"
        );
        replay.core.zero_requests = self.core.zero_requests.clone();
        ensure!(
            replay == *self,
            "دادهٔ حسابداری با تاریخچهٔ کامل آن سازگار نیست"
        );
        Ok(())
    }
    /// Generic persistence mutations cannot delete/rewrite posted records or
    /// circumvent locks by editing a historical period/account definition.
    pub fn validate_transition(&self, next: &Self) -> Result<()> {
        next.validate()?;
        self.core.validate_transition(&next.core)?;
        ensure!(
            next.journals.starts_with(&self.journals),
            "سند ثبت‌شده قابل تغییر یا حذف نیست؛ برگشت مرتبط ثبت کنید"
        );
        ensure!(
            next.requests.starts_with(&self.requests),
            "تاریخچهٔ درخواست ثبت‌شده قابل تغییر یا حذف نیست"
        );
        ensure!(
            next.audit.starts_with(&self.audit),
            "تاریخچهٔ حسابداری قابل تغییر یا حذف نیست"
        );
        ensure!(
            next.companies.starts_with(&self.companies),
            "هویت کسب‌وکار قابل تغییر یا حذف نیست"
        );
        ensure!(
            next.periods.len() >= self.periods.len() && next.accounts.len() >= self.accounts.len(),
            "سال مالی یا حساب قابل حذف نیست"
        );
        for (old, new) in self.periods.iter().zip(&next.periods) {
            let mut expected = old.clone();
            expected.locked = new.locked;
            ensure!(*new == expected, "مشخصات سال مالی موجود قابل تغییر نیست");
        }
        for (old, new) in self.accounts.iter().zip(&next.accounts) {
            if old != new {
                let mut expected = old.clone();
                ensure!(!old.archived, "حساب بایگانی‌شده قابل تغییر نیست");
                expected.archived = true;
                expected.version = old
                    .version
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("سرریز نسخهٔ حساب"))?;
                ensure!(
                    *new == expected,
                    "مشخصات حساب موجود قابل تغییر نیست؛ فقط بایگانی مجاز است"
                );
            }
        }
        Ok(())
    }
    fn company(&self, id: &str) -> Result<&Company> {
        self.companies
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| anyhow::anyhow!("کسب‌وکار پیدا نشد"))
    }
    fn period(&self, id: &str) -> Result<&FiscalPeriod> {
        self.periods
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("سال مالی پیدا نشد"))
    }
    pub(crate) fn activity(
        &mut self,
        company: &str,
        entity: &str,
        operation: AuditOperation,
        actor: &str,
        reason: &str,
    ) {
        self.audit.push(AuditEvent {
            id: id(),
            company_id: company.into(),
            entity_id: entity.into(),
            operation,
            actor: actor.into(),
            reason: reason.into(),
            recorded_at: now(),
        });
    }
    pub fn add_company(&mut self, name: &str, actor: &str) -> Result<String> {
        text(name)?;
        text(actor)?;
        let company_id = id();
        self.companies.push(Company {
            id: company_id.clone(),
            name: name.trim().into(),
        });
        self.activity(
            &company_id,
            &company_id,
            AuditOperation::CompanyCreated,
            actor,
            "ایجاد کسب‌وکار",
        );
        Ok(company_id)
    }
    pub fn add_period(
        &mut self,
        company_id: &str,
        name: &str,
        start: &str,
        end: &str,
        actor: &str,
    ) -> Result<String> {
        self.company(company_id)?;
        text(name)?;
        text(actor)?;
        let start_date = iso_date(start)?;
        let end_date = iso_date(end)?;
        ensure!(start_date <= end_date, "پایان سال مالی پیش از شروع آن است");
        ensure!(
            !self.periods.iter().any(|p| p.company_id == company_id
                && start <= p.end_date.as_str()
                && end >= p.start_date.as_str()),
            "سال مالی با دورهٔ موجود هم‌پوشانی دارد"
        );
        let period_id = id();
        self.periods.push(FiscalPeriod {
            id: period_id.clone(),
            company_id: company_id.into(),
            name: name.trim().into(),
            start_date: start.into(),
            end_date: end.into(),
            locked: false,
        });
        self.activity(
            company_id,
            &period_id,
            AuditOperation::PeriodCreated,
            actor,
            "ایجاد سال مالی",
        );
        Ok(period_id)
    }
    pub fn add_account(&mut self, input: NewAccount, actor: &str) -> Result<String> {
        self.company(&input.company_id)?;
        text(&input.code)?;
        text(&input.name)?;
        text(actor)?;
        ensure!(
            !self
                .accounts
                .iter()
                .any(|a| a.company_id == input.company_id
                    && normalize(&a.code) == normalize(&input.code)),
            "کد حساب در این کسب‌وکار تکراری است"
        );
        if let Some(parent) = &input.parent_id {
            ensure!(
                self.accounts
                    .iter()
                    .any(|a| a.id == *parent && a.company_id == input.company_id && !a.archived),
                "حساب والد معتبر در این کسب‌وکار انتخاب کنید"
            );
        }
        let account_id = id();
        self.accounts.push(Account {
            id: account_id.clone(),
            company_id: input.company_id.clone(),
            parent_id: input.parent_id,
            code: input.code.trim().into(),
            name: input.name.trim().into(),
            classification: input.classification,
            normal_balance: input.normal_balance,
            postable: input.postable,
            control: input.control,
            archived: false,
            version: 1,
        });
        self.activity(
            &input.company_id,
            &account_id,
            AuditOperation::AccountCreated,
            actor,
            "ایجاد حساب با طبقه‌بندی انتخاب‌شده",
        );
        Ok(account_id)
    }
    pub fn archive_account(&mut self, account_id: &str, actor: &str, reason: &str) -> Result<()> {
        text(actor)?;
        text(reason)?;
        ensure!(
            !self.core.setups.iter().any(|s| s
                .command
                .mappings
                .iter()
                .any(|m| m.account_id == account_id)),
            "حساب نگاشت‌شدهٔ تنظیمات فعال قابل بایگانی نیست؛ تغییر نگاشت پشتیبانی نمی‌شود"
        );
        ensure!(
            !self
                .accounts
                .iter()
                .any(|a| a.parent_id.as_deref() == Some(account_id) && !a.archived),
            "ابتدا حساب‌های فرزند فعال را بایگانی کنید"
        );
        let a = self
            .accounts
            .iter_mut()
            .find(|a| a.id == account_id)
            .ok_or_else(|| anyhow::anyhow!("حساب پیدا نشد"))?;
        ensure!(!a.archived, "حساب قبلاً بایگانی شده است");
        a.version = a
            .version
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("سرریز نسخهٔ حساب"))?;
        a.archived = true;
        let company = a.company_id.clone();
        self.activity(
            &company,
            account_id,
            AuditOperation::AccountArchived,
            actor,
            reason,
        );
        Ok(())
    }
    pub fn set_period_lock(
        &mut self,
        period_id: &str,
        locked: bool,
        actor: &str,
        reason: &str,
    ) -> Result<()> {
        text(actor)?;
        text(reason)?;
        let p = self
            .periods
            .iter_mut()
            .find(|p| p.id == period_id)
            .ok_or_else(|| anyhow::anyhow!("سال مالی پیدا نشد"))?;
        ensure!(p.locked != locked, "وضعیت قفل تغییری نکرده است");
        p.locked = locked;
        let company = p.company_id.clone();
        self.activity(
            &company,
            period_id,
            if locked {
                AuditOperation::PeriodLocked
            } else {
                AuditOperation::PeriodReopened
            },
            actor,
            reason,
        );
        Ok(())
    }
    pub fn post(&mut self, command: PostCommand) -> Result<String> {
        self.post_internal(command, false)
    }
    pub(crate) fn post_core_opening(&mut self, command: PostCommand) -> Result<String> {
        ensure!(
            command.event == crate::core::CORE_OPENING_EVENT
                && command.kind == JournalKind::Opening,
            "مجوز کنترل فقط برای افتتاحیهٔ تطبیق‌شده است"
        );
        self.post_internal(command, true)
    }
    fn post_internal(&mut self, command: PostCommand, control_allowed: bool) -> Result<String> {
        // Validate retry identity before locks: successful historical retries do not mutate books.
        text(&command.request_key)?;
        ensure!(
            !self
                .core
                .zero_requests
                .iter()
                .any(|r| r.request_key == command.request_key),
            "کلید درخواست برای شروع دفتر بدون مانده استفاده شده است"
        );
        if let Some(prior) = self
            .requests
            .iter()
            .find(|r| r.command.request_key == command.request_key)
        {
            ensure!(
                prior.command == command,
                "کلید درخواست با محتوای متفاوت استفاده شده است"
            );
            return Ok(prior.journal_id.clone());
        }
        if let Some(prior) = self
            .requests
            .iter()
            .find(|r| same_business(&r.command, &command))
        {
            ensure!(
                same_payload(&prior.command, &command),
                "رویداد اقتصادی قبلاً با محتوای متفاوت ثبت شده است"
            );
            let journal_id = prior.journal_id.clone();
            self.requests.push(PostRequestRecord {
                command,
                journal_id: journal_id.clone(),
            });
            return Ok(journal_id);
        }
        ensure!(
            control_allowed || command.event != crate::core::CORE_OPENING_EVENT,
            "افتتاحیهٔ کنترلی فقط از گردش کار تطبیق‌شده مجاز است"
        );
        ensure!(
            control_allowed
                || !self
                    .core
                    .setups
                    .iter()
                    .any(|s| s.command.company_id == command.company_id)
                || self
                    .core
                    .openings
                    .iter()
                    .any(|o| o.command.company_id == command.company_id),
            "ابتدا افتتاحیهٔ تطبیق‌شدهٔ این شرکت را ثبت کنید؛ ثبت دستی پیش از آن مجاز نیست"
        );
        self.company(&command.company_id)?;
        let period = self.period(&command.period_id)?;
        ensure!(
            period.company_id == command.company_id,
            "سال مالی متعلق به کسب‌وکار دیگری است"
        );
        ensure!(!period.locked, "سال مالی قفل است؛ ثبت یا برگشت مجاز نیست");
        iso_date(&command.date)?;
        ensure!(
            command.date >= period.start_date && command.date <= period.end_date,
            "تاریخ خارج از سال مالی انتخاب‌شده است"
        );
        text(&command.source_id)?;
        text(&command.event)?;
        text(&command.actor)?;
        text(&command.reason)?;
        ensure!(command.source_revision > 0, "نسخهٔ منبع باید مثبت باشد");
        ensure!(
            (2..=10_000).contains(&command.lines.len()),
            "سند باید بین ۲ تا ۱۰۰۰۰ ردیف معنادار داشته باشد"
        );
        if command.kind == JournalKind::Opening {
            ensure!(
                control_allowed
                    || !self
                        .core
                        .setups
                        .iter()
                        .any(|s| s.command.company_id == command.company_id),
                "افتتاحیهٔ شرکت هسته فقط از گردش کار تطبیق‌شده مجاز است"
            );
            ensure!(
                command.date == period.start_date,
                "افتتاحیه باید در اولین روز سال مالی ثبت شود"
            );
            ensure!(
                !self
                    .journals
                    .iter()
                    .any(|j| j.company_id == command.company_id && j.kind == JournalKind::Opening),
                "افتتاحیهٔ این کسب‌وکار قبلاً ثبت شده است؛ انتقال مجدد مانده‌ها پشتیبانی نمی‌شود"
            );
            ensure!(
                !self
                    .journals
                    .iter()
                    .any(|j| j.company_id == command.company_id),
                "افتتاحیه باید پیش از سایر اسناد این کسب‌وکار ثبت شود"
            );
        }
        let mut lines = Vec::with_capacity(command.lines.len());
        let mut debit = Money::ZERO;
        let mut credit = Money::ZERO;
        self.check_reversal(&command)?;
        for (index, line) in command.lines.iter().enumerate() {
            optional_text(&line.description)?;
            ensure!(
                (line.debit.0 > 0 && line.credit.0 == 0)
                    || (line.credit.0 > 0 && line.debit.0 == 0),
                "هر ردیف باید یک مبلغ مثبت فقط در بدهکار یا بستانکار داشته باشد"
            );
            let account = self
                .accounts
                .iter()
                .find(|a| a.id == line.account_id)
                .ok_or_else(|| anyhow::anyhow!("حساب ردیف پیدا نشد"))?;
            ensure!(
                account.company_id == command.company_id,
                "حساب ردیف متعلق به کسب‌وکار دیگری است"
            );
            let account = match &command.kind {
                JournalKind::Reversal { journal_id } => {
                    &self
                        .journals
                        .iter()
                        .find(|j| j.id == *journal_id)
                        .unwrap()
                        .lines[index]
                        .account
                }
                _ => {
                    ensure!(
                        account.postable && !account.archived,
                        "حساب ردیف بایگانی است یا قابل ثبت نیست"
                    );
                    account
                }
            };
            ensure!(
                control_allowed || !account.control,
                "حساب کنترلی به زیرسیستم معتبر نیاز دارد؛ ثبت مستقیم در این مرحله پشتیبانی نمی‌شود"
            );
            debit = debit.checked_add(line.debit)?;
            credit = credit.checked_add(line.credit)?;
            lines.push(JournalLine {
                account: account.clone(),
                debit: line.debit,
                credit: line.credit,
                description: line.description.clone(),
            });
        }
        ensure!(debit == credit, "جمع بدهکار و بستانکار برابر نیست");
        let journal_id = id();
        let recorded_at = now();
        let journal = Journal {
            id: journal_id.clone(),
            company_id: command.company_id.clone(),
            period_id: command.period_id.clone(),
            date: command.date.clone(),
            source_id: command.source_id.clone(),
            source_revision: command.source_revision,
            event: command.event.clone(),
            actor: command.actor.clone(),
            reason: command.reason.clone(),
            kind: command.kind.clone(),
            lines,
            recorded_at: recorded_at.clone(),
        };
        self.validate_report_range(&journal)?;
        self.journals.push(journal);
        self.audit.push(AuditEvent {
            id: id(),
            company_id: command.company_id.clone(),
            entity_id: journal_id.clone(),
            operation: AuditOperation::JournalPosted,
            actor: command.actor.clone(),
            reason: command.reason.clone(),
            recorded_at,
        });
        self.requests.push(PostRequestRecord {
            command,
            journal_id: journal_id.clone(),
        });
        Ok(journal_id)
    }
    /// A command cannot make any past/future financial-date cutoff unreportable.
    /// Wide checked accumulation avoids order-dependent intermediate overflow;
    /// persisted amounts and reported net balances/totals remain checked i64 IRR.
    fn validate_report_range(&self, candidate: &Journal) -> Result<()> {
        let mut journals: Vec<_> = self
            .journals
            .iter()
            .filter(|j| j.company_id == candidate.company_id)
            .chain(std::iter::once(candidate))
            .collect();
        journals.sort_by(|a, b| a.date.cmp(&b.date));
        let mut balances: BTreeMap<&str, i128> = BTreeMap::new();
        for (index, journal) in journals.iter().enumerate() {
            for line in &journal.lines {
                let value = balances.entry(&line.account.id).or_default();
                *value = value
                    .checked_add(i128::from(line.debit.0))
                    .and_then(|v| v.checked_sub(i128::from(line.credit.0)))
                    .ok_or_else(|| anyhow::anyhow!("سرریز ماندهٔ حساب"))?;
            }
            if journals
                .get(index + 1)
                .is_some_and(|next| next.date == journal.date)
            {
                continue;
            }
            let mut debit = 0i128;
            let mut credit = 0i128;
            for balance in balances.values() {
                if *balance >= 0 {
                    debit = debit
                        .checked_add(*balance)
                        .ok_or_else(|| anyhow::anyhow!("سرریز تراز آزمایشی"))?;
                } else {
                    credit = credit
                        .checked_sub(*balance)
                        .ok_or_else(|| anyhow::anyhow!("سرریز تراز آزمایشی"))?;
                }
            }
            ensure!(
                debit <= i128::from(i64::MAX) && credit <= i128::from(i64::MAX),
                "ثبت باعث سرریز مانده یا جمع تراز آزمایشی در یک تاریخ می‌شود"
            );
            ensure!(debit == credit, "تراز آزمایشی نابرابر است");
        }
        Ok(())
    }
    fn check_reversal(&self, command: &PostCommand) -> Result<()> {
        if let JournalKind::Reversal { journal_id } = &command.kind {
            let original = self
                .journals
                .iter()
                .find(|j| &j.id == journal_id)
                .ok_or_else(|| anyhow::anyhow!("سند اصلی برگشت پیدا نشد"))?;
            ensure!(
                original.event != crate::core::CORE_OPENING_EVENT,
                "برگشت افتتاحیهٔ کنترلی به اصلاح هماهنگ جزئیات نیاز دارد؛ در این مرحله پشتیبانی نمی‌شود"
            );
            ensure!(
                original.company_id == command.company_id,
                "برگشت بین دو کسب‌وکار مجاز نیست"
            );
            ensure!(
                !matches!(original.kind, JournalKind::Reversal { .. }),
                "برگشتِ سند برگشتی پشتیبانی نمی‌شود"
            );
            ensure!(
                command.date >= original.date,
                "تاریخ برگشت نباید پیش از سند اصلی باشد"
            );
            ensure!(
                !self.period(&original.period_id)?.locked,
                "سال مالی سند اصلی قفل است"
            );
            ensure!(!self.journals.iter().any(|j| matches!(&j.kind, JournalKind::Reversal { journal_id: old } if old == journal_id)), "سند اصلی قبلاً برگشت شده است");
            let expected: Vec<_> = original
                .lines
                .iter()
                .map(|l| JournalLineInput {
                    account_id: l.account.id.clone(),
                    debit: l.credit,
                    credit: l.debit,
                    description: l.description.clone(),
                })
                .collect();
            ensure!(
                command.lines == expected
                    && command.source_id == original.id
                    && command.source_revision == 1
                    && command.event == "reversal",
                "برگشت باید اثر دقیق سند اصلی را معکوس کند"
            );
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn reverse(
        &mut self,
        journal_id: &str,
        period_id: &str,
        date: &str,
        request_key: &str,
        actor: &str,
        reason: &str,
    ) -> Result<String> {
        let original = self
            .journals
            .iter()
            .find(|j| j.id == journal_id)
            .ok_or_else(|| anyhow::anyhow!("سند اصلی پیدا نشد"))?;
        let command = PostCommand {
            company_id: original.company_id.clone(),
            period_id: period_id.into(),
            date: date.into(),
            source_id: original.id.clone(),
            source_revision: 1,
            event: "reversal".into(),
            request_key: request_key.into(),
            actor: actor.into(),
            reason: reason.into(),
            kind: JournalKind::Reversal {
                journal_id: original.id.clone(),
            },
            lines: original
                .lines
                .iter()
                .map(|l| JournalLineInput {
                    account_id: l.account.id.clone(),
                    debit: l.credit,
                    credit: l.debit,
                    description: l.description.clone(),
                })
                .collect(),
        };
        self.post(command)
    }
    pub fn trial_balance(&self, company_id: &str, as_of: &str) -> Result<TrialBalance> {
        self.company(company_id)?;
        iso_date(as_of)?;
        let mut totals: BTreeMap<String, i128> = BTreeMap::new();
        for j in self
            .journals
            .iter()
            .filter(|j| j.company_id == company_id && j.date.as_str() <= as_of)
        {
            for l in &j.lines {
                let prior = totals.entry(l.account.id.clone()).or_default();
                *prior = prior
                    .checked_add(i128::from(l.debit.0))
                    .and_then(|v| v.checked_sub(i128::from(l.credit.0)))
                    .ok_or_else(|| anyhow::anyhow!("سرریز ماندهٔ حساب"))?;
            }
        }
        let mut rows = vec![];
        let mut total_debit = Money::ZERO;
        let mut total_credit = Money::ZERO;
        for (account_id, net) in totals {
            let a = self
                .accounts
                .iter()
                .find(|a| a.id == account_id)
                .ok_or_else(|| anyhow::anyhow!("حساب گزارش پیدا نشد"))?;
            let (debit, credit) = if net >= 0 {
                (
                    Money::from_rials(
                        i64::try_from(net).map_err(|_| anyhow::anyhow!("سرریز ماندهٔ حساب"))?,
                    ),
                    Money::ZERO,
                )
            } else {
                (
                    Money::ZERO,
                    Money::from_rials(
                        i64::try_from(
                            net.checked_neg()
                                .ok_or_else(|| anyhow::anyhow!("سرریز ماندهٔ حساب"))?,
                        )
                        .map_err(|_| anyhow::anyhow!("سرریز ماندهٔ حساب"))?,
                    ),
                )
            };
            total_debit = total_debit.checked_add(debit)?;
            total_credit = total_credit.checked_add(credit)?;
            rows.push(TrialBalanceRow {
                account_id,
                code: a.code.clone(),
                name: a.name.clone(),
                debit,
                credit,
            });
        }
        rows.sort_by(|a, b| a.code.cmp(&b.code));
        ensure!(total_debit == total_credit, "تراز آزمایشی نابرابر است");
        Ok(TrialBalance {
            company_id: company_id.into(),
            as_of: as_of.into(),
            rows,
            total_debit,
            total_credit,
        })
    }
}
