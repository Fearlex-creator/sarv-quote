# Procurement reports (M4)

These reports cover the staged procurement scope: resource registration, purchase orders, partial goods receipts, exact-cost receipt-to-bill matching, and internal warehouse transfers. They do not represent statutory invoices or tax reports. Returns, cost variances, cancellation, direct expense/asset purchases, sales, treasury, and closing require later workflows.

`procurement_export::document_csv` and `document_pdf` render a recorded document by its canonical record ID. `document_preview` returns the same human-readable content for the native preview. Each document uses its immutable command, posted result, frozen setup/policy, exact source allocations, and relevant immutable master definitions. A later receipt, bill, or unrelated master registration does not rewrite an older document's economics. The export identifies the financial date, document date, price reference/date and override reason where applicable, source event and line IDs, selected/base quantities, original costs, discount allocations, warehouse movements, and effect reference. A purchase order is nonfinancial; a transfer carries identical original cost between warehouse pools and is always nonfinancial. A zero-value operational effect has no financial journal.

`reconciliation_csv` and `reconciliation_pdf` use the genuine `procurement_reconcile(company_id, as_of)` query at the **end of the selected civil financial day**, including opening balances and recorded events up to that boundary. They show all twelve mapped controls with account code/name, general-ledger amount, detail amount and difference; stock by item/warehouse; original and unmatched receipt quantities/costs; supplier payable balances; same-cutoff trial totals; and eligible source events/movements. No opening means no verified report. A cutoff before opening is rejected. Quantities from different base units are never added into a company quantity total.

## Exact values and presentation

Money is integer IRR (rial). Display in toman is exact, including residual tenths: 1,251 rial is 125.1 toman. Quantity fields use integer thousandths; selected pack quantities and normalized base quantities are separate. Pack ratios and moving-average source ratios remain rational, with no floating-point conversion. The PDF uses Persian Pango/Cairo text with the existing licensed Vazirmatn/Noto Sans Arabic font configuration. It isolates entire codes, dates, identifiers, and ratio expressions for left-to-right rendering, repeats headers on every page, measures rows before drawing, and rejects an individual oversized row instead of clipping it. CSV remains available for oversized human documents.

Company, cutoff, canonical Gregorian date, selected date/digit preference, display unit, base currency, frozen policy version/summary/timezone, basis, and included states are explicit. Gregorian dates outside the bounded Jalali converter remain usable: a selected Jalali preference falls back to the canonical date labelled `Gregorian; Jalali unavailable`, and CSV marks `display_calendar=gregorian_fallback`. This fallback does not change financial dates or widen posting permissions.

## CSV v1

UTF-8 CSV begins with a BOM and uses the fixed `sarv-procurement-report-v1` version/header. Common columns identify company, cutoff, currency/display preferences, policy, included states, report basis, document kind, and source. Typed row kinds distinguish document metadata, order/receipt/bill lines, transfer summary, stock movements, controls, stock pools/totals, remaining receipts/payables, trial totals, and source events.

Canonical IDs, financial/source dates (including distinct original `document_date` and financial `availability_date` for remaining receipts), quantities, and all monetary machine fields (`*_irr`) remain stable ASCII. Friendly labels include Persian names and codes. Untrusted text cells use the existing spreadsheet formula guard. `snapshot_json` retains full structured source data without throwing away original inputs. The final `complete_snapshot` row contains the full frozen document plus linked sources/relevant masters, or the operational reconciliation plus opening and eligible records. This supports reconstruction of the exported accounting/source snapshot; it is not an import command or a substitute for a validated Sarv backup.

Both formats build the complete result in memory and publish through the existing `write_new` mechanism. An existing destination is never overwritten, and a failed validation/render leaves no destination file. Reports perform no accounting mutation.

## Literal synthetic validation

`tests/procurement_exports.rs` starts with the documented M3 opening of 1,251 rial, then applies the independent M4 sequence:

| End of financial day | Inventory IRR | AP IRR | GRNI IRR | Trial debit/credit IRR |
| --- | ---: | ---: | ---: | ---: |
| 2026-03-21 opening | 300 | 149 | 50 | 1,251 |
| 2026-03-22 receive 5 pieces for 75 | 375 | 149 | 125 | 1,326 |
| 2026-03-23 receive 7 pieces for 105 | 480 | 149 | 230 | 1,431 |
| 2026-03-24 bill 2 pieces of first receipt for 30 | 480 | 179 | 200 | 1,431 |
| 2026-03-25 bill remaining receipts for 150 | 480 | 329 | 50 | 1,431 |
| 2026-03-26 transfer 6 pieces for original cost 70 | 480 | 329 | 50 | 1,431 |

After transfer W1 holds 30 pieces/350 rial and W2 holds 12 pieces/130 rial. Opening GRNI of 5 pieces/50 rial remains independently unmatched. Every control difference is zero. Tests also check source-event linkage, hostile spreadsheet references, no-overwrite publication, frozen old documents after new master registration, zero-valued receipt without a journal, unchanged state, backup/restore-identical CSV, rejection before initialization/foreign-company documents, actual PDF text/canonical dates/page headers, and labelled Gregorian fallback.

The evidence generation hook is `SARV_PROCUREMENT_EXPORT_ARTIFACT_DIR=<fresh directory> cargo test --test procurement_exports`. Use a fresh destination because production exports reject overwrite. Checked synthetic examples and rendered pages are retained under `docs/assets/accounting-m4/export/`; they contain synthetic company/source data only.
