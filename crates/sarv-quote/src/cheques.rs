//! Cheque register (ADR-0005): an append-only log of cheque instruments and lifecycle events.
//!
//! The register owns no money. Every ledger effect is produced by existing audited sources and
//! the register only links to it:
//! - receiving / issuing / endorsing a cheque is a payment whose funds are the cheques notes
//!   account (`payments.*`), so invoice open items stay consistent;
//! - bouncing, returning or voiding reverses that payment;
//! - clearing is a `banking.cheque_clear.v1` movement between the notes account and a bank;
//! - depositing changes status only (spec §3.6: the in-collection step is optional).
//!
//! Legal transitions and posting shapes come from the pure machine in `iran::cheque`; `validate`
//! re-derives every status from the events and proves each claimed ledger effect exists.
use crate::{
    accounting::{JournalKind, JournalLineInput, Money, PostCommand},
    core::{CoreRole, CoreSetup},
    domain::{State, id, now},
    iran::{
        cheque::{self as machine, Action, Direction, Role},
        ids,
    },
    payments::{
        PaymentCommand, PaymentContext, PaymentData, PaymentInput, PaymentReversalInput,
        PaymentSide,
    },
};
use anyhow::{Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const CHEQUE_CLEAR_EVENT: &str = "banking.cheque_clear.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChequeDir {
    Received,
    Issued,
}
impl ChequeDir {
    fn machine(self) -> Direction {
        match self {
            Self::Received => Direction::Received,
            Self::Issued => Direction::Issued,
        }
    }
    fn notes_role(self) -> CoreRole {
        match self {
            Self::Received => CoreRole::ChequesReceivable,
            Self::Issued => CoreRole::ChequesPayable,
        }
    }
    fn payment_side(self) -> PaymentSide {
        match self {
            Self::Received => PaymentSide::Receive,
            Self::Issued => PaymentSide::Make,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChequeAct {
    Registered,
    Deposited,
    Cleared,
    Bounced,
    Spent,
    Returned,
    Voided,
}
impl ChequeAct {
    fn machine(self) -> Option<Action> {
        Some(match self {
            Self::Registered => return None,
            Self::Deposited => Action::Deposit,
            Self::Cleared => Action::Clear,
            Self::Bounced => Action::Bounce,
            Self::Spent => Action::Spend,
            Self::Returned => Action::ReturnToCustomer,
            Self::Voided => Action::Void,
        })
    }
}

/// What the user types about the instrument.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ChequeDetails {
    pub sayad_id: String,
    pub number: String,
    pub bank: String,
    pub branch: String,
    pub due_date: String,
    pub drawer_national_id: String,
    pub purpose: String,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cheque {
    pub id: String,
    pub company_id: String,
    pub direction: ChequeDir,
    pub amount: Money,
    pub party_id: String,
    pub details: ChequeDetails,
    /// Set when the cheque was on hand at the opening; it then has no originating payment.
    pub opening_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChequeEvent {
    pub id: String,
    pub cheque_id: String,
    pub company_id: String,
    pub action: ChequeAct,
    pub date: String,
    pub actor: String,
    pub reason: String,
    pub request_key: String,
    /// Registered: the receiving/issuing payment. Spent: the endorsement payment.
    /// Bounced / Returned / Voided: the reversal of the originating payment.
    pub payment_id: Option<String>,
    /// Cleared: the banking journal.
    pub journal_id: Option<String>,
    pub recorded_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChequeRegister {
    pub cheques: Vec<Cheque>,
    pub events: Vec<ChequeEvent>,
}
impl ChequeRegister {
    pub fn is_empty(&self) -> bool {
        self.cheques.is_empty() && self.events.is_empty()
    }
    /// History is append-only: nothing already recorded may change or disappear.
    pub fn validate_transition(&self, next: &Self) -> Result<()> {
        ensure!(
            next.cheques.starts_with(&self.cheques) && next.events.starts_with(&self.events),
            "سابقهٔ چک‌ها قابل تغییر یا حذف نیست"
        );
        Ok(())
    }
    pub fn cheque(&self, id: &str) -> Result<&Cheque> {
        self.cheques
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| anyhow!("چک پیدا نشد"))
    }
    fn events_of<'a>(&'a self, cheque_id: &'a str) -> impl Iterator<Item = &'a ChequeEvent> {
        self.events.iter().filter(move |e| e.cheque_id == cheque_id)
    }
    /// Current lifecycle state, derived by folding the events through the state machine.
    pub fn status(&self, cheque_id: &str) -> Result<machine::State> {
        let cheque = self.cheque(cheque_id)?;
        let amount = cheque.amount.rials();
        let mut state = machine::open(cheque.direction.machine(), amount)?.0;
        let mut first = true;
        for e in self.events_of(cheque_id) {
            if first {
                ensure!(
                    e.action == ChequeAct::Registered,
                    "رویداد نخست چک باید ثبت باشد"
                );
                first = false;
                continue;
            }
            let action = e.action.machine().ok_or_else(|| anyhow!("ثبت تکراری چک"))?;
            state = machine::apply(cheque.direction.machine(), state, action, amount)?.0;
        }
        ensure!(!first, "چک بدون رویداد ثبت");
        Ok(state)
    }
    fn by_request(&self, key: &str) -> Option<&ChequeEvent> {
        self.events.iter().find(|e| e.request_key == key)
    }
}

fn account_for(setup: &CoreSetup, role: Role, fund_account: &str) -> Result<String> {
    Ok(match role {
        Role::Bank => fund_account.to_owned(),
        Role::NotesReceivable | Role::NotesInCollection => {
            setup.account_id(CoreRole::ChequesReceivable)?.into()
        }
        Role::NotesPayable => setup.account_id(CoreRole::ChequesPayable)?.into(),
        Role::AccountsReceivable => setup.account_id(CoreRole::Receivables)?.into(),
        Role::AccountsPayable => setup.account_id(CoreRole::Payables)?.into(),
    })
}

fn validate_details(d: &ChequeDetails, require_sayad: bool) -> Result<()> {
    if !d.sayad_id.is_empty() || require_sayad {
        ids::sayad_id(&d.sayad_id).map_err(|e| anyhow!("شناسهٔ صیاد نامعتبر است: {e}"))?;
    }
    if !d.drawer_national_id.is_empty() {
        ensure!(
            d.drawer_national_id.chars().count() <= 32
                && !d.drawer_national_id.chars().any(char::is_control),
            "شناسهٔ صادرکننده نامعتبر است"
        );
    }
    for t in [&d.number, &d.bank, &d.branch, &d.purpose] {
        ensure!(
            t.chars().count() <= 160 && !t.chars().any(char::is_control),
            "فیلد چک نامعتبر است"
        );
    }
    ensure!(d.notes.chars().count() <= 2000, "یادداشت چک طولانی است");
    ensure!(
        chrono::NaiveDate::parse_from_str(&d.due_date, "%Y-%m-%d").is_ok(),
        "سررسید چک باید تاریخ میلادی YYYY-MM-DD باشد"
    );
    Ok(())
}

impl State {
    fn push_event(
        &mut self,
        cheque_id: &str,
        ctx: &PaymentContext,
        action: ChequeAct,
        payment_id: Option<String>,
        journal_id: Option<String>,
    ) {
        self.cheques.events.push(ChequeEvent {
            id: id(),
            cheque_id: cheque_id.into(),
            company_id: ctx.company_id.clone(),
            action,
            date: ctx.date.clone(),
            actor: ctx.actor.clone(),
            reason: ctx.reason.clone(),
            request_key: ctx.request_key.clone(),
            payment_id,
            journal_id,
            recorded_at: now(),
        });
    }
    fn cheque_for_action(
        &self,
        cheque_id: &str,
        ctx: &PaymentContext,
    ) -> Result<(Cheque, machine::State)> {
        let cheque = self.cheques.cheque(cheque_id)?.clone();
        ensure!(
            cheque.company_id == ctx.company_id,
            "چک متعلق به این کسب‌وکار نیست"
        );
        let status = self.cheques.status(cheque_id)?;
        Ok((cheque, status))
    }

    /// Receive a customer cheque: posts the receipt (funds = cheques receivable) and registers it.
    pub fn receive_cheque(
        &mut self,
        details: ChequeDetails,
        mut payment: PaymentInput,
    ) -> Result<String> {
        self.register_with_payment(ChequeDir::Received, details, &mut payment)
    }
    /// Issue a cheque to a supplier: posts the payment (funds = cheques payable) and registers it.
    pub fn issue_cheque(
        &mut self,
        details: ChequeDetails,
        mut payment: PaymentInput,
    ) -> Result<String> {
        self.register_with_payment(ChequeDir::Issued, details, &mut payment)
    }
    fn register_with_payment(
        &mut self,
        direction: ChequeDir,
        details: ChequeDetails,
        payment: &mut PaymentInput,
    ) -> Result<String> {
        let ctx = payment.context.clone();
        if let Some(prior) = self.cheques.by_request(&ctx.request_key) {
            return Ok(prior.cheque_id.clone());
        }
        ensure!(
            payment.side == direction.payment_side(),
            "جهت پرداخت با نوع چک سازگار نیست"
        );
        payment.funds_role = direction.notes_role();
        validate_details(&details, true)?;
        let amount = payment.total()?;
        ensure!(amount.rials() > 0, "مبلغ چک باید مثبت باشد");
        ensure!(
            !self
                .cheques
                .cheques
                .iter()
                .any(|c| c.company_id == ctx.company_id && c.details.sayad_id == details.sayad_id),
            "شناسهٔ صیاد تکراری است"
        );
        let party_id = payment.party_id.clone();
        let result = self
            .accounting
            .post_payment_command(PaymentCommand::Payment(payment.clone()))?;
        let cheque = Cheque {
            id: id(),
            company_id: ctx.company_id.clone(),
            direction,
            amount,
            party_id,
            details,
            opening_id: None,
        };
        let cheque_id = cheque.id.clone();
        self.cheques.cheques.push(cheque);
        self.push_event(
            &cheque_id,
            &ctx,
            ChequeAct::Registered,
            Some(result.record_id),
            None,
        );
        Ok(cheque_id)
    }

    /// Bring a cheque that was on hand at the opening into the register.
    pub fn register_opening_cheque(
        &mut self,
        ctx: &PaymentContext,
        opening_id: &str,
        details: ChequeDetails,
    ) -> Result<String> {
        if let Some(prior) = self.cheques.by_request(&ctx.request_key) {
            return Ok(prior.cheque_id.clone());
        }
        validate_details(&details, false)?;
        let opening = self
            .accounting
            .core
            .openings
            .iter()
            .find(|o| o.command.company_id == ctx.company_id)
            .ok_or_else(|| anyhow!("ابتدا افتتاحیه را تأیید کنید"))?;
        let oc = opening
            .command
            .cheques
            .iter()
            .find(|c| c.id == opening_id)
            .ok_or_else(|| anyhow!("چک افتتاحیه پیدا نشد"))?;
        ensure!(
            !self
                .cheques
                .cheques
                .iter()
                .any(|c| c.opening_id.as_deref() == Some(opening_id)),
            "این چک افتتاحیه قبلاً ثبت شده است"
        );
        let direction = match oc.direction {
            crate::core::ChequeDirection::Receivable => ChequeDir::Received,
            crate::core::ChequeDirection::Payable => ChequeDir::Issued,
        };
        let cheque = Cheque {
            id: id(),
            company_id: ctx.company_id.clone(),
            direction,
            amount: oc.amount,
            party_id: oc.party_id.clone(),
            details,
            opening_id: Some(opening_id.into()),
        };
        let cheque_id = cheque.id.clone();
        self.cheques.cheques.push(cheque);
        self.push_event(&cheque_id, ctx, ChequeAct::Registered, None, None);
        Ok(cheque_id)
    }

    /// Hand a received cheque to the bank for collection. Status only: no journal.
    pub fn deposit_cheque(&mut self, ctx: &PaymentContext, cheque_id: &str) -> Result<()> {
        if self.cheques.by_request(&ctx.request_key).is_some() {
            return Ok(());
        }
        let (cheque, status) = self.cheque_for_action(cheque_id, ctx)?;
        machine::apply(
            cheque.direction.machine(),
            status,
            Action::Deposit,
            cheque.amount.rials(),
        )?;
        self.push_event(cheque_id, ctx, ChequeAct::Deposited, None, None);
        Ok(())
    }

    /// The bank paid (received) or paid out (issued) the cheque: post the clearing movement.
    pub fn clear_cheque(
        &mut self,
        ctx: &PaymentContext,
        cheque_id: &str,
        fund_account_id: &str,
    ) -> Result<String> {
        if let Some(prior) = self.cheques.by_request(&ctx.request_key) {
            return prior
                .journal_id
                .clone()
                .ok_or_else(|| anyhow!("درخواست تکراری با نتیجهٔ ناسازگار"));
        }
        let (cheque, status) = self.cheque_for_action(cheque_id, ctx)?;
        let (_, lines) = machine::apply(
            cheque.direction.machine(),
            status,
            Action::Clear,
            cheque.amount.rials(),
        )?;
        let setup = self.accounting.validate_operational_context(ctx)?.clone();
        let lines = lines
            .iter()
            .map(|l| {
                Ok(JournalLineInput {
                    account_id: account_for(&setup, l.role, fund_account_id)?,
                    debit: Money::from_rials(l.debit_rial),
                    credit: Money::from_rials(l.credit_rial),
                    description: ctx.reason.clone(),

                    dimension: String::new(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let journal_id = self.accounting.post_banking(PostCommand {
            company_id: ctx.company_id.clone(),
            period_id: ctx.period_id.clone(),
            date: ctx.date.clone(),
            source_id: cheque.id.clone(),
            source_revision: 1,
            event: CHEQUE_CLEAR_EVENT.into(),
            request_key: ctx.request_key.clone(),
            actor: ctx.actor.clone(),
            reason: ctx.reason.clone(),
            kind: JournalKind::Manual,
            lines,
        })?;
        self.push_event(
            cheque_id,
            ctx,
            ChequeAct::Cleared,
            None,
            Some(journal_id.clone()),
        );
        Ok(journal_id)
    }

    /// Bounce, return or void: reverse the originating payment so invoice items re-open.
    pub fn reverse_cheque(
        &mut self,
        ctx: &PaymentContext,
        cheque_id: &str,
        act: ChequeAct,
    ) -> Result<()> {
        ensure!(
            matches!(
                act,
                ChequeAct::Bounced | ChequeAct::Returned | ChequeAct::Voided
            ),
            "عملیات برگشت نامعتبر است"
        );
        if self.cheques.by_request(&ctx.request_key).is_some() {
            return Ok(());
        }
        let (cheque, status) = self.cheque_for_action(cheque_id, ctx)?;
        machine::apply(
            cheque.direction.machine(),
            status,
            act.machine().ok_or_else(|| anyhow!("عملیات نامعتبر"))?,
            cheque.amount.rials(),
        )?;
        let origin = self
            .cheques
            .events_of(cheque_id)
            .find(|e| e.action == ChequeAct::Registered)
            .and_then(|e| e.payment_id.clone())
            .ok_or_else(|| {
                anyhow!("چک افتتاحیه سند پرداخت ندارد؛ برگشت یا ابطال آن پشتیبانی نمی‌شود")
            })?;
        let reversal = self
            .accounting
            .post_payment_command(PaymentCommand::Reversal(PaymentReversalInput {
                context: ctx.clone(),
                payment_id: origin,
            }))?;
        self.push_event(cheque_id, ctx, act, Some(reversal.record_id), None);
        Ok(())
    }

    /// Endorse a received cheque to a supplier: a supplier payment funded by cheques receivable.
    pub fn spend_cheque(&mut self, cheque_id: &str, mut payment: PaymentInput) -> Result<()> {
        let ctx = payment.context.clone();
        if self.cheques.by_request(&ctx.request_key).is_some() {
            return Ok(());
        }
        let (cheque, status) = self.cheque_for_action(cheque_id, &ctx)?;
        ensure!(
            cheque.direction == ChequeDir::Received,
            "فقط چک دریافتی قابل خرج‌کردن است"
        );
        machine::apply(
            cheque.direction.machine(),
            status,
            Action::Spend,
            cheque.amount.rials(),
        )?;
        ensure!(
            payment.side == PaymentSide::Make,
            "خرج چک باید پرداخت به تأمین‌کننده باشد"
        );
        payment.funds_role = CoreRole::ChequesReceivable;
        ensure!(
            payment.total()? == cheque.amount,
            "مبلغ پرداخت باید با مبلغ چک برابر باشد"
        );
        let result = self
            .accounting
            .post_payment_command(PaymentCommand::Payment(payment))?;
        self.push_event(
            cheque_id,
            &ctx,
            ChequeAct::Spent,
            Some(result.record_id),
            None,
        );
        Ok(())
    }

    /// Prove the register: every status derives from its events through the state machine and
    /// every claimed ledger effect exists with the right accounts, sides and amount.
    pub(crate) fn validate_cheques(&self) -> Result<()> {
        let reg = &self.cheques;
        let mut cheque_ids = HashSet::new();
        let mut sayads = HashSet::new();
        let mut openings = HashSet::new();
        let mut claimed = HashSet::new();
        let mut keys = HashSet::new();
        for c in &reg.cheques {
            ensure!(cheque_ids.insert(&c.id), "شناسهٔ چک تکراری است");
            ensure!(c.amount.rials() > 0, "مبلغ چک باید مثبت باشد");
            ensure!(
                self.accounting
                    .companies
                    .iter()
                    .any(|x| x.id == c.company_id),
                "کسب‌وکار چک پیدا نشد"
            );
            validate_details(&c.details, c.opening_id.is_none())?;
            if !c.details.sayad_id.is_empty() {
                ensure!(
                    sayads.insert((&c.company_id, &c.details.sayad_id)),
                    "شناسهٔ صیاد تکراری است"
                );
            }
            if let Some(o) = &c.opening_id {
                ensure!(openings.insert(o), "چک افتتاحیه دوباره ثبت شده است");
                let oc = self
                    .accounting
                    .core
                    .openings
                    .iter()
                    .filter(|op| op.command.company_id == c.company_id)
                    .flat_map(|op| op.command.cheques.iter())
                    .find(|x| &x.id == o)
                    .ok_or_else(|| anyhow!("چک افتتاحیهٔ مرجع پیدا نشد"))?;
                let same_dir = matches!(
                    (oc.direction, c.direction),
                    (
                        crate::core::ChequeDirection::Receivable,
                        ChequeDir::Received
                    ) | (crate::core::ChequeDirection::Payable, ChequeDir::Issued)
                );
                ensure!(
                    same_dir && oc.amount == c.amount && oc.party_id == c.party_id,
                    "چک با سطر افتتاحیه سازگار نیست"
                );
            }
        }
        for e in &reg.events {
            ensure!(keys.insert(&e.request_key), "کلید درخواست چک تکراری است");
            ensure!(
                e.company_id == reg.cheque(&e.cheque_id)?.company_id,
                "رویداد چک متعلق به کسب‌وکار دیگری است"
            );
        }
        for c in &reg.cheques {
            let status = reg.status(&c.id)?; // proves the transition order
            let _ = status;
            let notes_role = c.direction.notes_role();
            let mut registered = false;
            let mut origin: Option<&str> = None;
            for e in reg.events_of(&c.id) {
                let setup = self
                    .accounting
                    .core
                    .setups
                    .iter()
                    .find(|s| s.command.company_id == c.company_id)
                    .ok_or_else(|| anyhow!("تنظیمات حساب پیدا نشد"))?;
                let notes_account = setup.account_id(notes_role)?;
                match e.action {
                    ChequeAct::Registered => {
                        ensure!(!registered, "ثبت تکراری چک");
                        registered = true;
                        match (&c.opening_id, &e.payment_id) {
                            (Some(_), None) => {}
                            (None, Some(pid)) => {
                                ensure!(
                                    claimed.insert(pid.clone()),
                                    "سند پرداخت چند بار به چک‌ها نسبت داده شده است"
                                );
                                let r = self.accounting.payment_document(&c.company_id, pid)?;
                                let PaymentData::Payment(p) = &r.data else {
                                    bail!("سند ثبت چک باید پرداخت باشد")
                                };
                                ensure!(
                                    p.side == c.direction.payment_side()
                                        && r.funds_role() == notes_role
                                        && p.total == c.amount
                                        && p.party.id == c.party_id,
                                    "پرداخت مرتبط با چک سازگار نیست"
                                );
                                origin = Some(pid);
                            }
                            _ => bail!("منبع ثبت چک نامعتبر است"),
                        }
                    }
                    ChequeAct::Deposited => {}
                    ChequeAct::Cleared => {
                        let jid = e
                            .journal_id
                            .as_deref()
                            .ok_or_else(|| anyhow!("وصول چک سند ندارد"))?;
                        ensure!(
                            claimed.insert(jid.to_owned()),
                            "سند به چند چک نسبت داده شده است"
                        );
                        let rec = self
                            .accounting
                            .banking
                            .iter()
                            .find(|b| b.journal_id == jid)
                            .ok_or_else(|| anyhow!("سند وصول چک در دفتر بانکی نیست"))?;
                        ensure!(
                            rec.command.event == CHEQUE_CLEAR_EVENT
                                && rec.command.source_id == c.id
                                && rec.command.company_id == c.company_id,
                            "سند وصول به این چک تعلق ندارد"
                        );
                        let notes = rec
                            .command
                            .lines
                            .iter()
                            .find(|l| l.account_id == notes_account)
                            .ok_or_else(|| anyhow!("سند وصول حساب اسناد را ندارد"))?;
                        let (d, cr) = (notes.debit.rials(), notes.credit.rials());
                        let expected = match c.direction {
                            ChequeDir::Received => (0, c.amount.rials()),
                            ChequeDir::Issued => (c.amount.rials(), 0),
                        };
                        ensure!((d, cr) == expected, "مبلغ یا سمت سند وصول چک نامعتبر است");
                    }
                    ChequeAct::Bounced | ChequeAct::Returned | ChequeAct::Voided => {
                        let pid = e
                            .payment_id
                            .as_deref()
                            .ok_or_else(|| anyhow!("برگشت چک سند ندارد"))?;
                        ensure!(
                            claimed.insert(pid.to_owned()),
                            "سند به چند چک نسبت داده شده است"
                        );
                        let r = self.accounting.payment_document(&c.company_id, pid)?;
                        let PaymentData::Reversal(rev) = &r.data else {
                            bail!("برگشت چک باید ابطال پرداخت باشد")
                        };
                        ensure!(
                            Some(rev.payment_id.as_str()) == origin && rev.total == c.amount,
                            "ابطال به پرداخت چک مربوط نیست"
                        );
                    }
                    ChequeAct::Spent => {
                        let pid = e
                            .payment_id
                            .as_deref()
                            .ok_or_else(|| anyhow!("خرج چک سند ندارد"))?;
                        ensure!(
                            claimed.insert(pid.to_owned()),
                            "سند به چند چک نسبت داده شده است"
                        );
                        let r = self.accounting.payment_document(&c.company_id, pid)?;
                        let PaymentData::Payment(p) = &r.data else {
                            bail!("خرج چک باید پرداخت باشد")
                        };
                        ensure!(
                            p.side == PaymentSide::Make
                                && r.funds_role() == CoreRole::ChequesReceivable
                                && p.total == c.amount,
                            "پرداخت خرج چک سازگار نیست"
                        );
                    }
                }
            }
            ensure!(registered, "چک بدون رویداد ثبت");
        }
        // A payment the register relies on may only be reversed through the register, so a
        // cheque's status can never disagree with its ledger effect.
        let relied_on: HashSet<&str> = reg
            .events
            .iter()
            .filter(|e| matches!(e.action, ChequeAct::Registered | ChequeAct::Spent))
            .filter_map(|e| e.payment_id.as_deref())
            .collect();
        for r in &self.accounting.payments.records {
            if let PaymentData::Reversal(rev) = &r.data
                && relied_on.contains(rev.payment_id.as_str())
            {
                ensure!(
                    claimed.contains(r.id.as_str()),
                    "پرداخت مربوط به چک باید از طریق چک برگشت یا ابطال شود"
                );
            }
        }
        Ok(())
    }
}
