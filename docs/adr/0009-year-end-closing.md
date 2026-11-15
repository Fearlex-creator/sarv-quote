# 0009 — Year-end closing (moving temporary account balances into retained earnings)

**Status:** proposed · **Date:** 2026-11-15

## Context

Once a business can open a second fiscal year (Plan 017, `period_add`), income and expense accounts keep accumulating across fiscal years unless the prior year is closed. Today the engine has no closing entry:

- `CoreRole::RetainedEarnings` exists (`crates/sarv-quote/src/core.rs:113`), but nothing posts to it.
- `report` (`crates/sarv-quote/src/desktop_bridge.rs:735-826`) computes P&L rows by summing `line.debit - line.credit` across **all** journals with `date` in `from..=to` (`desktop_bridge.rs:744-755`).
- `balance_sheet` (`crates/sarv-quote/src/statements.rs:39-94`) builds on `AccountingState::trial_balance(company, as_of)` (`crates/sarv-quote/src/accounting.rs:1364-1410`), which sums all journals with `date <= as_of`. It groups `AccountClass::Revenue | AccountClass::Expense` into `current_earnings` ("Revenue minus expense not yet closed into equity by a year-end entry", `statements.rs:28`).
- `AccountingState::post_internal` (`crates/sarv-quote/src/accounting.rs:1126-1143`) allows only one `JournalKind::Opening` per company ("re-transfer of balances not supported") and forbids manual journals (`control_allowed == false`) from touching control accounts (`accounting.rs:1167-1170`), which includes `CoreRole::Cogs` and `CoreRole::InventoryLoss` (`core.rs:219-235`).
- `period_lock` (`desktop_bridge.rs:1800-1830`) sets `FiscalPeriod::locked` and runs `s.validate()` when locking, but posts no closing entry.

`docs/ACCOUNTING-SPEC.md` specifies:
- Legal books with "بستن حساب‌های موقت، سند اختتامیه" and "سند افتتاحیه" (§3.2, line 144).
- "Year-end close wizard (temporary accounts → P&L summary → retained earnings; closing entry; opening entry for next year) per Iranian practice" (§5.3, line 397; M1 checklist, line 576).

### Prototype findings

A prototype test (`crates/sarv-quote/tests/year_end_prototype.rs`) using `seed_fixture`, `journal_post`, `banking_post`, `period_lock`, and `period_add` established:

1. **Year 1 before close**: `revenue = 700000`, `expenses = 150000`, `net_profit = 550000`.
2. **Year 1 after manual closing entry + lock**: `report(y1_start..=y1_end)` returns `revenue = 0`, `expenses = 0`, `profit = 0` because `report` does not exclude the closing journal (**GAP**).
3. **Year 2 (`2027-03-21..=2028-03-19`)**: after posting `300000` revenue and `80000` expense in Year 2, `report(y2_start..=y2_end)` returns Year-2-only P&L (`revenue = 300000`, `expenses = 80000`, `profit = 220000`).
4. **Year 2 balance sheet & trial balance**: `total_assets = 771251`, `total_liabilities = 299`, `total_equity = 770952` (`balanced = true`), with `RetainedEarnings = 550000` (Year 1 profit) and `current_earnings = 220000` (Year 2 profit). Trial balance `total_debit = 851251 == total_credit`.
5. **Permanent accounts & subledgers**: receivables (2 open items), payables (2 open items), inventory (`stock_rows = 2`, `average_stock_cost = "10"`), and cheques (`0` in fixture) carried forward across the year boundary without a Year-2 opening entry, and `Store::open` + `validate_full()` succeeded.
6. **Control P&L accounts gap**: `CoreRole::Cogs` and `CoreRole::InventoryLoss` have `control = true`, so `journal_post` cannot zero them; an engine-level closing capability (`control_allowed = true`) is required.

## Decision

### 1. Mechanism: dedicated closing journal (`JournalKind::Closing`)
Add `JournalKind::Closing { period_id: String }` (and `AccountingState::post_closing` with `control_allowed = true`) to post a single balanced closing journal dated `period.end_date` that zeroes every non-zero `AccountClass::Revenue` and `AccountClass::Expense` account (including control P&L accounts `Cogs` and `InventoryLoss`) directly into `CoreRole::RetainedEarnings`.

**Alternatives considered:**
- *Reserved `source_id` / `event` prefix on `JournalKind::Manual`*: avoids touching `JournalKind`, but cannot close `control = true` P&L accounts (`Cogs`, `InventoryLoss`) without weakening manual-posting guards, and relies on string conventions rather than type-checked invariants in `State::validate`.
- *Intermediate P&L Summary account (`خلاصه سود و زیان`) + two vouchers*: matches traditional two-step Iranian manual bookkeeping, but adds a new `CoreRole` and two journals per close for identical net ledger effect. If a qualified accountant confirms two-step vouchers are required for statutory book printing, a `CoreRole::PnlSummary` intermediate step can be added cleanly behind `post_closing`.
- *Year-2 opening entry (`سند اختتامیه` + `سند افتتاحیه`)*: unnecessary in an append-only continuous ledger and conflicts with `accounting.rs:1132-1138` ("re-transfer of balances not supported").

### 2. Permanent accounts and subledgers
No Year-2 opening journal is posted. Balance-sheet accounts, open receivables/payables, inventory quantities/average cost, and cheques carry forward automatically because `trial_balance`, `balance_sheet`, `sales_reconcile`, `procurement_reconcile`, and `cheques` evaluate state cumulatively up to `as_of`.

### 3. Order of operations and guards
`period_close_preview` and `period_close_post` enforce:
1. All earlier fiscal periods for the company (`end_date < period.start_date`) are already locked/closed.
2. `period.locked == false` and no active (unreversed) `JournalKind::Closing { period_id }` exists for `period_id`.
3. `Store::integrity()` is not `Failed`, and `State::validate()` passes (ADR-0007).
4. **Unposted drafts**: refuse close (or require explicit acknowledgement) if `sales_drafts` contains drafts for the company dated `<= period.end_date`.
5. Post the `JournalKind::Closing` entry dated `period.end_date` (if any temporary account has a non-zero balance through `period.end_date`) and set `period.locked = true` atomically in one `Store::change`.

**Reopening a closed year (`period_reopen`)**: desktop-only (`actor != "local-agent"`, matching `desktop_bridge.rs:1812-1815`). Must atomically unlock the period (so `check_reversal` at `accounting.rs:1271` permits reversal) and post a `JournalKind::Reversal` of the closing journal, provided no later period is already closed.

### 4. Idempotency and audit
- `period_close_post` requires `request_key` and `expected_generation`. Retrying with the same `request_key` returns the existing closing journal ID without mutating books.
- Add `AuditOperation::PeriodClosed` (distinct from `PeriodLocked`) and `AuditOperation::PeriodClosingReversed` so `AccountingState::validate` replays and verifies closing entries separately from simple period locks.

### 5. Statements after close
- **`report` P&L (`desktop_bridge.rs:744-770`)**: exclude `JournalKind::Closing` (and `JournalKind::Reversal` targeting a `Closing` journal) when summing P&L `nets`, so closed years retain their true revenue, expense, and net profit figures.
- **`balance_sheet` (`statements.rs:39-94`)**: unchanged in formula — after close, `RetainedEarnings` reflects closed prior-year profit and `current_earnings` reflects only unclosed-year profit.
- **`trial_balance` (`accounting.rs:1364-1410`)**: add an optional `before_closing: bool` parameter (default `false` = after closing) that excludes `JournalKind::Closing` on `as_of` when `true`, supporting both pre-closing and post-closing trial balances.

### 6. UI flow (design only)
Add a **"Close fiscal year"** wizard on the Accounting page next to **"Lock fiscal period"**:
1. **Checklist**: prior periods closed, integrity check green, warnings for any unposted drafts or unreconciled items in the period.
2. **Preview**: calls `period_close_preview` and displays every temporary account's closing line and the net transfer to `RetainedEarnings`.
3. **Confirm**: calls `period_close_post` with the dialog's `request_key` and `expected_generation`.
4. **Result**: shows the posted closing voucher and offers an optional **"Open next fiscal year"** action (Plan 017 `period_add`) if no subsequent period exists yet.

### 7. Migration (books already spanning two years)
If a company already opened Year 2 via `period_add` without closing Year 1, `period_close_post` on Year 1 computes temporary account balances from journals with `date <= year1.end_date`, posts the closing entry on `year1.end_date`, and locks Year 1. Year 2's P&L (`year2.start_date..=year2.end_date`) is untouched, and Year 2's balance sheet immediately moves Year 1's net profit from `current_earnings` into `RetainedEarnings`.

## Consequences

- **Schema impact**: `JournalKind::Closing` and `AuditOperation::PeriodClosed` are additive enum variants; existing `CURRENT_SCHEMA` (7) files and backups remain valid without a schema bump (following ADR-0005).
- **Invariants (`State::validate`)**: must verify that every `JournalKind::Closing` journal is dated on its period's `end_date`, zeroes all temporary accounts as of that date, and has a matching `PeriodClosed` audit event.
- **Statements & UI**: `report` gains accurate historical P&L for closed years; trial balance gains a pre/post-close toggle; Accounting page gains the close wizard.

## Open questions for the owner / a qualified accountant

1. **P&L Summary account (`حساب خلاصه سود و زیان`)**: Does Iranian statutory practice / tax audit (تحریر دفاتر) require two separate vouchers (temporary accounts → P&L Summary, then P&L Summary → Retained Earnings), or is a single closing voucher directly into Retained Earnings acceptable for solo/SMB books?
2. **Net loss presentation**: Should a net loss post directly to `RetainedEarnings` (debit balance / accumulated deficit `زیان انباشته`) or to a separate contra-equity account?
3. **Unposted drafts at year-end**: Should unposted `sales_drafts` dated in the closing year hard-block the close, or only show a warning in the wizard checklist?
4. **Audit-adjustment period (period 13)**: Needed now, or deferred beyond M1?

## Implementation outline

| Slice | Scope | Rough effort | Required tests |
| --- | --- | --- | --- |
| **1. Engine close & reopen** | `JournalKind::Closing`, `AuditOperation::PeriodClosed`, `post_closing`, `period_close_preview` / `period_close_post`, `period_reopen`, `State::validate` replay checks | M | Idempotency, control P&L accounts (`Cogs`, `InventoryLoss`), prior-period-locked guard, integrity-failed guard, reopen + reversal, tampering detection |
| **2. Statements** | Exclude `Closing` (and its reversals) from `report` P&L; add `before_closing` option to `trial_balance` | S | Closed Year-1 P&L matches pre-close P&L; pre- vs post-close trial balance; multi-year balance sheet (`RetainedEarnings` vs `current_earnings`) |
| **3. Desktop wizard** | Accounting page "Close fiscal year" checklist → preview → confirm → optional `period_add` handoff; reopen action | M | Headless `sarv-desktop` state/dispatch tests + `png_scenes` snapshot if applicable |
| **4. Migration & soak** | Retroactive close of Year 1 when Year 2 already has activity | S | Engine integration test closing Year 1 after Year 2 postings exist, verifying `Store::open` + `validate_full()` |
