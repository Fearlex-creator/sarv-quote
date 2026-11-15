# 0010 — VAT on sales and purchase documents (user-entered rates)

**Status:** proposed · **Date:** 2026-11-15

## Context

Sarv currently has no VAT accounting in the core engine (`crates/sarv-quote`):

- `CoreRole` (`crates/sarv-quote/src/core.rs:98-120`) has 21 roles and no VAT payable or VAT receivable account role.
- `SalesState` (`crates/sarv-quote/src/sales.rs:345-383`) posts `Receivables` / `Sales` (and `Cogs` / `Inventory`) on fulfillment, and `SalesReturns` / `Receivables` (and `Inventory` / `Cogs`) on return, with no tax lines.
- `ProcurementState` (`crates/sarv-quote/src/procurement.rs:1668-1685`) posts `Inventory` / `Grni` on receipt, `Grni` / `Payables` on bill, and `Payables` or `Grni` / `Inventory` on return, with no tax lines.
- `sales_draft_from_quote` (`crates/sarv-quote/src/sales_draft.rs:380`) explicitly refuses any quotation with `tax_rial != 0 || tax_basis_points != 0`.
- `crates/sarv-desktop/src/app.rs:371` forces `"tax_basis_points"` to `"0"` when saving default settings.

Most Iranian businesses above the statutory registration threshold must charge VAT, record input VAT credits, and report output vs. input VAT per period (`docs/ACCOUNTING-SPEC.md` §3.3, §3.5). At the same time, **ADR-0004** (`docs/adr/0004-law-as-data-with-verify-flags.md`) requires that `iran::law` values remain read-only notices behind the `reviewed` gate (`require_reviewed` in `crates/sarv-quote/src/iran/law.rs:35-41`) and **must not** be wired into any calculation. Therefore, **all VAT rates are user-entered** (company setting and/or per-line), and `iran::law` is **not** read by any calculation or journal posting.

## Decision

### 1. Tax model and pricing modes

- **Company-level VAT registration**: add `vat_registered: bool` (default `false`) and `default_vat_basis_points: u32` (user-entered, `0..=10_000`, default `0`) to company settings / `CoreSetup`. When `vat_registered == false`, documents carry `tax_rial = 0` and post identical journals to today.
- **Per-item tax category**: add `tax_category: TaxCategory` (`Taxable` | `ZeroRated` | `Exempt`, default `Taxable`) to `CoreItem` (`core.rs:365`). `ZeroRated` (0% taxable supply, e.g. exports) and `Exempt` (Art. 9 exempt supply) both yield `effective_rate_bp = 0` and `tax_rial = 0`, while preserving the distinction on posted lines for VAT reporting and future Moadian mapping.
- **Per-line override & pricing mode**: each sales/purchase order line carries `tax_category: TaxCategory`, `vat_basis_points: u32` (defaulted from company setting when `Taxable`), and `pricing_mode: PricingMode` (`Exclusive` | `Inclusive`).
  - **B2B invoices** default to `Exclusive` (unit price is net of VAT; VAT is added on top).
  - **Retail / POS sales** may use `Inclusive` (unit price includes VAT; VAT is split out without changing the customer-facing total).
- **Non-deductible input VAT**: purchase bill lines carry `vat_deductible: bool` (default `true` when `vat_registered == true`, `false` when `vat_registered == false` or for non-creditable expenses).

### 2. Integer-rial rounding rule

All tax math lives in the engine in integer rials (`i64`, `0..=MAX_MONEY`, with `i128` checked intermediates), matching `positive_half_up` (`core.rs:504-512`) and `domain::tax_amount` (`domain.rs:335-342`).

- **Per-line rounding**: VAT is computed and rounded to whole rials **per line** (matching Moadian's line-level `vam` field in `ACCOUNTING-SPEC.md` §3.3). Document `net`, `tax`, and `total` are the exact sums of the rounded line values. There is no document-level re-rounding and no remainder allocation across lines.
- **Exclusive formula**:
  - `gross = positive_half_up(unit_price * quantity_thousandths, 1000)`
  - `net = gross - line_discount - document_discount`
  - If `category != Taxable` or `rate_bp == 0`: `tax = 0`, else `tax = positive_half_up(net * rate_bp, 10_000)`
  - `total = net + tax`
- **Inclusive split formula (zero drift)**:
  - `total = positive_half_up(inclusive_unit_price * quantity_thousandths, 1000) - line_discount - document_discount`
  - If `category != Taxable` or `rate_bp == 0`: `tax = 0`, else `tax = positive_half_up(total * rate_bp, 10_000 + rate_bp)`
  - `net = total - tax` (the `<= 1` rial remainder is absorbed by `net`, guaranteeing `net + tax == total` identically).

#### Worked examples (verified in `crates/sarv-quote/tests/vat_prototype.rs`)

1. **Exclusive with thousandths quantity and half-up rounding**:
   - `unit_price = 10_003` rials, `quantity_thousandths = 1_250` (1.250 units), `line_discount = 500`, `document_discount = 100`, `Taxable`, `rate_bp = 1000` (10%).
   - `gross = (10_003 * 1_250 + 500) / 1000 = 12_504` rials (`12_503.75` rounded half-up).
   - `net = 12_504 - 500 - 100 = 11_904` rials.
   - `tax = (11_904 * 1000 + 5_000) / 10_000 = 1_190` rials (`1_190.4` rounded half-up).
   - `total = 11_904 + 1_190 = 13_094` rials.
2. **Inclusive split with non-exact divisor (zero drift)**:
   - `inclusive_unit_price = 100_000` rials, `quantity_thousandths = 1_000` (1.000 unit), discounts `0`, `Taxable`, `rate_bp = 1000` (10%).
   - `total = 100_000` rials.
   - `tax = (100_000 * 1000 + 5_500) / 11_000 = 9_091` rials (`9_090.909...` rounded half-up).
   - `net = 100_000 - 9_091 = 90_909` rials.
   - Check: `net + tax = 90_909 + 9_091 = 100_000` rials (zero drift).

### 3. Accounts and posting matrix

Add two `CoreRole` variants in `crates/sarv-quote/src/core.rs`:

- `VatPayable` (Liability, Credit-normal, default code `"2501"`, `"مالیات بر ارزش افزوده پرداختنی"`)
- `VatReceivable` (Asset, Debit-normal, default code `"1402"`, `"مالیات بر ارزش افزوده دریافتنی"`)

| Event | Engine command | Debit account(s) & amount | Credit account(s) & amount | Notes |
| --- | --- | --- | --- | --- |
| Sale invoice | `sales.fulfill.v1` | `Receivables`: `net + tax`<br>`Cogs`: `cost` | `Sales`: `net`<br>`VatPayable`: `tax`<br>`Inventory`: `cost` | `VatPayable` line omitted if `tax == 0` |
| Sales return | `sales.return.v1` | `SalesReturns`: `net`<br>`VatPayable`: `tax`<br>`Inventory`: `cost` | `Receivables`: `net + tax`<br>`Cogs`: `cost` | Reverses `VatPayable` using frozen rate |
| Purchase receipt | `procurement.receive.v1` | `Inventory`: `net` | `Grni`: `net` | No tax effect at receipt |
| Purchase bill (deductible) | `procurement.bill.v1` | `Grni`: `net`<br>`VatReceivable`: `tax` | `Payables`: `net + tax` | Standard VAT-registered purchase |
| Purchase bill (non-deductible) | `procurement.bill.v1` | `Grni` / `OperatingExpense`: `net + tax` | `Payables`: `net + tax` | No `VatReceivable`; tax stays in cost/expense |
| Purchase return (billed, deductible) | `procurement.return.v1` | `Payables`: `net + tax` | `Inventory`: `net`<br>`VatReceivable`: `tax` | Reduces `VatReceivable` by allocated tax |
| Purchase return (unbilled) | `procurement.return.v1` | `Grni`: `net` | `Inventory`: `net` | No tax effect (not yet billed) |
| Payment / Receipt | `payments.receive.v1` / `payments.make.v1` | `Cash`/`Bank`/`Payables` | `Receivables`/`Cash`/`Bank` | No tax effect; settles gross `net + tax` |
| Customer / Supplier advance | `payments.*` (`advance_amount`) | `Cash`/`Bank` or `SupplierAdvances` | `CustomerAdvances` or `Cash`/`Bank` | No tax effect in this slice; tax posts on invoice/bill (see Open Questions) |
| POS sale | `sales.fulfill.v1` + `payments.receive.v1` | `Receivables` (then `Cash`/`Bank`): `net + tax`, `Cogs`: `cost` | `Sales`: `net`, `VatPayable`: `tax`, `Inventory`: `cost` | Typically `PricingMode::Inclusive` |

### 4. New roles in existing books & schema migration

- **Schema bump**: `CURRENT_SCHEMA` (`crates/sarv-quote/src/domain.rs:8`) `7 -> 8`.
- **Additive migration** in `store.rs` / `core.rs`:
  1. For every existing company, ensure chart accounts for `VatPayable` (`2501`) and `VatReceivable` (`1402`) exist (choosing the next unused code if `2501`/`1402` is taken) and append their `RoleMapping`s to the latest `CoreSetup`.
  2. Historical `SalesRecord`, `ProcurementRecord`, and `OpeningRecord` entries retain their existing `frozen_setup` (for deterministic replay) and `#[serde(default)]` zero-tax fields (`tax_rial = 0`, `vat_basis_points = 0`, `tax_category = Taxable`, `pricing_mode = Exclusive`).
  3. Companies with `vat_registered == false` continue to post zero-tax documents with no `VatPayable` / `VatReceivable` journal lines.

### 5. Returns and partial operations

- Tax parameters (`tax_category`, `vat_basis_points`, `pricing_mode`, `vat_deductible`) are **frozen on the posted line** (`FrozenOrderLine`, `PostedFulfillmentLine`, `PostedBillLine`) at posting time.
- Partial fulfillments, partial bills, and partial returns **never** re-read current company settings; they allocate `tax_rial` from the originating line using the engine's existing cumulative `allocation` helper (`sales.rs:227-241`, `procurement.rs:279-294`). Because `allocation` computes `target(cumulative_qty) - prior_allocated` and snaps to `orig_tax` when `cumulative_qty == orig_qty`, the sum of partial returns equals `orig_tax` exactly.

### 6. Documents, PDFs, and reports

- **Documents & PDF (`export_pdf.rs`)**: invoices and bills display per-line net, VAT rate (%), VAT amount, and line total; document footer shows subtotal (net), total VAT, and grand total; seller and buyer `tax_id` / `national_id` are rendered from `BusinessProfile` and `PartyProfile` (`contact_profile.rs:7-23`).
- **VAT period report**: a dedicated `VatReport` query over `[from, to]` aggregating:
  - Output VAT (`sales.fulfill.v1` minus `sales.return.v1`), broken down by `Taxable` / `ZeroRated` / `Exempt` bases.
  - Deductible Input VAT (`procurement.bill.v1` minus `procurement.return.v1`), plus separate total for non-deductible input VAT.
  - Net VAT payable (`output - deductible_input`, or credit carried forward if negative), with drill-down to source documents and journals.

### 7. Interaction with `iran::law` (ADR-0004)

- `iran::law::standard_vat` (`crates/sarv-quote/src/iran/law.rs:60-62`) is **never** read by any engine calculation, validation, or posting path.
- Only if `iran::law::require_reviewed(&entry).is_ok()` (meaning the owner has explicitly set `reviewed = true` and `verify = false`) may the desktop UI *suggest* the rate as a pre-fill in Settings or draft creation; the user must still review and confirm the saved rate.

### 8. Desktop changes

- **Settings (`crates/sarv-desktop/src/ui/views/settings.rs` & `app.rs:371`)**: remove the forced `"0"` for `tax_basis_points`; add a "VAT registered" (`vat_registered`) toggle and a user-entered "Default VAT rate (%)" input (stored as basis points).
- **Sales & Purchase editors**: show per-line tax category (`Taxable` / `ZeroRated` / `Exempt`), rate (%), and computed VAT column (sourced from engine preview), plus document VAT total.
- **POS view**: allow `PricingMode::Inclusive` so shelf prices split cleanly into net + VAT.
- **Quotation conversion (`sales_draft.rs:380`)**: lift the `q.tax_rial == 0 && q.tax_basis_points == 0` guard once sales drafts carry VAT fields.

### 9. Path to Moadian (outline only)

This design leaves room for a subsequent Moadian ADR without reworking the ledger:
- A separate `tax_invoice` aggregate linked 1:1 to `PostedFulfillment` / `PostedSalesReturn`.
- 22-character Tax ID generated at issue via `iran::tax_id::generate(memory_id, issued, serial)` (`crates/sarv-quote/src/iran/tax_id.rs:77-92`) using a monotonic per-memory serial counter.
- 13-digit Moadian product/service ID (`stuff_id`) on product profiles, and an idempotent submission outbox.

### 10. Implementation slices

| Slice | Scope | Rough effort | Required tests |
| --- | --- | --- | --- |
| S1: Roles & migration | Add `VatPayable`/`VatReceivable` to `CoreRole`, bump `CURRENT_SCHEMA` 7→8, migrate existing charts/setups | M | Schema 7→8 migration round-trip, replay with old `frozen_setup` |
| S2: Engine math & sales | Add VAT fields + `vat_prototype` math to `sales_draft.rs` & `sales.rs`, lift quote guard, post `VatPayable` on fulfill/return | L | Golden cases, partial fulfill/return, trial balance & control reconciliation, replay |
| S3: Procurement VAT | Add VAT fields & `vat_deductible` to `procurement.rs`, post `VatReceivable` (or capitalize/expense) on bill/return | M | Deductible vs non-deductible bills, billed/unbilled returns, reconciliation |
| S4: VAT report & PDF | `VatReport` query + drill-down, update `export_pdf.rs` with VAT columns & party tax IDs | M | Report totals vs ledger `VatPayable`/`VatReceivable` balances |
| S5: Desktop UI | Settings toggle/rate (remove forced `"0"`), line tax columns, POS inclusive toggle | M | `sarv-desktop` headless tests & `png_scenes` |

## Consequences

- **Positive**: VAT-registered businesses can issue compliant commercial invoices/bills, track output vs. input VAT in the general ledger, handle partial returns with zero rounding drift, and prepare for Moadian without touching the core accounting model.
- **Negative / Risks**: `CURRENT_SCHEMA` must bump (7 → 8) with careful backward-compatible replay of pre-VAT records; `sales_draft.rs` and `procurement.rs` gain additional per-line fields.
- **Explicit non-claim**: Nothing in this design may be presented to users as statutory compliance until a qualified Iranian accountant reviews the open questions below.

## Open questions for a qualified Iranian accountant

1. **Advances / prepayments**: Under Iranian VAT Law, does receiving a customer advance (`CustomerAdvances`) or paying a supplier advance (`SupplierAdvances`) trigger VAT at payment time, or only upon invoice/delivery?
2. **Non-deductible input VAT**: For exempt-supply businesses or non-creditable purchases (e.g. certain entertainment/passenger vehicles), should non-deductible input VAT always be capitalized into inventory/asset cost or expensed to a separate operating expense account?
3. **Zero-rated vs. exempt presentation**: Should `ZeroRated` (exports, Art. 10) and `Exempt` (Art. 9) supplies be tracked in separate revenue sub-accounts or is line-level tagging on `Sales` sufficient for the seasonal VAT declaration?
4. **Special / Art. 26 rates**: Are multiple simultaneous VAT rates (e.g. tobacco/fuel levies) needed on a single invoice line, or is a single `rate_bp` per line sufficient?
5. **Document-level discounts**: When a document-level discount is applied across mixed `Taxable` and `Exempt` lines, must it be allocated proportionally to net line amounts before computing VAT (as designed here), or restricted to taxable lines?
6. **Inclusive rounding tolerance**: For retail/POS tax-inclusive receipts, is absorbing the `<= 1` rial rounding remainder into `net` acceptable for Moadian Type 2/3 submissions?
7. **22-char Tax ID check digit**: Confirm the character-to-number mapping in `crates/sarv-quote/src/iran/tax_id.rs:58-68` against the latest official Moadian technical guide before production use.
