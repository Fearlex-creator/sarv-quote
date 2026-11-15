# 0011 — Bank statement reconciliation

**Status:** proposed · **Date:** 2026-11-15

## Context

`README.md` describes a per-account statement reconciliation dialog, which existed only as client-side state in the removed React shell. The native Rune desktop (`crates/sarv-desktop/src/ui/views/banking.rs`) has no reconciliation UI, and `crates/sarv-quote` has no persisted register to track which bank/cash journal lines have been matched against a bank statement.

This design spike (Plan 029) establishes the data model, accounting invariants, and two-phase implementation plan for bank statement reconciliation.

### Findings from the codebase

- **Journal line identity is stable**: `Journal` (`crates/sarv-quote/src/accounting.rs:189-202`) stores `lines: Vec<JournalLine>` (`accounting.rs:181-186`) without per-line IDs, and `AccountingState::validate_transition` (`accounting.rs:707`) enforces `next.journals.starts_with(&self.journals)`. Posted journals and their line ordering are immutable. Therefore `(journal_id, line_index)` is a stable, canonical line identity.
- **`banking` and `ledger` balances agree**: `banking` (`crates/sarv-quote/src/desktop_contacts_banking.rs:117-190`) computes closing balance via `trial_balance(to)` and splits `opening` (`j.date < from`) from `incoming`/`outgoing` (`from <= j.date <= to`). `ledger_rows` (`crates/sarv-quote/src/desktop_bridge.rs:2176-2229`) computes `opening_balance` (`j.date < from`) and running line balances (`from <= j.date <= to`). The prototype test (`crates/sarv-quote/tests/reconciliation_prototype.rs`) confirms that `banking` and `ledger` agree on opening, movements, and closing balances across deposits, withdrawals, and reversals.
- **Additive register precedent**: `ChequeRegister` (`crates/sarv-quote/src/cheques.rs:127-142`, ADR-0005) is an append-only register on `State` (`crates/sarv-quote/src/domain.rs:843-844`) with `#[serde(default, skip_serializing_if = "...")]`, validated on load and append-checked on save, with zero side effects on `AccountingState`.

## Decision

### 1. Unit of reconciliation

A reconciliation session is scoped to `(company_id, account_id)` for a `statement_date` (`YYYY-MM-DD`) and `statement_balance: Money` (integer rials).

The matchable items are **individual `JournalLine`s** on `account_id` with `journal.date <= statement_date`, identified by `LineRef { journal_id, line_index }`. Matching at the line level (rather than the journal level) handles compound journals that touch the same bank account more than once.

### 2. Additive register and append-only events

Following ADR-0005, reconciliation state lives in a strictly additive `ReconciliationRegister` on `State` (`crates/sarv-quote/src/domain.rs`), skipped when empty. Deleting `reconciliations` leaves `AccountingState` and all journals untouched.

We choose an **append-only event log** (`sessions` + `events`) over a mutable record with a revision counter. History cannot be silently rewritten (`Store` enforces `starts_with` in `validate_transition`), and reopening/re-editing a session is audited as explicit `Reopened` / `LineMatched` / `LineUnmatched` / `Completed` events.

```rust
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
```

### 3. Arithmetic and completion rule

- **Book balance** (as of `statement_date`): $\sum (\text{debit} - \text{credit})$ for all journal lines on `(company_id, account_id)` with `journal.date <= statement_date`.
- **Previous reconciled balance**: `cleared_balance` of the immediately preceding `Completed` session for `(company_id, account_id)` (or `0` for the first session).
- **Cleared balance**: `previous_reconciled_balance + Σ (line.debit - line.credit)` over the set of currently matched `LineRef`s in the session.
- **Difference**: `statement_balance - cleared_balance`.
- **Completion rule**: A session may transition to `Completed` **only when `difference == 0`**. If the bank statement contains an unrecorded fee or interest, the user posts it through the existing `banking_preview` + `banking_post` workflow ("Money out" / "Money in") and matches the resulting journal line. The reconciliation feature never invents journal lines.

### 4. Reversals

`AccountingState::reverse` (`crates/sarv-quote/src/accounting.rs:1316-1363`) appends a new `Journal` with `JournalKind::Reversal { journal_id }` and swapped debit/credit lines.

- **Same-period reversal**: Both the original line and the reversal line are unmatched in the `Open` session and visually linked/badged. If neither appeared on the bank statement, leaving both unmatched nets to `0`; if both appeared (e.g., a returned transfer), matching both also nets to `0`.
- **Prior-period reversal**: If the original line was matched in an earlier `Completed` session, it remains locked there; only the reversal line appears as matchable in the current `Open` session.

### 5. Cheques

Per ADR-0005 (`crates/sarv-quote/src/cheques.rs`), receiving, issuing, or depositing a cheque does **not** post to the bank account (`ChequeAct::Deposited` is status-only). Deposited-but-uncleared cheques are not matchable ledger lines; they may be surfaced in an informational "Pending deposited cheques" list for operator context. When a cheque clears (`ChequeAct::Cleared`), `banking.cheque_clear.v1` posts the bank line, which immediately becomes a normal matchable `LineRef`.

### 6. Validation rules on load (`State::validate`)

1. **Append-only transition (`Store`)**: `next.sessions.starts_with(&self.sessions)` and `next.events.starts_with(&self.events)`.
2. **Company and account**: Every `Reconciliation` references an existing `company_id` and an existing postable cash/bank `account_id` in that company.
3. **Dates**: `statement_date` is a valid ISO civil date (`YYYY-MM-DD`).
4. **Event integrity**: Events for a `reconciliation_id` share its `company_id`, begin with `ReconciliationAct::Started`, and have unique non-empty `request_key`s.
5. **State machine**: Folding events yields valid transitions: `Started` -> `Open`; `LineMatched` / `LineUnmatched` only while `Open` (no double-match or unmatch of an unmatched line); `Completed` only from `Open`; `Reopened` only from `Completed`.
6. **Journal line references**: Every `LineRef` in `LineMatched` / `LineUnmatched` points to an existing `Journal` in `accounting.journals` for `company_id` with `line_index < journal.lines.len()`, `journal.lines[line_index].account.id == account_id`, and `journal.date <= statement_date`.
7. **Uniqueness across sessions**: A `LineRef` is matched in at most one `Completed` session, and at most one `Open` session exists per `(company_id, account_id)` at any time.
8. **Session ordering and continuity**: `Completed` sessions for `(company_id, account_id)` are strictly ordered by `statement_date`, and each session's `previous_reconciled_balance` equals the prior completed session's `cleared_balance` (or `0` for the first).
9. **Zero difference on completion**: At every `Completed` event, `previous_reconciled_balance + Σ matched_deltas == statement_balance`.

### 7. Desktop bridge actions

- `reconciliation` (read): returns the `Open` session (if any), `Completed` history, unmatched book lines up to `statement_date`, `book_balance`, `cleared_balance`, `difference`, and pending deposited cheques.
- Mutating actions (all require `request_key` for idempotent replay and `expected_generation` from the prior `reconciliation` read, wired into `desktop_bridge.rs:1256-1272`):
  - `reconciliation_start`: opens a new session for `(company_id, account_id)` if no `Open` session exists and `statement_date` is after the latest `Completed` session's `statement_date`.
  - `reconciliation_set_matched`: diffs the submitted `matched_lines: Vec<LineRef>` against the `Open` session's current matched set and appends `LineMatched` / `LineUnmatched` events.
  - `reconciliation_complete`: verifies `difference == 0` and appends `Completed { cleared_balance }`.
  - `reconciliation_reopen`: allowed only for the **latest** `Completed` session on `(company_id, account_id)` when no `Open` session exists, appending `Reopened { reason }`.

### 8. Phase 2: Statement import

Follows the `opening_csv_preview` / `opening_csv_post` pattern (`crates/sarv-quote/src/core_import.rs`, `crates/sarv-quote/src/imports.rs`) with row/column diagnostics:

1. **Column mapping**: user maps CSV columns (`date`, `description`, `debit`, `credit` or signed `amount`, optional `reference`/`balance`). Bank-specific presets are stored as **data** (configuration), not per-bank code.
2. **Persian localization**: parse Jalali/Gregorian dates via `calendar::parse_date` and Persian/Arabic-Indic digits via `iran::ids::normalize_digits`.
3. **Preview & suggested matches** (`reconciliation_import_preview`): returns parsed statement rows, diagnostics, and suggested matches (matching `debit - credit` and date within ±N days) for one-by-one user confirmation.
4. **Unmatched statement rows**: offer "Record as Money in / Money out", pre-filling the existing `banking_preview` + `banking_post` dialog.

### 9. UI (describe only)

- **Banking screen** (`crates/sarv-desktop/src/ui/views/banking.rs`): add a per-account "Reconcile" action.
- **Two-pane reconciliation sheet**:
  - *Left pane*: book entries (`JournalLine`s up to `statement_date`) with checkboxes, date, reference, description, bidi-isolated debit/credit, and reversal/cheque badges.
  - *Right pane*: `statement_date`, `statement_balance` input, `previous_reconciled_balance`, `cleared_balance`, `difference`, "Complete" button (enabled only at `difference == 0`), and a "Record bank fee / interest" shortcut opening the Banking Money out/in dialog.
- **History**: list of `Completed` reconciliations with a printable summary (statement balance, cleared balance, matched count, and outstanding items as of `statement_date`).

### 10. Backups and schema

- **No schema bump**: Following ADR-0005 (`CURRENT_SCHEMA = 7`, `sarv-accounting-backup-v7`), `State::reconciliations` uses `#[serde(default, skip_serializing_if = "ReconciliationRegister::is_empty")]`. Existing v7 databases and backups without `reconciliations` load and validate unchanged.
- **Migration & backup test plan**:
  - Round-trip `backup_state` / `Store::restore_preview` with empty and populated `reconciliations`.
  - Verify a v7 JSON fixture without the `reconciliations` key loads cleanly.
  - Tamper tests: history rewrite (`validate_transition` fails), non-existent journal/line, wrong account, line matched in two completed sessions, and completion with `difference != 0` all fail `State::validate()`.

### Implementation outline

#### Phase 1 — Manual reconciliation (Effort: M)
1. **Slice 1A — Register & state machine (`crates/sarv-quote/src/reconciliation.rs`, `domain.rs`)**: `ReconciliationRegister`, event folding, `State::validate` rules, `validate_transition`. *Tests*: unit tests for folding, tamper detection, double-match rejection, zero-difference enforcement, and v7 backup round-trip.
2. **Slice 1B — Desktop bridge actions (`desktop_bridge.rs`)**: `reconciliation`, `reconciliation_start`, `reconciliation_set_matched`, `reconciliation_complete`, `reconciliation_reopen` with `expected_generation` and `request_key` idempotency. *Tests*: integration tests covering full lifecycle, reopen, reversals, cheque clearing, and stale-generation rejection.
3. **Slice 1C — Rune desktop UI (`crates/sarv-desktop/src/ui/views/banking.rs`)**: "Reconcile" entry point, two-pane reconciliation sheet, completed history + printable summary. *Tests*: headless desktop tests + `png_scenes` snapshot.

#### Phase 2 — Statement CSV import (Effort: L)
1. **Slice 2A — CSV parser & presets (`crates/sarv-quote/src/reconciliation_import.rs`)**: column-mapping parser, `calendar::parse_date`, `iran::ids::normalize_digits`, row/column diagnostics, data-driven bank presets. *Tests*: Jalali dates, Persian digits, malformed rows, signed vs split debit/credit columns.
2. **Slice 2B — Match suggestion & bridge (`desktop_bridge.rs`)**: `reconciliation_import_preview` returning suggested `(statement_row, LineRef)` pairs within ±N days. *Tests*: exact match, ambiguous duplicate amounts, date-window boundary.
3. **Slice 2C — Import UI (`crates/sarv-desktop`)**: file picker, column-mapping step, match-review table, "Record as Money in/out" pre-fill into Banking dialog.

## Consequences

- **Additive & reversible**: All reconciliation state is isolated from `AccountingState`; deleting `reconciliations` never corrupts the ledger.
- **No kernel changes**: Stable `(journal_id, line_index)` avoids adding IDs to `JournalLine` or bumping `CURRENT_SCHEMA`.
- **Single source of truth**: Missing bank fees/interest go through `banking_post`, preserving auditability and trial balance invariants.

## Open questions for the owner

1. **Reopening older sessions**: Should `reconciliation_reopen` be restricted strictly to the most recent `Completed` session for an account, or should reopening an older session be allowed (cascade-reopening newer sessions)?
2. **Zero-difference enforcement**: Is `difference == 0` mandatory for completion, or is an explicit "unreconciled difference" write-off workflow needed?
3. **Opening balance lines**: When starting the very first reconciliation on an account, should `core_openings` lines be left for the user to match manually or auto-matched?
4. **Phase 2 bank presets**: Which 2–3 Iranian bank CSV/Excel export formats (e.g. Mellat, Saman, Pasargad, Melli) should be prioritized as initial data presets?
5. **Phase 2 date tolerance**: What default day window (±N days, e.g. ±5 days) should be used for suggested statement matches, and should it be user-configurable?
