# 0008 — Engine-owned document numbering and quick counter sale

**Status:** proposed · **Date:** 2026-11-15

## Context

`AGENTS.md` requires that the UI call `sarv_quote::desktop::dispatch` and never invent journal lines, tax, or document IDs. Today, four places violate this boundary:

1. **UI-invented document numbers.** `crates/sarv-desktop/src/ui/views/sales.rs` constructs order references (`ORD-{sales_count+1}-{tail}`) and invoice references (`{invoice_prefix}-{sales_count+1}-{tail}`), plus `RET-…` for returns. `purchases.rs` similarly constructs `PO-…`, `R-…`, `B-…`, and `PRET-…`. These numbers are not sequential, change across retries, and can collide across windows. Only quotations use an engine counter (`settings.quote_prefix` and `settings.next_number` in `desktop_bridge.rs`).
2. **Spec requirement for gap-free numbering.** `docs/ACCOUNTING-SPEC.md` (line 759) specifies: `"Numbers are gap-free per fiscal year and assigned at post time."` Section 3.4 (line 161) likewise requires monotonic serial allocation per fiscal memory for Moadian e-invoicing.
3. **Guessed engine-generated IDs.** `fulfill` in `desktop_bridge.rs` derives `invoice_id` as `"{source_id}-invoice"`, and `sales.rs` mirrors that `format!` to settle the invoice. If the engine convention changes, payment fails after inventory has already been delivered.
4. **Multi-step counter sales and opening CSV.** A counter sale (and POS charge) currently executes six separate engine calls (`draft_preview`, `draft_save`, `draft_post`, `fulfill_preview`/`fulfill_post`, `payment_options`, `payment_preview`/`payment_post`). A failure midway leaves a posted order or invoice while the UI reports an error, and a retry creates a second draft with a new reference. Similarly, `setup.rs` (`opening_csv`) builds `sarv-opening-v1` CSV rows, derived `{p}-GL-{role}` IDs, and the balancing `owner_capital` line in the UI.

## Decision

1. **Number series.**
   - **Document kinds:** The engine issues sequential numbers for **sales invoices**, **sales orders**, **sales returns**, **purchase orders**, **goods receipts**, **purchase returns**, and **payment receipts** (matching quotations, which already use `settings.quote_prefix` / `settings.next_number`). **Supplier bills** do not get an internal series because the bill reference is the supplier's own invoice number.
   - **Scope:** Series are scoped **per company**. Per `docs/ACCOUNTING-SPEC.md` (line 759), official invoices and journals reset **per fiscal year** (`period_id`). Whether internal operational documents (orders, receipts, returns) also reset per fiscal year or run continuously is an open question for the owner.
   - **Format:** `{prefix}-{padded_number}` (or `{prefix}-{fy_code}-{padded_number}` if per-FY), with configurable prefixes in `business_profile` / `settings` (which already holds `invoice_prefix` and `quote_prefix`) and zero-padded counters (e.g. 4+ digits).

2. **Counter storage, atomicity, replay, and refused posts.**
   - Counters live in persisted engine `State` (a map keyed by `(company_id, series_key)` or `(company_id, period_id, series_key)`).
   - Allocation happens inside the **same `Store::change(|state| ...)` closure** as the post itself. Because `Store::change` clones/mutates a candidate `State`, runs `validate_changes`, and commits once in a SQLite `IMMEDIATE` transaction, the counter increment and the document post succeed or fail together.
   - **Refused posts:** If validation or posting fails, `Store::change` discards the candidate state — no number is consumed and no gap is created.
   - **Idempotency and replay:** On retry with the same `request_key` and matching payload, the engine returns the already-issued `reference` and IDs from the prior record without incrementing the counter. On open/replay, `State::validate` verifies that all engine-issued numbers in a series are unique, strictly monotonic without gaps after the series start point, and consistent with the stored counter high-water mark.

3. **Existing books migration.**
   - Existing posted documents already carry UI-generated references (`ORD-…`, `INV-…`, `PO-…`, `RET-…`, `PRET-…`, `R-…`, `B-…`). Those historical references remain immutable on existing records.
   - For each company/series, the engine initializes the counter after `max(existing numeric part)` found in existing records (or `1` if none).
   - **Schema bump:** Following ADR-0005, adding an optional/defaulted counter map on `State` (`#[serde(default, skip_serializing_if = "…")]`) does **not** require bumping `CURRENT_SCHEMA` (currently `7`), provided legacy records prior to series adoption are exempted from gapless checks. If a strict one-time backfill migration is preferred, `CURRENT_SCHEMA` can be bumped `7 -> 8`.

4. **Returning IDs from every `*_post` action.**
   - Every `*_post` bridge action returns the IDs and references it created (`record_id`, `invoice_id`, `journal_id`, issued `reference`, etc.) so the UI never constructs or guesses an ID.

5. **Quick counter sale (`sale_quick_preview` / `sale_quick_post`).**
   - **Inputs:** `{company_id, period_id, customer_id | walk_in: true, lines, note, date, due_date, payment: {method, received}, request_key, expected_generation}`.
   - **Atomicity:** `sale_quick_post` executes order creation/post, fulfillment (delivery + invoice), and optional payment settlement inside **one** `Store::change` closure, returning `{record_id, invoice_id, payment_id, journal_id, reference, _generation}`.
   - **Idempotency:** A single `request_key` governs the whole composite action; on retry, the engine compares the full business payload hash against the stored request and returns the prior result (or rejects a mismatched payload).
   - **Preview:** `sale_quick_preview` is read-only and returns computed totals, tax (`0` today), stock availability / validation errors, and the combined journal lines.

6. **Simple opening balance (`opening_simple_preview` / `opening_simple_post`).**
   - **Inputs:** `{company_id, as_of, cash, bank, request_key}` (plus optional localised labels or derived from the company's configured language).
   - Builds the opening ledger lines (cash, bank, zeroed control accounts, and balancing `owner_capital` credit = `cash + bank`) and derived IDs inside the engine, replacing UI CSV assembly.

7. **Desktop migration.**
   - Once the engine endpoints above land, the UI removes `next_order_ref`, `next_invoice_ref`, `ref_tail` in `sales.rs`, the corresponding `PO-`/`RET-`/`PRET-`/`R-`/`B-` reference builders in `sales.rs` and `purchases.rs`, the `format!("{fulfill_key}-invoice")` fallback in `sales.rs`, and `opening_csv` in `setup.rs`.
   - As an immediate first slice (Part B, no schema change), `fulfill_post` returns `invoice_id`, `sales.rs` consumes it, and `SalesState` / `PosState` persist references and step progress across retries.

## Consequences

- **Single source of truth:** Document numbers, IDs, and composite counter-sale postings are owned and validated by the audited engine core, eliminating UI/engine drift and half-posted sales.
- **No premature schema churn:** Part B delivers immediate retry safety and ID return without touching persisted structs; full number series and `sale_quick_*` / `opening_simple_*` wait for owner sign-off on the open questions below.
- **Migration testing:** Once accepted, follow-up plans will test counter initialization and replay against existing `docs/assets/accounting-m*/` fixtures.

## Open questions for the owner

1. **Per-fiscal-year reset scope:** Should all document series reset per fiscal year, or only official sales invoices (and journals), while orders, goods receipts, and returns run continuously?
2. **Which documents need a visible gap-free series:** Should sales orders, purchase orders, goods receipts, and returns each have their own gap-free series, or only sales invoices and payment receipts?
3. **Number formatting and prefixes:** What prefix and zero-padding format should each series use (e.g. `INV-1405-0001` vs `INV-0001`), and which prefixes should be editable in Business Profile?
4. **POS receipt numbering:** Should POS counter sales share the main sales invoice series or use a separate POS receipt series (with future Moadian Type 3 receipt mapping)?
5. **Opening account localisation:** For `opening_simple_post`, should default account/opening names ("Opening balances", "Main Cash", "Main Bank") be derived from the company's language setting in the engine or passed as parameters from the UI?
6. **Schema versioning preference:** For existing books migration, is additive state without a `CURRENT_SCHEMA` bump (staying at `7`) preferred, or an explicit `7 → 8` migration?
