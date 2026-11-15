# Plan 026 (design + first slice): The engine issues document numbers and posts a counter sale as one action

> **Executor instructions**: This is a **design-first** plan. Part A produces a short design document for the owner to approve; Part B implements only the first, low-risk slice. Do not go beyond Part B. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-quote/src/desktop_bridge.rs crates/sarv-quote/src/sales.rs crates/sarv-desktop/src/ui/views/sales.rs crates/sarv-desktop/src/ui/views/pos.rs crates/sarv-desktop/src/ui/views/purchases.rs crates/sarv-desktop/src/ui/views/setup.rs`

## Status

- **Priority**: P2
- **Effort**: L (Part A: S; Part B: M)
- **Risk**: MED–HIGH (document identity; multi-step posting)
- **Depends on**: 003, 004, 010; coordinate with 020
- **Category**: bug / direction (design)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Project rule (AGENTS.md): "The UI … does not invent journal lines, tax, or document ids." Three places break it today:

1. **Document numbers are invented by the UI.** Invoice numbers are `{prefix}-{sales_count+1}-{4 random chars}`; orders `ORD-…`, purchase orders `PO-…`, returns `RET-…`/`PRET-…`. They are not sequential, they change if the user retries, and two windows can produce the same count. Iranian invoices are expected to carry gap-free sequential numbers. Only quotations use the engine's counter.
2. **The UI guesses an engine-generated id.** After delivering goods, the sale flow settles the invoice whose id it constructs as `"{fulfill_key}-invoice"`, mirroring a `format!` inside the engine. If the engine's convention changes, the payment step fails *after* stock has been delivered.
3. **A counter sale is six separate engine calls** (preview, save draft, post draft, fulfil, payment options, payment). A failure midway leaves an order or invoice posted while the dialog still shows an error; clicking again creates a second draft with a new reference. POS behaves the same way.
4. (Related) **First-run opening balances**: the UI composes the opening CSV itself, including the balancing owner-capital line, ids derived from the first 8 characters of the company id, and English account names.

## Current state

- UI numbering — `crates/sarv-desktop/src/ui/views/sales.rs:142-149`:
  ```rust
  fn next_order_ref(s: &State, post_key: &str) -> String {
      format!("ORD-{}-{}", s.books.count("sales_count") + 1, ref_tail(post_key))
  }
  fn next_invoice_ref(s: &State, post_key: &str) -> String {
      let prefix = s.books.workspace["business_profile"]["invoice_prefix"].as_str().unwrap_or("INV");
      format!("{prefix}-{}-{}", s.books.count("sales_count") + 1, ref_tail(post_key))
  }
  ```
  Also `purchases.rs` (~145, `PO-…`), `sales.rs:831` (`RET-…`), `purchases.rs` (~525, `PRET-…`), receipts/bills `R-…`/`B-…` (`purchases.rs:186,203`).
- Guessed id — `sales.rs:249`: `let invoice_id = format!("{fulfill_key}-invoice");`; engine side `crates/sarv-quote/src/desktop_bridge.rs:711`: `format!("{}-invoice", string(p, "source_id")?)`.
- The chain — `sales.rs:163-258` (`post_sale`): `draft_preview` → `draft_save` → find the saved draft in the refreshed workspace → `draft_post {id, revision, request_key}` (no `expected_generation`) → `ops::post_operation(.., "fulfill", ..)` → `settle_invoice(..)`. POS: `crates/sarv-desktop/src/ui/views/pos.rs` (~160-190) calls the same.
- Engine counter used for quotations — `desktop_bridge.rs:1492-1497`: `settings.quote_prefix` + `settings.next_number`, incremented inside `store.change`.
- Atomicity primitive: `Store::change(|state| …)` applies a closure to a candidate state and commits it once (`crates/sarv-quote/src/store.rs:257…`). Several kernel calls inside **one** `change` closure commit together or not at all.
- Kernel entry points the chain uses (read them): `save_sales_draft`, the `draft_post` arm, `fulfill_post` arm (calls into `crates/sarv-quote/src/sales.rs`), `post_payment_command` (`payments.rs`).
- Opening — `crates/sarv-desktop/src/ui/views/setup.rs:284-360` (`opening_csv`): builds `sarv-opening-v1` rows in the UI (`{p}-GL-{role}`, `ledger("owner_capital", 0, cash + bank)`, `"Main Cash"`, `"Main Bank"`), then `opening_csv_preview` / `opening_csv_post`. The CSV format is documented in `docs/opening-csv.md`.
- Persisted records are replayed and validated on every open; **adding fields to persisted structs changes the schema** (`CURRENT_SCHEMA` in `crates/sarv-quote/src/domain.rs`, migrations described in `migrations.md`). Avoid schema changes in Part B.
- Related decided context: ADR-0005 (cheques integrate through additive state), ADR-0007 (save cost; full validation at safety points). `docs/SPEC-COVERAGE.md` §4.4: "gap-free per-FY numbering (not verified)".

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Engine tests | `cargo test -p sarv-quote` | all pass |
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Part A — Design document (deliverable: `docs/adr/0008-engine-owned-numbering.md`, status "proposed")

Investigate and write, in the existing ADR format (see `docs/adr/0005-…md` for tone and headings: Context / Decision / Consequences), answers to:

1. **Number series**: which document kinds get a series (sales invoice, sales order, sales return, purchase order, goods receipt, supplier bill is the supplier's number — not ours, purchase return, payment receipt). Per company; per fiscal year or continuous? (Iranian practice is per fiscal year; confirm against `docs/ACCOUNTING-SPEC.md` — search it for "gap-free" / "شماره" — and quote the relevant lines.) Format: prefix + zero-padding; where prefixes are configured (the business profile already has `invoice_prefix`).
2. **Where the counter lives** and how it is allocated atomically with the post (inside the same `Store::change`), how replay/validation proves there are no gaps or duplicates, and what happens to a number when a post is refused (it must not be consumed).
3. **Existing books**: documents already carry UI-made references. Proposal: series start after `max(existing numeric part)`; old references stay as they are. State whether a schema bump is needed.
4. **Returning ids**: every `*_post` action returns the ids of what it created (`record_id`, `invoice_id`, `journal_id`, issued `reference`), so the UI never constructs one.
5. **Quick sale action**: `sale_quick_preview` / `sale_quick_post` taking `{customer_id or walk_in: true, lines, note, date, payment: {method, received}}` and doing draft-post + fulfil + optional settlement in one `change`, returning all ids and one `_generation`. Define idempotency (one `request_key` for the whole action; payload compared on retry), and what the preview returns (totals, tax = 0 today, stock availability errors, journal lines).
6. **Simple opening action**: `opening_simple_preview` / `opening_simple_post {cash, bank}` building the same opening the UI's CSV does today, engine-side, with localised account names passed as parameters or derived from the company language.
7. **Migration of the desktop**: which UI functions disappear (`next_order_ref`, `next_invoice_ref`, `ref_tail`, the `-invoice` construction, `opening_csv` in setup.rs).
8. **Open questions for the owner** — list them explicitly (e.g. per-year reset; whether orders need their own visible series or only invoices; POS receipt numbering).

**Verify**: the file exists, has the three ADR headings and the eight topics; add its row to `docs/adr/README.md`.

**Then stop Part A and report the open questions.** Continue with Part B — it does not depend on the answers.

## Part B — First slice: return ids, stop guessing (no schema change)

### Step B1: `fulfill_post` returns the invoice id

In the `fulfill_post` path, add `"invoice_id"` to the response (the same value the engine computes at `desktop_bridge.rs:711`). Check what the response contains today by printing it in a test. Do the same for `receive_post` (receipt id — the desktop already reads `receipt["record_id"]`) and `bill_post` (bill/open-item id) if they create an open item with a derived id (`grep -n '"-invoice"\|"-bill"\|format!("{}-' crates/sarv-quote/src/desktop_bridge.rs`).

Engine test (`tests/desktop_bridge.rs` or `tests/sales.rs`): after `fulfill_post`, `response["invoice_id"]` is non-empty and appears in `payment_options` for that customer as an open item id.

### Step B2: The desktop uses the returned id

In `sales.rs` `post_sale`, take `invoice_id` from the `post_operation` result (it returns the `fulfill_post` response) instead of `format!("{fulfill_key}-invoice")`. Fallback to the old construction only if the field is missing, with a comment naming this plan.

### Step B3: Make a retried sale resume instead of duplicating

Record progress in `SalesState`: `posted_record_id: Option<String>`, `fulfilled: bool`, `invoice_id: Option<String>`, and compute the order/invoice references **once** when the dialog opens (store them next to `post_key`), not on every attempt. On Save: skip steps already done (if `posted_record_id` is set, do not save/post the draft again; if `fulfilled`, go straight to settlement). Clear the progress in `open_sale`. Apply the same to POS's charge path.

Desktop tests (`main.rs`; pattern `test_owner_walks_profiles_documents_and_books`, `test_pos_charges_the_catalog_and_keeps_the_books`):

1. `test_sale_retry_after_failed_payment_does_not_duplicate_the_order` — force the settle step to fail (e.g. set `received` to an unparseable value via state so `settle_invoice` errors, or select a pay method whose fund account is missing), assert one sale order + one invoice exist; fix the input; Save again; assert still one order and one invoice, and one payment.
2. `test_sale_reference_is_stable_across_attempts`.

**Verify**: `cargo test -p sarv-quote` and `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Scope

**In scope**: `docs/adr/0008-engine-owned-numbering.md` (create), `docs/adr/README.md`; Part B: `crates/sarv-quote/src/desktop_bridge.rs` (response fields only), one engine test file, `crates/sarv-desktop/src/ui/views/{sales,pos}.rs`, `src/main.rs` (tests).

**Out of scope (until the ADR is accepted)**: number series, the quick-sale action, the opening action, any persisted-struct or schema change.

## Git workflow

- Branch `advisor/026-engine-owned-numbering`; commits `docs(adr): propose engine-owned numbering and quick sale`, `feat(engine): post actions return the ids they create`, `fix(desktop): resume a half-posted sale instead of duplicating it`.

## Done criteria

- [ ] `docs/adr/0008-engine-owned-numbering.md` exists with status "proposed" and an "Open questions" list
- [ ] `grep -n 'format!("{fulfill_key}-invoice")' crates/sarv-desktop/src/ui/views/sales.rs` appears only in the documented fallback
- [ ] Both suites pass with 1 engine + 2 desktop tests added
- [ ] `plans/README.md` updated, listing the open questions

## STOP conditions

- Returning `invoice_id` requires changing a persisted struct.
- The retry test shows the engine itself creates a duplicate on a repeated `draft_post` with the same `request_key` (idempotency failure in the kernel) — report; that is an engine bug outside this slice.
- You are tempted to implement the number series "while here". Do not: it needs the owner's answers.

## Maintenance notes

- After the ADR is accepted, write follow-up plans: (1) number series + replay validation, (2) `sale_quick_*`, (3) `opening_simple_*`, each with migration tests against the fixtures under `docs/assets/accounting-m*/`.
- Plan 020 keeps all UI-built references in one helper so they can be swapped for engine-issued numbers in one place.
