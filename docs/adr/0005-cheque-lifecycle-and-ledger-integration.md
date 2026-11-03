# 0005 — Cheque register wired into the ledger and audit replay

**Status:** implemented (owner approval 2026-11-03) · **Date:** 2026-11-03

## Decision (as built)
`iran::cheque` stays the single source of truth for legal transitions and posting shapes. The register
(`cheques.rs`) is an append-only log on `State` (`ChequeRegister { cheques, events }`) that owns no
money: every ledger effect comes from an existing audited source, and the register links to it.

| Step | Ledger effect |
| --- | --- |
| Receive / issue | a payment whose funds are the cheques notes account (`payments.receive.v1` / `payments.make.v1`), so invoice open items settle exactly as for cash |
| Deposit | status only (spec §3.6: the in-collection step is optional). No `ChequesInCollection` role was added |
| Clear | new `banking.cheque_clear.v1` movement: received = Dr bank / Cr cheques receivable; issued = Dr cheques payable / Cr bank. Lines are built from the state machine |
| Bounce / return / void | reversal of the originating payment, so the invoice or bill re-opens and the subledger stays equal to the control account |
| Spend (endorse) | a supplier payment funded by cheques receivable |

Opening cheques can be registered and then deposited, cleared or spent; they cannot be bounced, returned
or voided because they have no originating payment to reverse.

## Changes to the audited core
- `payments.rs`: funds may be cheques receivable (receive, endorse) or cheques payable (issue).
- `procurement_reconcile`: the two notes roles now reconcile against opening + cheque payments + clearing
  (the payable is credit-normal, so its deltas are flipped).
- `accounting.rs`: one new `post_banking` event with its own shape checks (also enforced on replay).
- `State::validate` re-derives every cheque's status from its events and proves each claimed journal or
  payment exists with the right accounts, sides and amount; a payment relied on by a cheque can only be
  reversed through the register. `Store` rejects any save that rewrites cheque history.
- **No schema bump**: the new field is additive and skipped when empty, like the cashbook and profiles.
  Existing data loads and validates unchanged. (ADR draft proposed 7 → 8; it was unnecessary.)

## Proof
`tests/cheques.rs` (14): every lifecycle path posts balanced entries, keeps the trial balance equal and all
control accounts reconciled, passes the full replay and a serialise/reload round trip unchanged; the
posted clearing entry equals what the state machine prescribes; refused steps change nothing; retries
are idempotent; seven kinds of tampering are detected; the ledger itself rejects malformed clearing
movements. The sign handling and the clearing-direction rule were mutation-checked. A browser test
(`tests/cheques.spec.ts`) receives, deposits and clears a cheque through the Payments screen.

## Not built
Cheque books and leaf ranges, batch operations, the due-date calendar, the Sayad bank-API plugin,
guarantee cheques (spec §5.8 items beyond the lifecycle).
