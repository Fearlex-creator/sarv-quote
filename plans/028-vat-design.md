# Plan 028 (design spike): VAT on sales and purchases with a user-entered rate

> **Executor instructions**: This is a **design** plan. The deliverable is a design document with a posting matrix and a prototype test; no production code changes. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-quote/src/core.rs crates/sarv-quote/src/sales.rs crates/sarv-quote/src/sales_draft.rs crates/sarv-quote/src/procurement.rs crates/sarv-quote/src/iran`

## Status

- **Priority**: P2 (it gates use by any VAT-registered business, and all Moadian e-invoicing work)
- **Effort**: M (spike); the build that follows is L–XL
- **Risk**: LOW for the spike; HIGH for the build (posting matrix, returns, replay, schema)
- **Depends on**: none for the spike; the build should follow 018 (chart of accounts) and 026 (numbering)
- **Category**: direction (design)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Most Iranian businesses above the registration threshold must charge value-added tax and show it on every invoice. Sarv cannot: there is no VAT payable or receivable account role, taxed quotations are explicitly refused, the desktop forces the tax setting to zero, and invoices post revenue and receivable only. `docs/SPEC-COVERAGE.md` rates VAT "Partial: rate data in `iran::law`; engine uses a per-quote basis-points setting" and Moadian e-invoicing "Tax ID only". Nothing statutory can be built until an invoice can carry tax.

## Hard constraints (decided — do not design around them)

- **ADR-0004** (`docs/adr/0004-law-as-data-with-verify-flags.md`): law values are effective-dated, flagged `verify`, behind an owner `reviewed` gate, and "not wired into any calculation yet". AGENTS.md: "Law data is a read-only notice. It is not an input to a journal." Therefore the VAT **rate must come from the user** (a company setting and/or per line), never from `iran::law`, until a qualified reviewer flips the gate.
- "The UI … does not invent journal lines, tax, or document ids." All tax math lives in the engine, in integer rials with a documented rounding rule.
- Money is integer rials; toman is display only.
- ADR-0002: build on the existing engine; no rewrite.
- Out of scope by owner decision: payroll, live submission to the tax authority in this design (Moadian is a later design that builds on this one).

## What exists to build on

- Roles: `CoreRole` — `crates/sarv-quote/src/core.rs:99-121`. No VAT role. Read how roles are mapped to accounts at setup (`core.rs` ~580-640) and how existing books would gain a new role (a migration that adds accounts to every company).
- Refusal of taxed quotations: `crates/sarv-quote/src/sales_draft.rs:367-369` ("not yet active"). Settings carry `tax_basis_points` (`settings` arm in `crates/sarv-quote/src/desktop_bridge.rs` ~1746; the desktop writes `"0"` at `crates/sarv-desktop/src/ui/views/settings.rs:239`). The original quotation engine computes a "tax/addition" on quotes: read `crates/sarv-quote/src/domain.rs` for `tax_basis_points` usage and its rounding.
- Sales posting: `crates/sarv-quote/src/sales.rs` — find where the invoice journal lines are built on fulfil (revenue, receivables, COGS, inventory) and on return. Procurement: `crates/sarv-quote/src/procurement.rs` — bill posting (inventory/GRNI, payables) and purchase returns.
- Tax identifier: `crates/sarv-quote/src/iran/tax_id.rs` — `generate(memory_id, issued, serial)` and `validate` (Moadian Tax ID with Verhoeff check), unused so far.
- Law data shape: `crates/sarv-quote/src/iran/law.rs` (rates with effective dates and flags) — for display and for a future reviewed default only.
- Spec: `docs/ACCOUNTING-SPEC.md` §3.3 (VAT, effective-dated tax codes), §3.4 (Moadian), §3.5 (seasonal report). Read them and quote what you rely on; items marked "⚠ verify" in the spec stay unverified.
- Schema/migration mechanics: `CURRENT_SCHEMA` in `domain.rs`, `migrations.md`, and the migration tests (`crates/sarv-quote/tests/*_migration.rs`) — any new persisted field needs a schema step and fixtures.

## Questions the design must answer

1. **Tax model**: company-level default rate (basis points, user-entered) + per-item tax category (taxable / exempt / zero-rated) + per-line override? What is the minimum that produces a correct invoice? Prices entered tax-exclusive or tax-inclusive (Iranian B2B invoices are exclusive; retail/POS often inclusive) — support which, and how is inclusive split without rounding drift?
2. **Rounding**: per line or per document; the exact integer formula; who absorbs the remainder. Must be stated as a rule and tested with hand-calculated goldens.
3. **Accounts and posting matrix** — a table for every event: sale invoice, sales return, purchase bill, purchase return, payment (no tax effect), advance (decide), POS sale. Columns: debit/credit accounts with the new roles (e.g. `VatPayable`, `VatReceivable`), amounts. Include non-deductible input VAT if the spec mentions it.
4. **New roles in existing books**: migration that adds the two accounts to each company's chart and mappings; behaviour for companies that are not VAT-registered (setting off → no tax lines, identical postings to today).
5. **Documents**: what the invoice shows (per-line tax, total tax, buyer/seller tax ids — which already exist as free-text profile fields); PDF changes.
6. **Reports**: a VAT report per period (output tax, input tax, net payable) — the basis of the seasonal declaration; drill-down to documents.
7. **Returns and partial operations**: tax follows the original document's rate, not the current setting. Where is the rate frozen (on the line at posting time)?
8. **Interaction with law data**: when (and only when) an entry is `reviewed`, the UI may *suggest* the rate; the user still confirms. Spell out the gate.
9. **Path to Moadian** (outline only): tax-invoice aggregate, Tax ID generation at issue, outbox — what this design must leave room for.
10. **Open questions for a qualified Iranian accountant** — an explicit list.

## Steps

### Step 1: Read and map

Read the files above. Record, with `file:line`, every place an invoice/bill/return journal is constructed and every persisted struct that would need a tax field.

### Step 2: Golden calculations

In `crates/sarv-quote/tests/vat_prototype.rs`, write **pure** helper functions inside the test file (not in `src/`) implementing your proposed rounding rule, and a table of hand-calculated cases (at least 12: single line, multi-line, quantity fractions in thousandths, inclusive and exclusive pricing, a return of part of a line, zero-rated and exempt lines, large amounts near i64 limits using checked math). The test asserts the helper against the hand-calculated numbers. This pins the arithmetic before any engine code exists.

**Verify**: `cargo test -p sarv-quote --test vat_prototype` → passes.

### Step 3: Write the design

Create `docs/adr/0010-vat-on-documents.md` (status "proposed"; headings Context / Decision / Consequences as in `docs/adr/0005-…md`). Include the posting matrix as a table, the rounding rule with two worked examples, the schema/migration plan, the desktop changes (settings: "VAT registered" switch + rate; line editor column; totals; removal of the forced `"0"`), an implementation outline in shippable slices with rough effort and required tests per slice, and the open-questions list. Add the row to `docs/adr/README.md`.

## Scope

**In scope**: `crates/sarv-quote/tests/vat_prototype.rs` (create), `docs/adr/0010-vat-on-documents.md` (create), `docs/adr/README.md`.

**Out of scope**: any change under `crates/*/src`; Moadian submission; seasonal-report file formats; withholding tax; payroll taxes.

## Git workflow

- Branch `advisor/028-vat-design`; commits `test(engine): VAT rounding goldens`, `docs(adr): propose VAT on documents`.

## Done criteria

- [ ] `cargo test -p sarv-quote --test vat_prototype` passes with ≥12 golden cases
- [ ] `docs/adr/0010-vat-on-documents.md` exists, status "proposed", contains a posting-matrix table and an open-questions list
- [ ] The ADR states explicitly that rates are user-entered and `iran::law` is not read by any calculation (ADR-0004)
- [ ] No changes under `crates/*/src` from this plan
- [ ] `plans/README.md` updated

## STOP conditions

- The spec's VAT section contradicts ADR-0004 or the "user-entered rate" constraint in a way you cannot reconcile — report the contradiction; do not pick a side.
- You find the engine already posts a tax amount somewhere in the accounting path (not just quotations) — report it; the "Current state" premise would be wrong.

## Maintenance notes

- Nothing in this design may be described to users as statutory compliance. The product's standing position (`docs/ROADMAP.md` item 5, ADR-0004) is that a qualified local accountant must review tax behaviour before such a claim.
