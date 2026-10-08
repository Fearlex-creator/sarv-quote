# Read-only purchase-return and supplier-credit exports

M6 extends `procurement_export::{document_preview,document_csv,document_pdf}` without changing their signatures. These functions accept the operational procurement record ID, the distinct physical `return_id`, or the linked `credit_id`. A credit ID selects **اعتبار تأمین‌کننده** (`supplier_credit`); a record/return ID selects **برگشت فیزیکی خرید** (`purchase_return`). A credit view retains its linked physical return identity/state and original-cost line allocation but omits stock movement rows. An unbilled return has no supplier credit. Missing and foreign-company identities fail.

The report is read-only and states the physical return state (`returned`) separately from the supplier credit state (`posted`). Zero-value billed returns retain both identities and operational quantities; their financial-effect metadata says nonfinancial when there is no journal. Posting and credit do not imply payment, refund, approval or statutory submission.

Each return/credit line exports its frozen item from the original PO, supplier, original warehouse, base unit, receipt record/line and specific bill record/line where billed. It shows original allocation quantity/value, returned quantity/value and the exact quantity/value left immediately after this particular record. Original receipt and bill references and their separate paper/financial dates accompany the return's paper/financial dates. A billed view shows the linked bill's remaining AP after this event. Later returns and appended catalog resources cannot change that historic document remainder.

Unbilled allocations share the original receipt cursor with bill matching. Billed allocations use only the specific frozen bill-line cursor. The report repeats the original-cost constraint: inadequate warehouse quantity/value or orphan value prevents posting and requires a separately reviewed adjustment; there is no invented variance account, current-average substitution or warehouse borrowing. The export never performs a posting or adjustment.

CSV uses UTF-8 BOM, safe spreadsheet text cells, canonical Latin integer IRR/base-thousandth values and a complete JSON source snapshot. Return-bearing reports use `sarv-procurement-report-v2` and add explicit return/credit identities/states, record and source line IDs, original and remaining values, source paper/financial dates, allocation basis and display digit policy. Existing procurement documents/reconciliations without a return retain the v1 field set. `display_unit`, `display_calendar`, and `display_digits` describe presentation; `amount_irr` is always the canonical exact amount. One toman is exactly ten rials; a one-rial amount displays as `0.1` toman. Display preferences do not change recorded values.

PDF uses the existing Cairo/Pango RTL document renderer and bundled OFL Vazirmatn in a private per-export Fontconfig map. Return-bearing reports register the bundled fonts directly, including in headless workers, without global font installation. Historical v1 procurement reports retain their prior rendering path. Page headings repeat company, document identity, financial cutoff, base IRR, display unit, policy and page number. Return documents also state display calendar and digits explicitly. Rows wrap and remain together; an overheight row fails with a CSV alternative rather than silently clipping.

## Verification

Activate the retained local environment before running Rust commands:

```sh
source /workspace/sarv-environment/activate.sh
SARV_PURCHASE_RETURN_EXPORT_ARTIFACT_DIR=/workspace/sarv-quote/docs/assets/accounting-m6/exports cargo test --offline --locked --test purchase_return_exports
```

The focused tests use independent literal expectations:

- M5 March31 Inv422/AP329/GRNI50/TB1630, then billed R-B returns3/45 and4/60 produce Inv377/AP284/TB1585 and Inv317/AP224/TB1525. The original bill150 remainder becomes105 then45; W1 is the original warehouse.
- Receipt3/value1: bill1/value0, unbilled return1/value1, final bill1/value0, followed by a zero-value return of the first bill with a distinct posted credit and no empty journal. The historic unbilled export retains its event-time remaining1/value0 after the later bill.
- Receipt3/value2 unbilled returns reverse literal1,0,1 rials, including the final residual.
- Formula-like paper references are escaped only in CSV text cells; source JSON retains the original reference. Missing/foreign identities reject and export calls leave state unchanged.
- Physical, credit and12-line return PDFs are text-checked on every page, checked for embedded Vazirmatn, and independently rasterized with Poppler. Actual visual inspection is recorded separately in `docs/assets/accounting-m6/exports/inspection.md` after inspecting all generated pages.

Test counts, text extraction and screenshots alone do not establish accounting correctness; the literal control/lineage assertions provide arithmetic evidence. These synthetic local fixtures are not accountant acceptance or statutory/production certification. Physical printing, native screen-reader behavior, other platforms and production workloads remain untested here. Prior M5 exports are historical evidence and are not regenerated or relabeled by these tests.
