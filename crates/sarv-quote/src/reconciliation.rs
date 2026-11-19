//! Append-only bank statement reconciliation register (ADR-0011).
//! Strictly additive on `State` (no schema bump); deleting reconciliations never alters the ledger.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use anyhow::{Result, bail, ensure};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::accounting::{AccountingState, JournalKind, Money};
use crate::domain::text;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconciliationStatus {
    Open,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineRef {
    pub journal_id: String,
    pub line_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reconciliation {
    pub id: String,
    pub company_id: String,
    pub account_id: String,
    pub statement_date: String,
    pub statement_balance: Money,
    pub previous_reconciled_balance: Money,
    pub created_at: String,
    pub created_by: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconciliationAct {
    Started,
    LineMatched { line: LineRef },
    LineUnmatched { line: LineRef },
    Completed { cleared_balance: Money },
    Reopened { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationEvent {
    pub id: String,
    pub reconciliation_id: String,
    pub company_id: String,
    pub action: ReconciliationAct,
    pub actor: String,
    pub reason: String,
    pub request_key: String,
    pub recorded_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationRegister {
    pub sessions: Vec<Reconciliation>,
    pub events: Vec<ReconciliationEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFold {
    pub status: ReconciliationStatus,
    pub matched: BTreeSet<LineRef>,
    pub cleared_balance: Money,
}

#[derive(Clone, Debug, Serialize)]
pub struct MatchableLine {
    pub journal_id: String,
    pub line_index: usize,
    pub date: String,
    pub description: String,
    pub debit: Money,
    pub credit: Money,
    pub matched: bool,
    pub is_reversal: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReconciliationView {
    pub open_session: Option<Reconciliation>,
    pub completed: Vec<Reconciliation>,
    pub lines: Vec<MatchableLine>,
    pub book_balance: Money,
    pub previous_reconciled_balance: Money,
    pub cleared_balance: Money,
    pub difference: Money,
}

fn line_delta(accounting: &AccountingState, company_id: &str, account_id: &str, statement_date: &str, line: &LineRef) -> Result<Money> {
    let journal = accounting
        .journals
        .iter()
        .find(|j| j.id == line.journal_id && j.company_id == company_id)
        .ok_or_else(|| anyhow::anyhow!("Reconciliation references missing journal {}", line.journal_id))?;
    ensure!(
        journal.date.as_str() <= statement_date,
        "Matched journal line {} is dated after statement_date",
        line.journal_id
    );
    let jl = journal
        .lines
        .get(line.line_index)
        .ok_or_else(|| anyhow::anyhow!("Reconciliation line index out of bounds"))?;
    ensure!(jl.account.id == account_id, "Reconciliation line belongs to a different account");
    jl.debit.checked_sub(jl.credit)
}

impl ReconciliationRegister {
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty() && self.events.is_empty()
    }

    pub fn validate_transition(&self, next: &Self) -> Result<()> {
        ensure!(next.sessions.starts_with(&self.sessions), "Reconciliation sessions are append-only");
        ensure!(next.events.starts_with(&self.events), "Reconciliation events are append-only");
        Ok(())
    }

    pub fn fold_session(&self, accounting: &AccountingState, session: &Reconciliation) -> Result<SessionFold> {
        let mut status: Option<ReconciliationStatus> = None;
        let mut matched: BTreeSet<LineRef> = BTreeSet::new();
        for ev in self.events.iter().filter(|e| e.reconciliation_id == session.id) {
            ensure!(ev.company_id == session.company_id, "Reconciliation event company mismatch");
            match &ev.action {
                ReconciliationAct::Started => {
                    ensure!(status.is_none(), "Duplicate Started event for reconciliation");
                    status = Some(ReconciliationStatus::Open);
                }
                ReconciliationAct::LineMatched { line } => {
                    ensure!(status == Some(ReconciliationStatus::Open), "Can only match lines while Open");
                    let _ = line_delta(accounting, &session.company_id, &session.account_id, &session.statement_date, line)?;
                    ensure!(matched.insert(line.clone()), "Line already matched in session");
                }
                ReconciliationAct::LineUnmatched { line } => {
                    ensure!(status == Some(ReconciliationStatus::Open), "Can only unmatch lines while Open");
                    ensure!(matched.remove(line), "Cannot unmatch a line that is not matched");
                }
                ReconciliationAct::Completed { cleared_balance } => {
                    ensure!(status == Some(ReconciliationStatus::Open), "Can only complete an Open session");
                    let computed = Self::compute_cleared(accounting, session, &matched)?;
                    ensure!(
                        computed == *cleared_balance && computed == session.statement_balance,
                        "Reconciliation can only complete when difference is zero"
                    );
                    status = Some(ReconciliationStatus::Completed);
                }
                ReconciliationAct::Reopened { .. } => {
                    ensure!(
                        status == Some(ReconciliationStatus::Completed),
                        "Can only reopen a Completed session"
                    );
                    status = Some(ReconciliationStatus::Open);
                }
            }
        }
        let status = status.ok_or_else(|| anyhow::anyhow!("Reconciliation {} missing Started event", session.id))?;
        let cleared_balance = Self::compute_cleared(accounting, session, &matched)?;
        Ok(SessionFold {
            status,
            matched,
            cleared_balance,
        })
    }

    fn compute_cleared(accounting: &AccountingState, session: &Reconciliation, matched: &BTreeSet<LineRef>) -> Result<Money> {
        let mut bal = session.previous_reconciled_balance;
        for lr in matched {
            let delta = line_delta(accounting, &session.company_id, &session.account_id, &session.statement_date, lr)?;
            bal = bal.checked_add(delta)?;
        }
        Ok(bal)
    }

    pub fn validate(&self, accounting: &AccountingState) -> Result<()> {
        let mut seen_session_ids = HashSet::new();
        let mut seen_keys = HashSet::new();
        for ev in &self.events {
            ensure!(!ev.request_key.trim().is_empty(), "Reconciliation event requires request_key");
            ensure!(seen_keys.insert(ev.request_key.clone()), "Duplicate reconciliation request_key");
            ensure!(
                self.sessions.iter().any(|s| s.id == ev.reconciliation_id),
                "Reconciliation event references unknown session"
            );
        }

        let mut open_per_account: HashSet<(String, String)> = HashSet::new();
        let mut completed_lines: HashSet<(String, String, LineRef)> = HashSet::new();
        let mut last_completed: BTreeMap<(String, String), (String, Money)> = BTreeMap::new();

        for s in &self.sessions {
            ensure!(seen_session_ids.insert(s.id.clone()), "Duplicate reconciliation session id");
            ensure!(
                accounting.companies.iter().any(|c| c.id == s.company_id),
                "Reconciliation references unknown company"
            );
            ensure!(
                accounting
                    .accounts
                    .iter()
                    .any(|a| a.id == s.account_id && a.company_id == s.company_id && a.postable),
                "Reconciliation references unknown or non-postable account"
            );
            crate::calendar::parse_date(&s.statement_date, crate::calendar::Calendar::Gregorian)?;

            let fold = self.fold_session(accounting, s)?;
            let acct_key = (s.company_id.clone(), s.account_id.clone());
            match fold.status {
                ReconciliationStatus::Open => {
                    ensure!(
                        open_per_account.insert(acct_key.clone()),
                        "At most one Open reconciliation session per account"
                    );
                }
                ReconciliationStatus::Completed => {
                    if let Some((prev_date, prev_bal)) = last_completed.get(&acct_key) {
                        ensure!(
                            s.statement_date.as_str() >= prev_date.as_str(),
                            "Completed reconciliations must be ordered by statement_date"
                        );
                        ensure!(
                            s.previous_reconciled_balance == *prev_bal,
                            "Previous reconciled balance discontinuity"
                        );
                    } else {
                        ensure!(
                            s.previous_reconciled_balance == Money::ZERO,
                            "First reconciliation must start from zero previous balance"
                        );
                    }
                    for lr in &fold.matched {
                        ensure!(
                            completed_lines.insert((s.company_id.clone(), s.account_id.clone(), lr.clone())),
                            "Journal line matched in multiple completed reconciliations"
                        );
                    }
                    last_completed.insert(acct_key, (s.statement_date.clone(), fold.cleared_balance));
                }
            }
        }
        Ok(())
    }

    pub fn view_for_account(
        &self,
        accounting: &AccountingState,
        company_id: &str,
        account_id: &str,
        as_of: &str,
    ) -> Result<ReconciliationView> {
        let mut open_session: Option<Reconciliation> = None;
        let mut open_fold: Option<SessionFold> = None;
        let mut completed: Vec<Reconciliation> = Vec::new();
        let mut already_reconciled: HashSet<LineRef> = HashSet::new();
        let mut prev_bal = Money::ZERO;

        for s in self
            .sessions
            .iter()
            .filter(|s| s.company_id == company_id && s.account_id == account_id)
        {
            let fold = self.fold_session(accounting, s)?;
            match fold.status {
                ReconciliationStatus::Open => {
                    open_session = Some(s.clone());
                    open_fold = Some(fold);
                }
                ReconciliationStatus::Completed => {
                    for lr in &fold.matched {
                        already_reconciled.insert(lr.clone());
                    }
                    prev_bal = fold.cleared_balance;
                    completed.push(s.clone());
                }
            }
        }

        let cutoff = open_session.as_ref().map(|s| s.statement_date.as_str()).unwrap_or(as_of);
        let mut book_balance = Money::ZERO;
        let mut lines = Vec::new();
        for j in accounting
            .journals
            .iter()
            .filter(|j| j.company_id == company_id && j.date.as_str() <= cutoff)
        {
            let is_reversal = matches!(j.kind, JournalKind::Reversal { .. });
            for (idx, jl) in j.lines.iter().enumerate() {
                if jl.account.id != account_id {
                    continue;
                }
                let delta = jl.debit.checked_sub(jl.credit)?;
                book_balance = book_balance.checked_add(delta)?;
                let lr = LineRef {
                    journal_id: j.id.clone(),
                    line_index: idx,
                };
                if already_reconciled.contains(&lr) {
                    continue;
                }
                let matched = open_fold.as_ref().is_some_and(|f| f.matched.contains(&lr));
                lines.push(MatchableLine {
                    journal_id: j.id.clone(),
                    line_index: idx,
                    date: j.date.clone(),
                    description: if jl.description.trim().is_empty() {
                        j.reason.clone()
                    } else {
                        jl.description.clone()
                    },
                    debit: jl.debit,
                    credit: jl.credit,
                    matched,
                    is_reversal,
                });
            }
        }

        let (previous_reconciled_balance, cleared_balance, difference) = if let (Some(os), Some(of)) = (&open_session, &open_fold) {
            (
                os.previous_reconciled_balance,
                of.cleared_balance,
                os.statement_balance.checked_sub(of.cleared_balance)?,
            )
        } else {
            (prev_bal, prev_bal, Money::ZERO)
        };

        Ok(ReconciliationView {
            open_session,
            completed,
            lines,
            book_balance,
            previous_reconciled_balance,
            cleared_balance,
            difference,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start(
        &mut self,
        accounting: &AccountingState,
        company_id: &str,
        account_id: &str,
        statement_date: &str,
        statement_balance: Money,
        actor: &str,
        request_key: &str,
    ) -> Result<String> {
        text(request_key)?;
        if let Some(ev) = self.events.iter().find(|e| e.request_key == request_key) {
            return Ok(ev.reconciliation_id.clone());
        }
        crate::calendar::parse_date(statement_date, crate::calendar::Calendar::Gregorian)?;
        let view = self.view_for_account(accounting, company_id, account_id, statement_date)?;
        ensure!(
            view.open_session.is_none(),
            "An open reconciliation session already exists for this account"
        );
        if let Some(last) = view.completed.last() {
            ensure!(
                statement_date >= last.statement_date.as_str(),
                "Statement date must be on or after the last completed reconciliation"
            );
        }
        let id = format!("rec-{}", Uuid::new_v4().simple());
        let now = Utc::now().to_rfc3339();
        self.sessions.push(Reconciliation {
            id: id.clone(),
            company_id: company_id.into(),
            account_id: account_id.into(),
            statement_date: statement_date.into(),
            statement_balance,
            previous_reconciled_balance: view.previous_reconciled_balance,
            created_at: now.clone(),
            created_by: actor.into(),
        });
        self.events.push(ReconciliationEvent {
            id: format!("recev-{}", Uuid::new_v4().simple()),
            reconciliation_id: id.clone(),
            company_id: company_id.into(),
            action: ReconciliationAct::Started,
            actor: actor.into(),
            reason: "Start bank reconciliation".into(),
            request_key: request_key.into(),
            recorded_at: now,
        });
        Ok(id)
    }

    pub fn set_matched(
        &mut self,
        accounting: &AccountingState,
        reconciliation_id: &str,
        desired: Vec<LineRef>,
        actor: &str,
        request_key: &str,
    ) -> Result<()> {
        text(request_key)?;
        if self
            .events
            .iter()
            .any(|e| e.request_key == request_key || e.request_key.starts_with(&format!("{request_key}:")))
        {
            return Ok(());
        }
        let session = self
            .sessions
            .iter()
            .find(|s| s.id == reconciliation_id)
            .ok_or_else(|| anyhow::anyhow!("Reconciliation session not found"))?
            .clone();
        let fold = self.fold_session(accounting, &session)?;
        ensure!(fold.status == ReconciliationStatus::Open, "Reconciliation session is not open");

        let target: BTreeSet<LineRef> = desired.into_iter().collect();
        let now = Utc::now().to_rfc3339();
        let mut seq = 0usize;
        for lr in fold.matched.difference(&target) {
            seq += 1;
            self.events.push(ReconciliationEvent {
                id: format!("recev-{}", Uuid::new_v4().simple()),
                reconciliation_id: session.id.clone(),
                company_id: session.company_id.clone(),
                action: ReconciliationAct::LineUnmatched { line: lr.clone() },
                actor: actor.into(),
                reason: "Unmatch statement line".into(),
                request_key: format!("{request_key}:{seq}"),
                recorded_at: now.clone(),
            });
        }
        for lr in target.difference(&fold.matched) {
            let _ = line_delta(accounting, &session.company_id, &session.account_id, &session.statement_date, lr)?;
            seq += 1;
            self.events.push(ReconciliationEvent {
                id: format!("recev-{}", Uuid::new_v4().simple()),
                reconciliation_id: session.id.clone(),
                company_id: session.company_id.clone(),
                action: ReconciliationAct::LineMatched { line: lr.clone() },
                actor: actor.into(),
                reason: "Match statement line".into(),
                request_key: format!("{request_key}:{seq}"),
                recorded_at: now.clone(),
            });
        }
        Ok(())
    }

    pub fn complete(&mut self, accounting: &AccountingState, reconciliation_id: &str, actor: &str, request_key: &str) -> Result<()> {
        text(request_key)?;
        if self.events.iter().any(|e| e.request_key == request_key) {
            return Ok(());
        }
        let session = self
            .sessions
            .iter()
            .find(|s| s.id == reconciliation_id)
            .ok_or_else(|| anyhow::anyhow!("Reconciliation session not found"))?
            .clone();
        let fold = self.fold_session(accounting, &session)?;
        ensure!(fold.status == ReconciliationStatus::Open, "Reconciliation session is not open");
        let diff = session.statement_balance.checked_sub(fold.cleared_balance)?;
        if diff != Money::ZERO {
            bail!(
                "Reconciliation difference must be zero before completing (difference: {})",
                diff.rials()
            );
        }
        self.events.push(ReconciliationEvent {
            id: format!("recev-{}", Uuid::new_v4().simple()),
            reconciliation_id: session.id,
            company_id: session.company_id,
            action: ReconciliationAct::Completed {
                cleared_balance: fold.cleared_balance,
            },
            actor: actor.into(),
            reason: "Complete bank reconciliation".into(),
            request_key: request_key.into(),
            recorded_at: Utc::now().to_rfc3339(),
        });
        Ok(())
    }

    pub fn reopen(
        &mut self,
        accounting: &AccountingState,
        reconciliation_id: &str,
        actor: &str,
        reason: &str,
        request_key: &str,
    ) -> Result<()> {
        text(request_key)?;
        if self.events.iter().any(|e| e.request_key == request_key) {
            return Ok(());
        }
        let session = self
            .sessions
            .iter()
            .find(|s| s.id == reconciliation_id)
            .ok_or_else(|| anyhow::anyhow!("Reconciliation session not found"))?
            .clone();
        let view = self.view_for_account(accounting, &session.company_id, &session.account_id, &session.statement_date)?;
        ensure!(view.open_session.is_none(), "Close the open reconciliation before reopening");
        ensure!(
            view.completed.last().map(|s| s.id.as_str()) == Some(reconciliation_id),
            "Only the latest completed reconciliation can be reopened"
        );
        let note = if reason.trim().is_empty() {
            "Reopen bank reconciliation".to_string()
        } else {
            reason.trim().to_string()
        };
        self.events.push(ReconciliationEvent {
            id: format!("recev-{}", Uuid::new_v4().simple()),
            reconciliation_id: session.id,
            company_id: session.company_id,
            action: ReconciliationAct::Reopened { reason: note.clone() },
            actor: actor.into(),
            reason: note,
            request_key: request_key.into(),
            recorded_at: Utc::now().to_rfc3339(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounting::{AccountClass, BalanceSide, JournalKind, JournalLineInput, NewAccount, PostCommand};
    use crate::domain::State;

    fn sample_state_with_bank_lines() -> (State, String, String) {
        let mut st = State::default();
        let company_id = st.accounting.add_company("Test Co", "tester").expect("company");
        let period_id = st
            .accounting
            .add_period(&company_id, "FY2026", "2026-01-01", "2026-12-31", "tester")
            .expect("period");
        let bank_id = st
            .accounting
            .add_account(
                NewAccount {
                    company_id: company_id.clone(),
                    parent_id: None,
                    code: "1120".into(),
                    name: "Bank".into(),
                    classification: AccountClass::Asset,
                    normal_balance: BalanceSide::Debit,
                    postable: true,
                    control: false,
                },
                "tester",
            )
            .expect("bank");
        let capital_id = st
            .accounting
            .add_account(
                NewAccount {
                    company_id: company_id.clone(),
                    parent_id: None,
                    code: "3100".into(),
                    name: "Capital".into(),
                    classification: AccountClass::Equity,
                    normal_balance: BalanceSide::Credit,
                    postable: true,
                    control: false,
                },
                "tester",
            )
            .expect("capital");
        let expense_id = st
            .accounting
            .add_account(
                NewAccount {
                    company_id: company_id.clone(),
                    parent_id: None,
                    code: "5200".into(),
                    name: "Bank Fees".into(),
                    classification: AccountClass::Expense,
                    normal_balance: BalanceSide::Debit,
                    postable: true,
                    control: false,
                },
                "tester",
            )
            .expect("expense");

        // Deposit +100_000 on 2026-03-01
        st.accounting
            .post(PostCommand {
                company_id: company_id.clone(),
                period_id: period_id.clone(),
                date: "2026-03-01".into(),
                source_id: "src-1".into(),
                source_revision: 1,
                event: "manual".into(),
                request_key: "j-1".into(),
                actor: "tester".into(),
                reason: "Owner deposit".into(),
                kind: JournalKind::Manual,
                lines: vec![
                    JournalLineInput {
                        account_id: bank_id.clone(),
                        debit: Money::from_rials(100_000),
                        credit: Money::ZERO,
                        description: "Deposit".into(),
                    },
                    JournalLineInput {
                        account_id: capital_id,
                        debit: Money::ZERO,
                        credit: Money::from_rials(100_000),
                        description: "Capital".into(),
                    },
                ],
            })
            .expect("deposit");

        // Expense -25_000 on 2026-03-05
        st.accounting
            .post(PostCommand {
                company_id: company_id.clone(),
                period_id,
                date: "2026-03-05".into(),
                source_id: "src-2".into(),
                source_revision: 1,
                event: "manual".into(),
                request_key: "j-2".into(),
                actor: "tester".into(),
                reason: "Bank fee".into(),
                kind: JournalKind::Manual,
                lines: vec![
                    JournalLineInput {
                        account_id: expense_id,
                        debit: Money::from_rials(25_000),
                        credit: Money::ZERO,
                        description: "Fee".into(),
                    },
                    JournalLineInput {
                        account_id: bank_id.clone(),
                        debit: Money::ZERO,
                        credit: Money::from_rials(25_000),
                        description: "Bank".into(),
                    },
                ],
            })
            .expect("fee");

        (st, company_id, bank_id)
    }

    #[test]
    fn start_match_complete_and_reopen_lifecycle() {
        let (mut st, company_id, bank_id) = sample_state_with_bank_lines();
        let before = st.reconciliations.clone();

        let rec_id = st
            .reconciliations
            .start(
                &st.accounting,
                &company_id,
                &bank_id,
                "2026-03-31",
                Money::from_rials(75_000),
                "tester",
                "rec-start-1",
            )
            .expect("start");

        let view = st
            .reconciliations
            .view_for_account(&st.accounting, &company_id, &bank_id, "2026-03-31")
            .expect("view");
        assert_eq!(view.book_balance, Money::from_rials(75_000));
        assert_eq!(view.lines.len(), 2);
        assert_eq!(view.difference, Money::from_rials(75_000));

        // Cannot complete while difference != 0
        assert!(
            st.reconciliations
                .complete(&st.accounting, &rec_id, "tester", "rec-done-fail")
                .is_err()
        );

        // Match both lines -> difference becomes 0
        let refs: Vec<LineRef> = view
            .lines
            .iter()
            .map(|l| LineRef {
                journal_id: l.journal_id.clone(),
                line_index: l.line_index,
            })
            .collect();
        st.reconciliations
            .set_matched(&st.accounting, &rec_id, refs, "tester", "rec-match-1")
            .expect("match");
        let view2 = st
            .reconciliations
            .view_for_account(&st.accounting, &company_id, &bank_id, "2026-03-31")
            .expect("view2");
        assert_eq!(view2.difference, Money::ZERO);
        assert_eq!(view2.cleared_balance, Money::from_rials(75_000));

        // Complete succeeds
        st.reconciliations
            .complete(&st.accounting, &rec_id, "tester", "rec-done-1")
            .expect("complete");
        st.reconciliations.validate(&st.accounting).expect("valid");
        before.validate_transition(&st.reconciliations).expect("append-only");

        // Lines matched in completed session no longer appear as unreconciled
        let view3 = st
            .reconciliations
            .view_for_account(&st.accounting, &company_id, &bank_id, "2026-04-30")
            .expect("view3");
        assert!(view3.open_session.is_none());
        assert_eq!(view3.completed.len(), 1);
        assert_eq!(view3.previous_reconciled_balance, Money::from_rials(75_000));
        assert!(view3.lines.is_empty());

        // Reopen latest completed session
        st.reconciliations
            .reopen(&st.accounting, &rec_id, "Found missed fee", "tester", "rec-reopen-1")
            .expect("reopen");
        let view4 = st
            .reconciliations
            .view_for_account(&st.accounting, &company_id, &bank_id, "2026-03-31")
            .expect("view4");
        assert!(view4.open_session.is_some());
        assert!(view4.completed.is_empty());
        assert_eq!(view4.lines.len(), 2);
        st.reconciliations.validate(&st.accounting).expect("valid after reopen");
    }

    #[test]
    fn validate_transition_rejects_deleted_or_mutated_events() {
        let (mut st, company_id, bank_id) = sample_state_with_bank_lines();
        st.reconciliations
            .start(
                &st.accounting,
                &company_id,
                &bank_id,
                "2026-03-31",
                Money::from_rials(75_000),
                "tester",
                "rec-start-1",
            )
            .expect("start");
        let after_start = st.reconciliations.clone();

        let mut tampered = after_start.clone();
        tampered.events.pop();
        assert!(after_start.validate_transition(&tampered).is_err());

        let mut mutated = after_start.clone();
        mutated.events[0].reason = "Tampered".into();
        assert!(after_start.validate_transition(&mutated).is_err());
    }
}
