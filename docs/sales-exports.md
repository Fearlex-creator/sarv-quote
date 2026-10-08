# M5 sales documents and reconciliation exports

M5 provides local, read-only CSV and native Persian PDF exports for stock-goods sales orders, paired delivery/invoices, paired normal-restock returns/credits, and end-of-day receivables/control reconciliation. These reports use the existing `core-policy-v1` accrual/perpetual policy. They do not establish payment, refund, tax submission or statutory certification.

The public functions in `sales_export` follow `procurement_export`: `document_csv`, `document_pdf` and `document_preview` take an `AccountingState`, company ID, document ID, money unit, date preference and digit preference; the file exporters additionally take a destination `Path`. `reconciliation_csv` and `reconciliation_pdf` replace the document ID with a canonical Gregorian cutoff. `DocumentPreview` has title, company, displayed as-of date, policy version, status and human-readable lines.

Document lookup accepts the sales record ID, command source ID and the distinct delivery/invoice/return/credit IDs. The chosen delivery or return identity changes the heading while retaining its paired financial source. A company mismatch, unknown source, unavailable opening or cutoff before the opening fails before writing. Existing destination files are never overwritten. PDF generation and CSV export operate on the supplied snapshot and never post or mutate books; the UI runs them in its existing bounded worker.

## Amounts, dates and states

CSV format `sarv-sales-report-v1` is UTF-8 with BOM. Every row identifies company, canonical financial cutoff, selected display calendar/unit, base currency `IRR`, policy, included states, accounting basis, source event and row identity. Integer IRR amounts and thousandths quantities are authoritative; exact displayed toman values preserve a residual rial as a decimal. Financial values never use floating point. CSV text cells are protected against formula evaluation, including references and price sources.

Document rows preserve frozen customer/item identities, original units and pack ratios, quantity, SKU, original price provenance/date, override reason, gross amount, line/document discount, net amount, and original issue or returned cost. Metadata shows distinct delivery/invoice and return/credit identities/references/states, original source lineage, document date, financial date and invoice due date. These dates remain distinct; the PDF includes Gregorian and Jalali document/due dates. Jalali cutoffs outside the supported conversion range explicitly fall back to Gregorian and label the fallback in CSV.

Orders have no ledger or on-hand effect. A delivery/invoice with zero net and zero cost still retains its operational quantity and states; its financial-effect ID is empty and its included state explicitly says nonfinancial. A posted invoice does not mean money was collected. Normal returns credit the invoice and restore its frozen original cost to the original warehouse; they do not reopen the order.

For example, the synthetic 10-piece order at 20 IRR with a 1 IRR document discount totals 199 IRR. Its first five pieces receive net 100 IRR and discount 1 IRR, so the balanced allocated gross is 101 IRR. The final five pieces receive net/gross 99 IRR and zero remaining discount. The first partial return credits net/gross 40 IRR and restores 23 IRR cost; the final return credits net 60 IRR, gross 61 IRR, remaining discount 1 IRR and restores the final 35 IRR cost. Exported gross/discount allocations follow these persisted allocations rather than recalculating quantity times current price. Each invoice/return visibly labels `ناخالص تخصیص‌یافته از سفارش`, labels the price as the original order price with its original price unit, and explains that original net/discount components are cumulatively rounded separately, with the final exact residual. Allocated gross equals net plus allocated discounts and can differ from original unit price times the partial quantity; the difference can exceed one rial. CSV names the method `cumulative_original_net_and_discounts_half_up_final_residual_gross_sum`.

A divisible fixture makes this visible: 1.5 units at 1 IRR have gross 2, document discount 1 and net 1. Three deliveries of 0.5 units receive net `[0, 1, 0]`, document discount `[0, 1, 0]` and allocated gross `[0, 2, 0]`. The original unit price stays 1 IRR; the partial PDFs state the allocation rather than imply fresh unit-price multiplication.

A quotation link is reported only when the posted order actually has one. Its quote ID, series, revision, number, full source SHA-256 and zero-based source-line mapping are retained. A direct order explicitly has no quotation link. Whole-State validation proves an authentic retained approved quotation and exact conversion; the lower-level accounting export cannot confer quotation approval by itself. The sample linked order is generated through the actual quotation approval and durable draft conversion flow, and its retained approved source is included beside the CSV/PDF.

## Reconciliation and reconstruction

The cutoff includes opening and posted operational events through the end of that financial day. Controls disclose account/role, ledger balance, independent detail and exact difference, including AR, inventory, COGS, Sales and SalesReturns. Stock rows retain warehouse/base unit, quantity and value; unlike units are not added. Receivable rows show frozen original invoice/party/document date/due date/reference, original and remaining IRR, and credit-note lineage. No receipt allocation or aging bucket is invented. Zero outstanding invoices remain traceable.

Sales totals include separately identified ordinary/legacy journal adjustments where applicable. `typed_sales_total`, `typed_sales_returns_total` and `typed_cogs_total` expose only M5 linked operational amounts. `ordinary_journal_adjustment` rows disclose journal ID, date, role and signed amount; they never become fabricated invoices or returns. The sales margin is labeled as sales minus returns and COGS, not complete company P&L.

Source aliases with multiple events reject and require the unique record or paired identity. Canonical record/paired identities take precedence over source aliases.

Each document CSV includes a `complete_snapshot` JSON row containing the exact sales record, its ancestor orders/fulfillments and relevant immutable masters. Its contents do not change when a later return or unrelated registry is added. Reconciliation includes the exact computed controls, opening, cutoff sales and procurement sources, and relevant masters. Full application backup/export remains the complete business-data route; these scoped reports do not replace it.

## Persian PDF and evidence

The native renderer follows the established Cairo/Pango procurement renderer: A4 pages, measured wrapped RTL rows, bidirectional isolation for codes, repeated source/date/unit/policy headings and explicit failure when a row cannot fit. A private per-PDF Fontconfig map registers the bundled Vazirmatn Regular/Bold fonts and keeps their private files alive through shaping/embedding, then removes them. No global font installation or network service is required. Unmodified Vazirmatn v33.003 is redistributable/embeddable under the bundled SIL OFL 1.1; see `assets/fonts/README.md`, `OFL.txt` and `AUTHORS.txt`.

`tests/sales_exports.rs` covers literal M4-to-M5 cutoff controls, original-cost return/residuals, paired identities, formula guards, no-overwrite, byte-identical backup/restore, early/foreign books, explicit Gregorian fallback, approved-quotation conversion provenance, zero-money operational documents, ordinary journal adjustments, the divisible partial allocation counterexample, actual PDF text and embedded Vazirmatn, and repeated headings. Run from the repository:

```sh
source /workspace/sarv-environment/activate.sh
cargo test --test sales_exports
```

Set `SARV_SALES_EXPORT_ARTIFACT_DIR` to a new directory when running the tests to emit synthetic sample CSV/PDFs and source/text/font evidence. Destinations are create-new, so use a fresh directory for reruns. Final samples under `docs/assets/accounting-m5/exports/final/` cover the order, delivery, invoice, return, credit, final credit/invoice, final reconciliation, authentic quotation-linked order and three divisible partial invoices. The full baseline requirement mapping is SAL-01/02/04/05, INV-05/07, RPT-02/03, DATA-02, IR-01/03/04, UX-11 and QA-01; these artifacts prove this bounded M5 portion, not the broader requirements' deferred treasury/service/tax workflows. Physical printing, native reader/accountant approval, accessibility, Windows support and large-report performance remain unaccepted here.
