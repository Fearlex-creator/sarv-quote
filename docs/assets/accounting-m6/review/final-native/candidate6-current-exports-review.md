# Independent current candidate6 export sample review

Reviewer `m6_final_native_review`; no Cargo, GUI process or production edit. Personally inspected every **14 pages across six current PDFs**, including all six long-return pages and all four Persian-identity reconciliation pages. Reconciliation rasters were generated solely under this reviewer's owned directory.

Current sample manifest SHA256 `e3995c96e63ef2f54262918a3c132e6371aab126f08c241b72cb25475e92b343`. All34 artifact hashes independently match. Before/after guards match; current36 production,35 test and19 compile-input hashes independently match, and the immutable candidate6 release binary remains exact. The sealed log records four focused export tests passing; the reviewer did not rerun tests or equate them with financial/PERF gates.

| PDF | Pages personally inspected | SHA256 |
| --- | ---: | --- |
| billed-physical-return.pdf | 1 | 9950c6c7a28f99e17f9157078ac648acac402fdf82355c230b094b1f78d6d697 |
| billed-supplier-credit.pdf | 1 | 88f54cddf46fe423dbb43496ee7592d54e497ce9a56c8a808061b7be7407ba3b |
| long-physical-return.pdf | 6 | 6961d7dca70a0e9d6741c3447f3691e03d867671399ba8f3a7c3517501744dda |
| persian-identities-reconciliation.pdf | 4 | 353f33841cf54ace52e81ac07d84330da7ee948f518676692efd78e9d427533e |
| unbilled-return-one-rial.pdf | 1 | 145611a8c7be5602a4f0fa6f1ebe41cd1a5028501014aa8aa35885d37d34f5f2 |
| zero-supplier-credit.pdf | 1 | 9dab6266bfae5d0b274de146f6791b5658fd442b93900b3f1b0b4956358fc4c5 |

Long-return line0–11, each original receipt allocation and each source stock movement is present, readable and kept together. Repeated company/source/currency/policy/date headers and page numbers remain readable. The return is12×15IRR=180IRR/18toman, with12 exact original-cost stock movements. No clipping, overwritten text or omitted source line was identified. This12-line sample does not establish200-line usability or performance budgets.

One-rial return shows1IRR/0.1toman with quantity1.000, original3.000/1IRR and frozen remainder1.000/0IRR, no supplier credit. The zero-value supplier-credit case has Posted state, no journal, one zero-value physical movement and zero amount; the PDF explicitly distinguishes this from a financial journal/payment. CSV complete document snapshots match metadata snapshots. Source IDs, line IDs, journal/company/receipt/bill UUIDs and SKUs checked in actual extracted PDF text are exact.

Default PDF extraction reorders parentheses of formula-shaped reference `=RETURN-PDF()` into `)(=RETURN-PDF`. Fresh `pdftotext -raw` output preserves literal `=RETURN-PDF()` and corresponding credit/unbilled/zero-reference codes verbatim. Exact full-code checks use the actual default or raw output, without changing any digit, punctuation, whitespace or bidi character. Both extraction methods and hash-pinned raw outputs are retained in the machine receipt. No broad normalization was used to waive identity checks. CSV uses its documented leading-apostrophe formula escape while exact source references remain in complete snapshots.

Reconciliation's new Return source-event IDs RETURN-123/CREDIT-456 and numeric SKU123456 are exact under Persian display digits; amount45/date2026-04-01 localize correctly. Earlier historical inventory/receipt/movement rows still render the numeric SKU in Persian under the documented v1 compatibility boundary. Numeric-only paper refs12345/67890 are covered by the inspected preview regression, while this actual reconciliation PDF exposes mixed IDs/numeric SKU; do not imply it prints every paper-reference field.

`candidate6-current-exports-result.json` SHA256 `dc521f0887a9bf4cd07a462096cd7feda7e996736e178caa51fd13a96b3a60e1` records26 independent structural/hash/value checks, every inspected page, all current sample artifact hashes and all reviewer-owned raster/raw-text hashes. No new material defect found in these samples. PDF tagging/screen-reader use, physical printing, statutory acceptance and full PUR/INV/G4/PERF/production gates remain unaccepted.
