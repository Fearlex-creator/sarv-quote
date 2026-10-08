# M6 focused export page inspection

The exporter owner actually opened all ten 90-DPI Poppler PNG pages with `view_image` after the passing focused test run on 2026-10-08. This records visual inspection of the generated PDFs, separately from the exact financial assertions in `tests/purchase_return_exports.rs`. It is not native app, accountant, statutory or physical-print acceptance.

| PDF | Pages inspected | Observed result |
| --- | --- | --- |
| billed-physical-return.pdf | 1 | Physical return heading; linked return/credit identities and separate states; original receipt/bill rows and their paper/financial dates;45IRR/4.5toman; original W1 movement and before/after values readable. |
| billed-supplier-credit.pdf | 1 | Distinct credit heading/source identity; linked physical return identified;45IRR/4.5toman; original bill-line allocation and105IRR bill remainder readable; no physical movement row. |
| unbilled-return-one-rial.pdf | 1 | Unbilled physical return with no credit; shared receipt cursor shows original3/value1 and event-time remaining1/value0;1IRR/0.1toman and original W1 movement readable. |
| zero-supplier-credit.pdf | 1 | Distinct posted credit identity/state, quantity1 and0IRR; metadata explicitly says no GL monetary effect; original zero-value bill line and its dates readable. |
| long-physical-return.pdf | 1–6 | All12original receipt/return rows and12stock movements present; page headings repeat company, return ID, cutoff, display date/calendar/digits, unit, policy and page number. Page4 includes180IRR/18toman document total. No row split, overlapping line or clipping observed on any page. |

Persian shaping is readable, source codes/dates are isolated for mixed-direction text, and the original warehouse, quantities, exact IRR and displayed toman remain visible. The long-source/date rows wrap inside the page margins. Font reports show bundled Vazirmatn embedded and subsetted. PDFs are untagged; accessibility and physical printing remain untested.

`focused-test.log` contains the successful offline/locked3-test result. Two initial diagnostic logs retain the caught fixture-ID collision, test tuple compile issue and headless-font fallback; failed-run documents were removed from this evidence directory before regeneration. `focused-manifest.json` hashes the actual test executable, source captured after this focused run, bundled font/license inputs and every PDF/CSV/PNG artifact. This is a focused source epoch; root owns the final integrated freeze/build/native acceptance and historical M5 attribution.

Second Persian-reader review: the native owner independently viewed all ten exact rendered pages and reported no material clipping/readability defect. The report and exact PNG hashes are recorded in `../native/export-second-reader.json`; this is typography/readability evidence, with synthetic/physical-print/accountant limits retained.
