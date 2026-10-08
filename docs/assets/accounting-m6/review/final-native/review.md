# Independent candidate3 native review

Reviewer task: `m6_final_native_review`. Production source is read-only. No Cargo, GUI process, network action or production data operation was performed. The accounting requirements and execution brief were read fully before review, together with the sealed purchase-return API and independent M6 acceptance contract.

This review is bounded to the continuous native sidebar, supplier-credit export controls, chooser cancellation, asynchronous delivery, focus and accessible names. It does not close full financial, performance or production gates.

## Frozen attribution

- Candidate3 production fingerprint: `fb425a75263719f798b42970eab91b7e22ea1a9550be8f77f5eda67b1754be95`.
- Native executable: `/workspace/sarv-evidence/bin/sarv-M6-candidate3`, SHA256 `9856176d68d99555249ee969b13e7dbced8329dc2441db226a1fc0385b3254a0`; personally rehashed.
- Reviewed `src/ui.rs`: `9da88d41f4759fd9bb7b589f9961caf32e3455016d5e3930ee994e7e1a8610e9`.
- Reviewed `src/style.css`: `717e18a97b7b8b34f0931684ef8ed8d12e0c9376925adfca83a542dc0cd199b7`.
- Reviewed `src/ui/procurement.rs`: `db7724bec34021eb5db7ea4129a7b09c28d27a623a5858b5aac19b876a8fe4f0`.

All three reviewed source hashes match the frozen candidate3 qualification manifest. Its 236 ordinary tests and five opt-ins are qualification evidence; this reviewer did not rerun them and does not treat their counts as native workflow acceptance.

## Sidebar: no material defect identified

Actual native run: `artifacts/sidebar-native/run-rkxgps05`, retained under `docs/assets/accounting-m6/native/sidebar-candidate3`. Independently rehashed all 16 portable artifacts and matched every manifest entry. Observer SHA256 `ade0f5fe2379a31d03c9cb6d8302585343e1d6ea6bda7d62752d40dc5eb18193` matches current and frozen observer attribution. Read the raw 26-check receipt and keyboard events; all 26 checks pass.

Personally inspected eight PNGs using `view_image`: initial, standard light/dark, narrow sidebar light/dark focus, narrow light history focus, narrow dark content and restart. Also inspected the retained original M5 standard light/dark comparison images; these are historical M5 evidence, not a fresh M5 execution.

In wide candidate3 captures the right native panel reaches the top and bottom, the compact brand occupies its own native header, and the utility section stays anchored below the scrollable destinations. The former outer empty strip visible in the retained M5 screenshots is absent. Current-page and keyboard-focus treatments are visible. Narrow mode opens the navigation page with current-row focus; the lower destinations remain within the scrollable list above the footer. The observed keyboard receipt covers all Alt destinations, arrow/Enter routing, narrow Escape returning focus to the toggle, settings appearance persistence, restart and unchanged complete saved payload.

Source confirms each NavigationSplitView page owns its ToolbarView/HeaderBar, the content status belongs to the content toolbar, and GTK owns scrolling/focus/native surfaces. No material sidebar repair is requested on the inspected combinations.

## Export and accessible-name source review

The posted-return detail creates separate physical-document and supplier-credit CSV/PDF buttons. Supplier-credit buttons capture the exact persisted `credit_id`; chooser title/default filename distinguish credit from physical return. The immutable accounting/settings snapshot and export worker preserve source attribution. PDF/CSV generation runs through `run_background`, which gates concurrent workers, catches worker panic and reports completion/error on GTK's main thread.

Chooser cancellation returns before a destination or worker is used. Fresh candidate3 execution below verifies chooser cancellation and physical-document CSV/PDF save without State mutation. Generation cancellation after Save is not exposed; a preparation status and completion/error exist, but this does not prove full PERF-02 progress/cancellation acceptance.

The prior LabelledBy defect is fixed in the reviewed frozen source: persistent quantity-caption labels receive the same unique accessible name as their Entry whenever the row slot is rebound. Slot rebinding preserves draft input and clears/restores only surviving mapped focus. Distinct supplier-credit CSV/PDF captions provide distinct native button names. Source inspection cannot substitute for actual AT-SPI/keyboard observations or screen-reader speech.

## Acceptance limits

Fresh candidate3 purchase-return run `artifacts/purchase-returns-native/run-i41jh8ii` failed on its 29th check; 28 earlier checks passed. The failure is a genuine report identity defect described below, so full candidate3 native/export acceptance is withheld. Wayland, high contrast, enlarged text, screen-reader speech, physical printing, representative Persian-reader/accountant task acceptance and complete performance budgets remain unaccepted by this review. Native screenshot/test passes do not establish full PUR/INV, G3/G4, accountant/statutory or production readiness. Existing headless financial/data and focused exporter epochs retain their separate attribution.

## C3-R01 — P2: Persian digit preference rewrites frozen identifiers/references

The actual candidate3 physical-return PDF changes body credit reference `SC-M6-NATIVE-1` to `SC-M۶-NATIVE-۱` and changes ASCII numeric characters inside frozen journal, return, credit, receipt and bill-line UUIDs. The primary header source ID and separately appended primary reference remain ASCII, creating inconsistent representations of the same identity in one report. Personally inspected the raw extracted PDF text and a fresh Poppler raster (`candidate3-failed-physical-return.png`); the transformations are present visually, not just in extraction.

`src/procurement_export.rs:309` (`human_line`) calls `prefs.digits.format(&row.label)` at lines319/321 after immutable code/ID/reference spans have already been inserted with `ltr`. Unicode directional isolation prevents ordering problems but does not stop digit substitution. This violates IR-01's tested mixed-script codes and copy/paste values, IR-02's distinguishable exact codes, and the source-traceability purpose of RPT-02/DATA-02. The observer's exact ASCII identity assertion should remain. This is a report fidelity defect, not a financial posting or durable-data corruption claim.

Requested bounded repair: preserve original code/ID/reference characters in v2 purchase-return/supplier-credit human labels while still applying digit preferences to genuine quantities, amounts, dates and sequence numbers. Preserve historical v1 report rendering. Labels also isolate numeric/date spans, so skipping all isolated text indiscriminately would lose their requested formatting; distinguish identity from numeric/date content. Add meaningful mixed-code/digit coverage and rerun the actual frozen native export sequence.

The failed receipt independently records unique quantity-field AT-SPI names, retained overreturn input, cancellation without posting, Jalali canonical-date preservation, exact original-cost/toman preview, one native45IRR return (Inventory377/AP284/TB1585), unchanged other pools/prefixes, distinct return/credit state/identity, separate credit buttons, credit chooser cancellation without complete State mutation and physical CSV source/snapshot identity. Those passed branches remain useful evidence; supplier-credit saves and later second-return/restart branches were not reached in this run.
