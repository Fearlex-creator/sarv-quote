# 0005 — Cheque lifecycle: pure state machine now, ledger integration after a design pass

**Status:** partially implemented · **Date:** 2026-11-03

## Implemented
`iran::cheque` is a pure state machine (`open`, `apply`) returning the next state and balanced posting
lines by account *role*, exactly per spec §3.6. Tests cover every direction × state × action
combination (nine legal transitions; all others rejected; all postings balanced) and lifecycle nets.

## Not implemented, and why
Wiring it into the ledger touches the audited core:
1. `payments.rs` only accepts `funds_role ∈ {Cash, Bank}` (line ~384). Receiving a cheque as payment
   needs `ChequesReceivable` / `ChequesPayable` as funds, with matching reversal and reconciliation.
2. Clearing a cheque is a new journal-producing event. Every such module is part of the audit replay in
   `accounting.rs` and the control-account reconciliation (`procurement_reconcile`); an unreviewed
   event type could break integrity checks on existing data.
3. There is no `ChequesInCollection` `CoreRole`. Adding one changes the frozen role mapping of every
   existing company. Spec §3.6 calls the deposit step optional, so the proposed policy is: *in
   collection* shares the `ChequesReceivable` account and **deposit posts no journal**.
4. A bounced or returned received cheque must re-open the customer's invoice items, so it should be
   the existing payment reversal, not a free-standing `Dr AR / Cr Notes` entry (that would break
   subledger = control account).

## Proposed design (needs review before coding)
Append-only `ChequesState { records, requests }` like `PaymentsState`: instruments and status events,
state derived by folding. Only two new journals (clear received, clear issued); everything else reuses
payments. Schema bump 7 → 8 through the recovery-backup path. Use the `diagnosing-bugs`/`tdd` skills:
write the failing integrity test first.
