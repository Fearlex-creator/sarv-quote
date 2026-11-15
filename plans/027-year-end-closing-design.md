# Plan 027 (design spike): Year-end closing — move the year's profit into equity and carry balances forward

> **Executor instructions**: This is a **design and prototype** plan, not a build-everything plan. The deliverables are a design document and a throwaway-quality prototype test that proves the approach on the engine. Do not ship UI. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-quote/src/accounting.rs crates/sarv-quote/src/statements.rs crates/sarv-quote/src/core.rs`

## Status

- **Priority**: P2
- **Effort**: M (spike); the build that follows is L
- **Risk**: LOW for the spike; HIGH for the eventual build (touches period invariants and statements)
- **Depends on**: 017 (fiscal years) must be DONE
- **Category**: direction (design)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Once a business can open a second fiscal year (plan 017), its income and expense accounts keep accumulating across years unless the year is closed. Today the engine has no closing concept: the balance sheet computes "earnings not yet closed into equity by a year-end entry" (comment at `crates/sarv-quote/src/statements.rs:28`), a `RetainedEarnings` role exists but nothing posts to it, and an opening can be posted only once per company. Without closing, the second year's profit-and-loss statement is only correct because reports filter by date — and equity never shows retained earnings. Accountants expect an explicit, reviewable closing entry and a locked prior year.

## What exists to build on

- Roles: `CoreRole` in `crates/sarv-quote/src/core.rs:99-121` includes `RetainedEarnings` (line ~114). Read the whole enum and how roles map to accounts in a company's setup (`setups[].command.mappings`).
- Period lock: `set_period_lock(period_id, locked, actor, reason)` (`accounting.rs` ~1040-1060); the `period_lock` action runs full `s.validate()` when closing (`crates/sarv-quote/src/desktop_bridge.rs:1690-1702`) and refuses to close when the integrity check has failed.
- Journals: `journal_preview` / `journal_post` take arbitrary balanced `lines` and a `date` inside the period (`accounting.rs:1348-1351`); reversal via `journal_reverse`.
- Statements: `crates/sarv-quote/src/statements.rs` — `balance_sheet` (line ~39) and `aging` (~135); `trial_balance(company, as_of)` in `accounting.rs`. P&L rows come from `fn report` in `desktop_bridge.rs` (~860-1000) using `from`/`to`.
- Opening restriction: `accounting.rs:1380-1385` refuses a second opening ("re-transfer of balances not supported").
- Decided context: ADR-0002 (build on the existing engine), ADR-0007 (full validation at safety points such as period close). `docs/ACCOUNTING-SPEC.md` describes the intended close: search it for "year-end", "closing", "بستن حساب" and read those sections; quote the requirements you rely on.
- Out of scope by owner decision: payroll, team server, multi-currency (roadmap item 4).

## Questions the design must answer

1. **Mechanism**: one closing journal dated the last day of the year that zeroes every income and expense account against `RetainedEarnings` (or against an intermediate "income summary" account, per Iranian practice — "حساب خلاصه سود و زیان" then "سود (زیان) انباشته"). Which does the spec ask for? Does the entry need a distinct journal *kind* so reports can exclude it from the P&L of the year it closes (otherwise the year's P&L would read zero)? How is "exclude closing entries" expressed without a schema change — a reserved `source_id` prefix, or a new field (schema bump)?
2. **Permanent accounts**: balance-sheet accounts carry forward by date already (no opening entry needed for year 2) — verify with a test, including subledgers (receivables/payables open items, stock quantities and average cost, cheques).
3. **Order of operations and guards**: closing requires: all prior periods closed; integrity check passed; no unposted drafts dated in the year? (decide); then lock. Reopening a closed year: must the closing journal be reversed first? Who may do it (desktop only — see plan 009)?
4. **Idempotency and audit**: `request_key` for the close; an audit event `PeriodClosed` distinct from `PeriodLocked`? Replay rules.
5. **Statements after close**: balance sheet equity section shows retained earnings + current-year earnings; P&L per year unaffected; trial balance "before closing" vs "after closing" views.
6. **UI flow** (describe only): a "Close fiscal year" wizard — checklist (unreconciled items, unlocked earlier periods, integrity), preview of the closing journal with totals, confirm, result. Where it lives (Accounting page next to "Lock fiscal period"), and how it relates to plan 017's "New fiscal year" (close can offer to open the next year).
7. **Migration**: books that already span two years without a close (possible after plan 017) — closing the first year later must still work.

## Steps

### Step 1: Read and map

Read the files listed above. Write down, with `file:line` references, exactly how P&L and balance-sheet figures are computed today and which would change if a closing journal existed.

### Step 2: Prototype as an engine test (no production code changes)

Create `crates/sarv-quote/tests/year_end_prototype.rs` (helpers from `tests/desktop_bridge.rs:1-19`). Using only existing actions (and `add_period` directly, or `period_add` from plan 017):

1. Seed the fixture; post some income and expense journals in year 1.
2. Compute year-1 net profit from `report`.
3. Post a closing journal by hand through `journal_post`: debit each income account / credit each expense account for its balance, the difference to the `RetainedEarnings` account, dated the last day of year 1.
4. Lock year 1. Open year 2. Post activity in year 2.
5. Assert and **record what actually happens**: year-1 P&L after the closing entry (expected problem: it now reads zero); year-2 P&L (should contain only year-2 activity); balance sheet as of the end of year 2 (retained earnings = year-1 profit; "unclosed earnings" = year-2 profit only); trial balance still balances; store reopens and validates.

Mark assertions that document a known gap with a comment `// GAP: …` rather than making the test fail; the test is evidence for the design.

**Verify**: `cargo test -p sarv-quote --test year_end_prototype -- --nocapture` runs and prints the observed figures.

### Step 3: Write the design

Create `docs/adr/0009-year-end-closing.md` (status "proposed"; format like `docs/adr/0005-cheque-lifecycle-and-ledger-integration.md`): Context (with the prototype's findings), Decision (your recommended mechanism, with the alternatives considered and why they lose), Consequences (schema impact, migration, statements, UI), and an **Open questions for the owner / a qualified accountant** section. Include an implementation outline split into shippable slices with rough effort each (engine action `period_close_preview`/`period_close_post`; statements changes; desktop wizard; reopen flow), and the tests each slice needs.

Add the row to `docs/adr/README.md`.

## Scope

**In scope**: `crates/sarv-quote/tests/year_end_prototype.rs` (create), `docs/adr/0009-year-end-closing.md` (create), `docs/adr/README.md`.

**Out of scope**: any change under `crates/*/src`; any UI.

## Git workflow

- Branch `advisor/027-year-end-closing-design`; commits `test(engine): year-end closing prototype`, `docs(adr): propose year-end closing`.

## Done criteria

- [ ] `cargo test -p sarv-quote --test year_end_prototype` passes (with GAP comments where behaviour is not yet right)
- [ ] `docs/adr/0009-year-end-closing.md` exists, status "proposed", answers the seven questions, lists open questions
- [ ] `git diff --stat a4776749..HEAD -- crates/sarv-quote/src crates/sarv-desktop/src` shows no changes from this plan
- [ ] `plans/README.md` updated

## STOP conditions

- Plan 017 is not DONE (there is no second period to test with) — stop; do not prototype around it by editing periods in SQLite.
- The prototype shows balance-sheet accounts or subledgers do **not** carry across years correctly — this is a defect in plan 017's territory; report it with the failing figures.

## Maintenance notes

- Statutory details of the Iranian close (the intermediate summary account, required disclosures) should be confirmed by a qualified accountant before the build; ADR-0004 sets the precedent that unverified statutory behaviour is flagged, not assumed.
