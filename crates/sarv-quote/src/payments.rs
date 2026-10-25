//! Typed settlement of customer receivables and supplier payables.
//!
//! Payments move exact money between a cash/bank account and the operational
//! control accounts. Allocations are always tied to open items (invoices,
//! bills or opening open items); any surplus becomes an explicit advance.
//! Every command is idempotent through its request key and freezes the chart
//! setup exactly like the sales and procurement ledgers.
use crate::accounting::{AccountingState, JournalKind, JournalLineInput, Money, PostCommand};
use crate::core::{CoreRole, CoreSetup, civil_date};
use crate::domain::{id, text};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

pub const PAYMENTS_EVENT_PREFIX: &str = "payments.";
pub const PAYMENT_REVERSAL_EVENT: &str = "payments.reverse.v1";
pub type PaymentContext = crate::procurement::ProcurementContext;

macro_rules! record {($n:ident {$($f:ident:$t:ty),* $(,)?})=>{
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)] #[serde(deny_unknown_fields)] pub struct $n {$(pub $f:$t),*}
};}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentsState {
    pub records: Vec<PaymentRecord>,
    pub requests: Vec<PaymentRequest>,
}
impl PaymentsState {
    pub fn is_empty(&self) -> bool {
        self.records.is_empty() && self.requests.is_empty()
    }
    pub fn validate_transition(&self, next: &Self) -> Result<()> {
        ensure!(
            next.records.starts_with(&self.records) && next.requests.starts_with(&self.requests),
            "سابقهٔ پرداخت قابل تغییر یا حذف نیست"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentSide {
    /// Money received from a customer against invoices.
    Receive,
    /// Money paid to a supplier against bills.
    Make,
}
impl PaymentSide {
    pub fn key(self) -> &'static str {
        match self {
            Self::Receive => "receive",
            Self::Make => "make",
        }
    }
}

record!(PaymentAllocation {
    open_item_id: String,
    amount: Money
});
record!(PaymentInput {
    context: PaymentContext,
    side: PaymentSide,
    party_id: String,
    funds_role: CoreRole,
    reference: String,
    allocations: Vec<PaymentAllocation>,
    advance_amount: Money
});
record!(PaymentReversalInput {
    context: PaymentContext,
    payment_id: String
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentCommand {
    Payment(PaymentInput),
    Reversal(PaymentReversalInput),
}
impl PaymentCommand {
    pub fn context(&self) -> &PaymentContext {
        match self {
            Self::Payment(x) => &x.context,
            Self::Reversal(x) => &x.context,
        }
    }
    pub(crate) fn context_mut(&mut self) -> &mut PaymentContext {
        match self {
            Self::Payment(x) => &mut x.context,
            Self::Reversal(x) => &mut x.context,
        }
    }
    pub fn event(&self) -> &'static str {
        match self {
            Self::Payment(x) => match x.side {
                PaymentSide::Receive => "payments.receive.v1",
                PaymentSide::Make => "payments.make.v1",
            },
            Self::Reversal(_) => PAYMENT_REVERSAL_EVENT,
        }
    }
}
impl PaymentInput {
    pub fn applied(&self) -> Result<Money> {
        let mut total = Money::ZERO;
        for a in &self.allocations {
            total = total.checked_add(a.amount)?;
        }
        Ok(total)
    }
    pub fn total(&self) -> Result<Money> {
        self.applied()?.checked_add(self.advance_amount)
    }
}

record!(PostedPaymentAllocation {
    open_item_id: String,
    party_id: String,
    reference: String,
    document_date: String,
    due_date: String,
    amount: Money
});
record!(PostedPayment {
    side: PaymentSide,
    party: crate::core::CoreParty,
    reference: String,
    allocations: Vec<PostedPaymentAllocation>,
    applied: Money,
    advance_amount: Money,
    total: Money
});
record!(PostedPaymentReversal {
    payment_id: String,
    side: PaymentSide,
    funds_role: CoreRole,
    party: crate::core::CoreParty,
    reference: String,
    allocations: Vec<PostedPaymentAllocation>,
    applied: Money,
    advance_amount: Money,
    total: Money
});
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentData {
    Payment(PostedPayment),
    Reversal(PostedPaymentReversal),
}
record!(PaymentRecord {
    id: String,
    command: PaymentCommand,
    command_sha256: String,
    frozen_setup: CoreSetup,
    sequence: u64,
    journal_id: String,
    data: PaymentData
});
record!(PaymentRequest {
    request_key: String,
    company_id: String,
    record_id: String,
    business_sha256: String
});
record!(PaymentResult { record_id: String, journal_id: String, sequence: u64 });
record!(PaymentPreview {
    data: PaymentData,
    journal_lines: Vec<JournalLineInput>,
    total: Money
});
impl PaymentRecord {
    /// Funds account this payment moved money through.
    pub fn funds_role(&self) -> CoreRole {
        match (&self.command, &self.data) {
            (PaymentCommand::Payment(x), _) => x.funds_role,
            (_, PaymentData::Reversal(p)) => p.funds_role,
            _ => CoreRole::Cash,
        }
    }
}

fn hash<T: Serialize>(v: &T) -> Result<String> {
    Ok(crate::imports::hash(&serde_json::to_vec(v)?))
}
fn business_hash(c: &PaymentCommand) -> Result<String> {
    hash(&(c.event(), c))
}

fn role_lines(
    setup: &CoreSetup,
    rows: &[(CoreRole, bool, Money)],
    reference: &str,
) -> Result<Vec<JournalLineInput>> {
    let mut out = Vec::new();
    for (role, debit, amount) in rows {
        if *amount == Money::ZERO {
            continue;
        }
        out.push(JournalLineInput {
            account_id: setup.account_id(*role)?.into(),
            debit: if *debit { *amount } else { Money::ZERO },
            credit: if *debit { Money::ZERO } else { *amount },
            description: reference.into(),
        });
    }
    Ok(out)
}

/// One open item with its settleable remainder as-of a date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentOpenItem {
    pub id: String,
    pub party_id: String,
    pub reference: String,
    pub document_date: String,
    pub due_date: String,
    pub remaining: Money,
}

impl AccountingState {
    /// Net settlements of all open items as-of a date, in one pass.
    pub(crate) fn payment_allocation_map(
        &self,
        company: &str,
        asof: &str,
    ) -> Result<BTreeMap<String, Money>> {
        let mut out: BTreeMap<String, Money> = BTreeMap::new();
        for r in &self.payments.records {
            if r.command.context().company_id != company || r.command.context().date.as_str() > asof
            {
                continue;
            }
            match &r.data {
                PaymentData::Payment(p) => {
                    for a in &p.allocations {
                        let total = out.entry(a.open_item_id.clone()).or_insert(Money::ZERO);
                        *total = total.checked_sub(a.amount)?;
                    }
                }
                PaymentData::Reversal(p) => {
                    for a in &p.allocations {
                        let total = out.entry(a.open_item_id.clone()).or_insert(Money::ZERO);
                        *total = total.checked_add(a.amount)?;
                    }
                }
            }
        }
        Ok(out)
    }
    /// Net advance balance created by payments for one advance role as-of a date.
    pub fn payment_advance_delta(
        &self,
        company: &str,
        role: CoreRole,
        asof: &str,
    ) -> Result<Money> {
        let side = match role {
            CoreRole::CustomerAdvances => PaymentSide::Receive,
            CoreRole::SupplierAdvances => PaymentSide::Make,
            _ => return Ok(Money::ZERO),
        };
        let mut total = Money::ZERO;
        for r in &self.payments.records {
            if r.command.context().company_id != company || r.command.context().date.as_str() > asof {
                continue;
            }
            let (advance, positive, same_side) = match &r.data {
                PaymentData::Payment(p) => (p.advance_amount, true, p.side == side),
                PaymentData::Reversal(p) => (p.advance_amount, false, p.side == side),
            };
            if advance == Money::ZERO || !same_side {
                continue;
            }
            total = if positive {
                total.checked_add(advance)?
            } else {
                total.checked_sub(advance)?
            };
        }
        Ok(total)
    }
    /// Net effect of payments on one cash/bank account as-of a date.
    pub fn payment_fund_delta(
        &self,
        company: &str,
        account: &str,
        asof: &str,
    ) -> Result<Money> {
        let mut total = Money::ZERO;
        for r in &self.payments.records {
            if r.command.context().company_id != company || r.command.context().date.as_str() > asof {
                continue;
            }
            let (amount, funds_role, receive_in) = match &r.data {
                PaymentData::Payment(p) => (p.total, r.funds_role(), p.side == PaymentSide::Receive),
                PaymentData::Reversal(p) => {
                    (p.total, p.funds_role, p.side != PaymentSide::Receive)
                }
            };
            if amount == Money::ZERO {
                continue;
            }
            let hits = r
                .frozen_setup
                .command
                .mappings
                .iter()
                .any(|m| m.role == funds_role && m.account_id == account);
            if !hits {
                continue;
            }
            if receive_in {
                total = total.checked_add(amount)?;
            } else {
                total = total.checked_sub(amount)?;
            }
        }
        Ok(total)
    }
    pub fn payment_open_items(
        &self,
        company: &str,
        party: Option<&str>,
        side: PaymentSide,
        asof: &str,
    ) -> Result<Vec<PaymentOpenItem>> {
        civil_date(asof)?;
        let rows: Vec<PaymentOpenItem> = match side {
            PaymentSide::Receive => self
                .sales_receivables(company, asof)?
                .into_iter()
                .map(|r| PaymentOpenItem {
                    id: r.id,
                    party_id: r.party_id,
                    reference: r.reference,
                    document_date: r.document_date,
                    due_date: r.due_date,
                    remaining: r.remaining,
                })
                .collect(),
            PaymentSide::Make => self
                .procurement_open_payables(company, asof)?
                .into_iter()
                .map(|r| PaymentOpenItem {
                    id: r.id,
                    party_id: r.party_id,
                    reference: r.reference,
                    document_date: r.document_date,
                    due_date: r.due_date,
                    remaining: r.remaining,
                })
                .collect(),
        };
        Ok(rows
            .into_iter()
            .filter(|r| r.remaining.rials() > 0)
            .filter(|r| party.is_none_or(|p| r.party_id == p))
            .collect())
    }
    pub fn payment_open_item_list(
        &self,
        company: &str,
        party: &str,
        side: PaymentSide,
        asof: &str,
    ) -> Result<Vec<PaymentOpenItem>> {
        self.payment_open_items(company, Some(party), side, asof)
    }

    pub fn preview_payment(&self, command: &PaymentCommand) -> Result<PaymentPreview> {
        match command {
            PaymentCommand::Payment(input) => self.preview_payment_receive_make(input),
            PaymentCommand::Reversal(input) => {
                let c = &input.context;
                self.validate_operational_context(c)?;
                self.preview_payment_reversal(&c.company_id, &input.payment_id)
            }
        }
    }
    fn preview_payment_receive_make(&self, input: &PaymentInput) -> Result<PaymentPreview> {
        let c = &input.context;
        let setup = self.validate_operational_context(c)?;
        civil_date(&c.date)?;
        ensure!(
            matches!(input.funds_role, CoreRole::Cash | CoreRole::Bank),
            "حساب تسویه باید صندوق یا بانک باشد"
        );
        text(&input.reference)?;
        let resources = self.procurement_resources(&c.company_id)?;
        let party = resources
            .parties
            .iter()
            .find(|p| p.id == input.party_id)
            .cloned()
            .ok_or_else(|| anyhow!("طرف حساب معتبر نیست"))?;
        match input.side {
            PaymentSide::Receive => ensure!(party.customer, "طرف حساب نقش مشتری ندارد"),
            PaymentSide::Make => ensure!(party.supplier, "طرف حساب نقش تأمین‌کننده ندارد"),
        }
        ensure!(
            (1..=1000).contains(&input.allocations.len()),
            "تعداد ردیف تخصیص نامعتبر است"
        );
        ensure!(input.advance_amount.rials() >= 0, "پیش‌دریافت منفی مجاز نیست");
        let mut seen = HashSet::new();
        for a in &input.allocations {
            ensure!(a.amount.rials() > 0, "مبلغ تخصیص باید مثبت باشد");
            ensure!(seen.insert(a.open_item_id.as_str()), "ردیف باز تکراری است");
        }
        let applied = input.applied()?;
        let total = input.total()?;
        ensure!(total.rials() > 0, "مبلغ کل پرداخت باید مثبت باشد");
        let mut open: BTreeMap<String, Money> = self
            .payment_open_items(&c.company_id, Some(&input.party_id), input.side, &c.date)?
            .into_iter()
            .map(|i| (i.id, i.remaining))
            .collect();
        let mut frozen = Vec::new();
        for a in &input.allocations {
            let open_row = self
                .payment_open_item_by_id(&c.company_id, input.side, &a.open_item_id, &c.date)?
                .ok_or_else(|| anyhow!("ردیف باز معتبر نیست یا مانده ندارد"))?;
            let remaining = open
                .get_mut(&a.open_item_id)
                .ok_or_else(|| anyhow!("ردیف باز معتبر نیست یا مانده ندارد"))?;
            ensure!(
                a.amount.rials() <= remaining.rials(),
                "مبلغ تخصیص از ماندهٔ سند بیشتر است"
            );
            *remaining = remaining.checked_sub(a.amount)?;
            frozen.push(PostedPaymentAllocation {
                open_item_id: a.open_item_id.clone(),
                party_id: open_row.party_id,
                reference: open_row.reference,
                document_date: open_row.document_date,
                due_date: open_row.due_date,
                amount: a.amount,
            });
        }
        let (control_role, advance_role, funds_debit) = match input.side {
            PaymentSide::Receive => (CoreRole::Receivables, CoreRole::CustomerAdvances, true),
            PaymentSide::Make => (CoreRole::Payables, CoreRole::SupplierAdvances, false),
        };
        let rows = [
            (input.funds_role, funds_debit, total),
            (control_role, !funds_debit, applied),
            (advance_role, !funds_debit, input.advance_amount),
        ];
        let journal_lines = role_lines(setup, &rows, &input.reference)?;
        ensure!(!journal_lines.is_empty(), "سند پرداخت خالی است");
        Ok(PaymentPreview {
            data: PaymentData::Payment(PostedPayment {
                side: input.side,
                party,
                reference: input.reference.clone(),
                allocations: frozen,
                applied,
                advance_amount: input.advance_amount,
                total,
            }),
            journal_lines,
            total,
        })
    }
    fn payment_open_item_by_id(
        &self,
        company: &str,
        side: PaymentSide,
        open_item: &str,
        asof: &str,
    ) -> Result<Option<PaymentOpenItem>> {
        Ok(self
            .payment_open_items(company, None, side, asof)?
            .into_iter()
            .find(|i| i.id == open_item))
    }

    pub fn preview_payment_reversal(
        &self,
        company: &str,
        payment_id: &str,
    ) -> Result<PaymentPreview> {
        let original = self.payment_document(company, payment_id)?;
        let PaymentData::Payment(p) = &original.data else {
            anyhow::bail!("سند پرداخت قابل برگشت نیست")
        };
        let PaymentCommand::Payment(input) = &original.command else {
            anyhow::bail!("سند پرداخت قابل برگشت نیست")
        };
        ensure!(
            !self.payments.records.iter().any(|r| match &r.data {
                PaymentData::Reversal(v) => v.payment_id == payment_id,
                _ => false,
            }),
            "این پرداخت قبلاً برگشت خورده است"
        );
        let rows = match input.side {
            PaymentSide::Receive => [
                (input.funds_role, false, p.total),
                (CoreRole::Receivables, true, p.applied),
                (CoreRole::CustomerAdvances, true, p.advance_amount),
            ],
            PaymentSide::Make => [
                (input.funds_role, true, p.total),
                (CoreRole::Payables, false, p.applied),
                (CoreRole::SupplierAdvances, false, p.advance_amount),
            ],
        };
        let journal_lines = role_lines(&original.frozen_setup, &rows, &p.reference)?;
        Ok(PaymentPreview {
            data: PaymentData::Reversal(PostedPaymentReversal {
                payment_id: payment_id.into(),
                side: input.side,
                funds_role: input.funds_role,
                party: p.party.clone(),
                reference: p.reference.clone(),
                allocations: p.allocations.clone(),
                applied: p.applied,
                advance_amount: p.advance_amount,
                total: p.total,
            }),
            journal_lines,
            total: p.total,
        })
    }

    pub fn post_payment_command(&mut self, command: PaymentCommand) -> Result<PaymentResult> {
        let mut next = self.clone();
        let out = next.post_payment_inner(command)?;
        *self = next;
        Ok(out)
    }
    fn post_payment_inner(&mut self, command: PaymentCommand) -> Result<PaymentResult> {
        let c = command.context().clone();
        text(&c.request_key)?;
        let business = business_hash(&command)?;
        self.ensure_fresh_payment_key(&c.request_key)?;
        if let Some(alias) = self
            .payments
            .requests
            .iter()
            .find(|r| r.request_key == c.request_key)
        {
            ensure!(
                alias.company_id == c.company_id && alias.business_sha256 == business,
                "کلید درخواست با محتوای متفاوت استفاده شده است"
            );
            let prior = self.payment_document(&c.company_id, &alias.record_id)?;
            ensure!(
                prior.command_sha256 == hash(&prior.command)?
                    && business_hash(&prior.command)? == business
                    && self.core.setups.contains(&prior.frozen_setup),
                "تصویر سند پرداخت نامعتبر است"
            );
            return Ok(PaymentResult {
                record_id: prior.id.clone(),
                journal_id: prior.journal_id.clone(),
                sequence: prior.sequence,
            });
        }
        if let Some(prior) = self.payments.records.iter().find(|r| {
            r.command.context().company_id == c.company_id
                && r.command.context().source_id == c.source_id
                && r.command.event() == command.event()
        }) {
            ensure!(
                prior.command_sha256 == hash(&prior.command)?
                    && business_hash(&prior.command)? == business
                    && self.core.setups.contains(&prior.frozen_setup),
                "رویداد پرداخت قبلاً با محتوای متفاوت ثبت شده است"
            );
            if let PaymentCommand::Payment(p) = &command {
                let prior_input = match &prior.command {
                    PaymentCommand::Payment(x) => x,
                    _ => anyhow::bail!("نوع رویداد پرداخت تغییر کرده است"),
                };
                ensure!(
                    prior_input.party_id == p.party_id
                        && prior_input.reference == p.reference
                        && prior_input.allocations == p.allocations
                        && prior_input.advance_amount == p.advance_amount,
                    "رویداد پرداخت قبلاً با محتوای متفاوت ثبت شده است"
                );
            }
            self.payments.requests.push(PaymentRequest {
                request_key: c.request_key.clone(),
                company_id: c.company_id.clone(),
                record_id: prior.id.clone(),
                business_sha256: business,
            });
            return Ok(PaymentResult {
                record_id: prior.id.clone(),
                journal_id: prior.journal_id.clone(),
                sequence: prior.sequence,
            });
        }
        if let PaymentCommand::Reversal(reversal) = &command {
            let prior = self.payment_document(&c.company_id, &reversal.payment_id)?;
            ensure!(
                prior.command.context().date.as_str() <= c.date.as_str(),
                "تاریخ برگشت پرداخت پیش از خود پرداخت است"
            );
        }
        let preview = self.preview_payment(&command)?;
        let setup = self
            .core
            .setups
            .iter()
            .find(|s| s.id == c.setup_id)
            .ok_or_else(|| anyhow!("پیکربندی حساب‌ها یافت نشد"))?
            .clone();
        let record_id = id();
        ensure!(
            !self.operational_known_ids().contains(&record_id),
            "شناسهٔ رویداد پرداخت با منابع تداخل دارد"
        );
        let sequence = self.operational_sequence(&c.company_id)?;
        let command_sha256 = hash(&command)?;
        let journal_id = self.post_payment_journal(financial_command(
            &command,
            preview.journal_lines.clone(),
        ))?;
        let record = PaymentRecord {
            id: record_id.clone(),
            command,
            command_sha256,
            frozen_setup: setup,
            sequence,
            journal_id: journal_id.clone(),
            data: preview.data,
        };
        let out = PaymentResult {
            record_id,
            journal_id,
            sequence,
        };
        self.payments.records.push(record);
        self.payments.requests.push(PaymentRequest {
            request_key: c.request_key.clone(),
            company_id: c.company_id.clone(),
            record_id: out.record_id.clone(),
            business_sha256: business,
        });
        self.ensure_payment_controls(&c.company_id)?;
        Ok(out)
    }
    fn ensure_fresh_payment_key(&self, key: &str) -> Result<()> {
        // A payment retry reuses its own manual request record; the posting
        // kernel rejects genuinely new keys that collide with manual commands.
        ensure!(
            !self.core.zero_requests.iter().any(|r| r.request_key == key)
                && !self.procurement.requests.iter().any(|r| r.request_key == key)
                && !self.sales.requests.iter().any(|r| r.request_key == key),
            "کلید درخواست قبلاً استفاده شده است"
        );
        Ok(())
    }

    pub fn payment_document(&self, company: &str, id: &str) -> Result<&PaymentRecord> {
        self.payments
            .records
            .iter()
            .find(|r| r.id == id && r.command.context().company_id == company)
            .ok_or_else(|| anyhow!("سند پرداخت در این شرکت یافت نشد"))
    }
    pub(crate) fn payment_entity_ids(&self) -> Vec<&str> {
        self.payments
            .records
            .iter()
            .map(|r| r.id.as_str())
            .collect()
    }
    /// Reconstruct one posted payment during integrity replay.
    pub(crate) fn replay_payment_record(&mut self, record: &PaymentRecord) -> Result<String> {
        text(&record.id)?;
        ensure!(
            !self.operational_known_ids().contains(&record.id),
            "شناسهٔ سوابق تغییرات پرداخت تکراری است"
        );
        let before = self.payments.records.len();
        let out = self.post_payment_command(record.command.clone())?;
        ensure!(
            self.payments.records.len() == before + 1,
            "سوابق تغییرات رویداد پرداخت تکراری است"
        );
        let created = self.payments.records.last_mut().unwrap();
        created.id = record.id.clone();
        self.payments.requests.last_mut().unwrap().record_id = record.id.clone();
        let mut expected = record.clone();
        expected.journal_id = out.journal_id.clone();
        ensure!(
            *created == expected,
            "محتوا یا تخصیص‌های رویداد پرداخت نامعتبر است"
        );
        Ok(out.journal_id)
    }
    pub(crate) fn replay_payment_request(
        &mut self,
        request: &PaymentRequest,
    ) -> Result<PaymentResult> {
        let prior = self.payment_document(&request.company_id, &request.record_id)?;
        ensure!(
            request.company_id == prior.command.context().company_id
                && request.business_sha256 == business_hash(&prior.command)?,
            "هویت یا هش درخواست پرداخت نامعتبر است"
        );
        let mut c = prior.command.clone();
        c.context_mut().request_key = request.request_key.clone();
        self.post_payment_command(c)
    }
    fn ensure_payment_controls(&self, company: &str) -> Result<()> {
        ensure!(
            self.procurement_reconcile(company, "9999-12-31")?.reconciled,
            "ثبت پرداخت باعث اختلاف حساب کنترلی می‌شود"
        );
        Ok(())
    }
    pub(crate) fn post_payment_journal(&mut self, command: PostCommand) -> Result<String> {
        ensure!(
            command.event.starts_with(PAYMENTS_EVENT_PREFIX)
                && command.kind == JournalKind::Manual,
            "رویداد پرداخت فقط از گردش کار معتبر مجاز است"
        );
        self.post_internal(command, true, false)
    }
}

pub(crate) fn financial_command(
    c: &PaymentCommand,
    lines: Vec<JournalLineInput>,
) -> PostCommand {
    let x = c.context();
    PostCommand {
        company_id: x.company_id.clone(),
        period_id: x.period_id.clone(),
        date: x.date.clone(),
        source_id: x.source_id.clone(),
        source_revision: x.source_revision,
        event: c.event().into(),
        request_key: x.request_key.clone(),
        actor: x.actor.clone(),
        reason: x.reason.clone(),
        kind: JournalKind::Manual,
        lines,
    }
}
