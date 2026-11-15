# Plan 029 (design spike): Bank statement reconciliation

> **Executor instructions**: This is a **design and prototype** plan. Deliverables: a design document and a prototype engine test. No production code changes, no UI. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-quote/src/desktop_contacts_banking.rs crates/sarv-quote/src/cheques.rs crates/sarv-desktop/src/ui/views/banking.rs README.md`

## Status

- **Priority**: P3
- **Effort**: M (spike); the build that follows is M (manual reconciliation) + L (statement import)
- **Risk**: LOW for the spike; MED for the build (new persisted state)
- **Depends on**: none for the spike; the build should follow 019 (ledger with running balance)
- **Category**: direction (design)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

`README.md` (the **Banking** paragraph, line 46) tells users: "Each account also offers a statement reconciliation dialog: enter the closing statement balance, see the exact difference, and mark the entries you found on the statement." The Rune desktop has no such dialog, and the engine has no place to remember which entries were matched — the feature existed only as client-side state in the removed React shell. Monthly bank reconciliation is a basic control for any business with a bank account; `docs/SPEC-COVERAGE.md` §5.7 lists "Statement import with bank mappers not started".

Until this is built, the README sentence is wrong; plan 032 removes it. This spike designs the real feature.

## What exists to build on

- `banking` action: implemented in `crates/sarv-quote/src/desktop_contacts_banking.rs` (256 lines; read it all). It returns, per cash/bank account, opening and closing balances and dated movements for a `from`/`to` range. The desktop loads it in `Books::refresh` (`crates/sarv-desktop/src/data.rs:89-92`) and renders it in `crates/sarv-desktop/src/ui/views/banking.rs` (233 lines).
- The precedent for adding a new persisted register without disturbing the ledger: cheques. ADR-0005 (`docs/adr/0005-cheque-lifecycle-and-ledger-integration.md`) describes the additive `ChequeRegister` (events log, pure state machine, replay). Code: `crates/sarv-quote/src/cheques.rs`. Read both — the reconciliation register should follow the same shape: an append-only log, validated on load, with no effect on journals.
- Preview/post import pattern with diagnostics: opening CSV — `crates/sarv-quote/src/core_import.rs` (`opening_csv_preview` / `opening_csv_post`), format doc `docs/opening-csv.md`; generic CSV diagnostics in `crates/sarv-quote/src/imports.rs` (row/column errors).
- Journal identity: each journal has `id`, `date`, `source_id`, lines with `account.id`, `debit`, `credit` (see the `ledger` arm in `crates/sarv-quote/src/desktop_bridge.rs:1809-1821`). Posted journals are immutable; corrections are reversals.
- Schema mechanics: `CURRENT_SCHEMA` in `crates/sarv-quote/src/domain.rs`; `migrations.md`; backup formats are versioned per schema (`crates/sarv-quote/src/store.rs:520-530`).
- Decided constraints: live bank feeds and payment processors are **out of scope** (README: "Use offline/manual imports first"); money is integer rials; the UI invents no journal lines (a missing bank fee is posted through the existing Banking "money out" preview+post, not by the reconciliation feature itself).

## Questions the design must answer

1. **Unit of reconciliation**: a reconciliation "session" per account with a statement end date and closing balance; entries are journal **lines** on that account (a journal may have two lines on the same account — handle it). Identity of a line when lines have no id: `(journal_id, line_index)` — is the index stable? (Journals are immutable, so yes — confirm in the code.)
2. **State**: an additive register, e.g. `reconciliations: Vec<Reconciliation { id, company_id, account_id, statement_date, statement_balance, matched: Vec<LineRef>, status: Open|Completed, events… }>`. Append-only events (started, line matched/unmatched, completed, reopened) vs. a mutable record with revision — pick one, following ADR-0005's reasoning. Validation rules on load: referenced journals exist and belong to the account; a line is matched in at most one completed reconciliation; completed sessions per account are ordered by date.
3. **Arithmetic**: book balance as of the statement date; cleared balance = previous reconciled balance + Σ matched lines; difference = statement balance − cleared balance; completion allowed only at difference 0 (or with an explicit, recorded "unreconciled difference" — decide).
4. **Reversals**: a matched journal that is later reversed — both lines cancel; how are they shown and matched?
5. **Cheques**: deposited-but-uncleared cheques are the classic reconciling items; how the cheque register's states map into the reconciliation view (clearing a cheque posts the bank line — it then becomes matchable).
6. **Actions**: `reconciliation_start`, `reconciliation_set_matched {lines}`, `reconciliation_complete`, `reconciliation_reopen`, `reconciliation` (read). Which require `expected_generation` (all mutating ones) and `request_key`.
7. **Statement import (phase 2)**: a generic CSV with a column-mapping step (date, description, debit, credit / signed amount, balance) → preview with diagnostics → suggested matches (same amount, date within N days) the user accepts one by one; unmatched statement rows offer "record as money in/out" through the existing banking preview. Bank-specific presets are data, not code. Persian/Jalali dates and Persian digits in bank exports must parse (reuse `calendar::parse_date`, `iran::ids::normalize_digits`).
8. **UI (describe only)**: per-account "Reconcile" on the Banking page; a two-pane sheet (book entries with checkboxes; summary with statement balance, cleared balance, difference); completed reconciliations listed with a printable summary.
9. **Backups and schema**: schema bump, backup format version, migration test plan.
10. **Open questions for the owner.**

## Steps

### Step 1: Read and map

Read the files listed. Note with `file:line` how the banking action computes balances, and whether journal lines carry anything usable as a stable identity.

### Step 2: Prototype the arithmetic as a test (no `src/` changes)

Create `crates/sarv-quote/tests/reconciliation_prototype.rs` (helpers from `tests/desktop_bridge.rs:1-19`): seed the fixture; post several banking movements via `banking_post` (read `banking_preview`/`banking_post` usage in existing tests: `grep -rn "banking_post" crates/sarv-quote/tests | head`); fetch the account's movements through the `ledger` or `banking` action; in the **test file**, implement the proposed difference calculation over a chosen "matched" subset and assert the hand-calculated cleared balance and difference for three scenarios (all matched; one outstanding deposit; a reversed entry).

**Verify**: `cargo test -p sarv-quote --test reconciliation_prototype` → passes.

### Step 3: Write the design

Create `docs/adr/0011-bank-reconciliation.md` (status "proposed"; headings as in `docs/adr/0005-…md`), answering the ten questions, with the data model as Rust struct sketches, the validation rules as a list, an implementation outline in two phases (manual reconciliation; statement import) split into shippable slices with rough effort and tests per slice, and the open-questions list. Add the row to `docs/adr/README.md`.

## Scope

**In scope**: `crates/sarv-quote/tests/reconciliation_prototype.rs` (create), `docs/adr/0011-bank-reconciliation.md` (create), `docs/adr/README.md`.

**Out of scope**: any change under `crates/*/src`; any connection to a bank; the README (plan 032 corrects it).

## Git workflow

- Branch `advisor/029-bank-reconciliation-design`; commits `test(engine): reconciliation arithmetic prototype`, `docs(adr): propose bank reconciliation`.

## Done criteria

- [ ] `cargo test -p sarv-quote --test reconciliation_prototype` passes with three scenarios
- [ ] `docs/adr/0011-bank-reconciliation.md` exists, status "proposed", with data-model sketches and open questions
- [ ] No changes under `crates/*/src` from this plan
- [ ] `plans/README.md` updated

## STOP conditions

- Journal lines have no stable identity (e.g. lines are re-ordered on save or replay) — report; the design then needs line ids, which is a kernel change to discuss first.
- The `banking` action's balances disagree with the `ledger` action's for the same account and range — report the discrepancy with numbers before designing on top of either.

## Maintenance notes

- Keep reconciliation state strictly additive and separate from journals, as cheques are: it must be possible to delete every reconciliation and leave the ledger untouched.
