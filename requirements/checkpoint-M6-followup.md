# M6 follow-up — 200-line native verification

Authorized by the user's request to complete the remaining 200-line UI and performance work and push the follow-up. Baseline: merged `main` at `e0675273602c36059efa862d47a20cf705e9d3eb`; C8 production digest `11b48de2e85b9465a22f4a618e4675801b1db08a5a2c35b35cb892e8ea381cf3`. The earlier incomplete observer epoch and all strict timing misses remain immutable historical evidence.

Current C9 [build qualification](../docs/assets/accounting-m6/checks/candidate9/manifest.json) passes 237 ordinary Rust tests, zero failures, five ignored opt-ins each run separately and passing, formatting, strict all-target Clippy and release. All 36 production files, 35 Rust test files and 19 static compile inputs stayed exact before/after. Production digest `87ece16d15640d60ee0bdba3c38c8158e12a50221b3e65d8ec088bd7a4766e7a`; immutable binary digest `da427a634b9b31a58ff81da2cca67043f9c8a9a7ddddb378f4c21e084e8a1903`. Only opt-in editor instrumentation differs from C8; accounting, export, Store and schema remain unchanged.

[Independent editor-trace review](../docs/assets/accounting-m6/review/observer-native-candidate8/candidate9-editor-trace-review.md) passes 17 static checks. The synthetic-only native 16 ms trace starts after ReturnRows construction, before remaining review binding, presentation and first layout, and ends at the first Preview collection request. It includes that remaining setup, editing/paging and its terminal gap. It excludes earlier row construction, later invalid-field reveal and later preview/post. Positive ticks, exact finite gaps and observed maximum are still required from an actual run; static review is not responsiveness acceptance.

Two bounded [observer diagnostics](../docs/assets/accounting-m6/performance/observer-sig11-diagnosis/) completed without reproducing the original SIGSEGV: fresh 20-line opening and the prior two source-picker lifecycles followed by that opening. Owned processes closed and all C8 guards stayed exact. The original crash cause remains unknown. The subsequent executor transport disconnection recovered without resetting the workspace; C9 source, tests, compile inputs and binary were rehashed intact before continuing.

## Bounded acceptance

| Requirement scope | Required evidence | Current status |
| --- | --- | --- |
| UX-08/09/13: long editor | Enter all 200 original source quantities using real keys; at most 25 attached entries across eight pages; preserve exact IDs, order and raw values through Next/Previous and validation | Completed for this bounded workflow; independent functional review PASS |
| UX-09/14: error visibility | Invalid final quantity 2, hidden on page 1, must reveal its actual row with raw input retained; whole focused field/text visibly readable after layout; immediate user Tab must retain its new focus | Completed for this bounded workflow; independent functional review PASS |
| UX-11: review | Reach all 400 source/movement detail strings across sixteen pages, with exact original IDs,1 piece / 15 IRR and every pool transition | Completed for this bounded workflow; independent functional review PASS |
| ACC-02/03, INV-05/07: posting/restart | One atomic full200 return of 3,000 IRR: Dr AP 3,000 / Cr Inventory 3,000, distinct linked return/credit, all 200 stock movements, no altered original records, complete stopped State equal after restart | Completed for this bounded workflow; independent functional review PASS |
| PERF-01/02, ARCH-04 | Declared hardware/dataset/method; final n=10 startup cohorts and n=20 source searches, raw arrays and nearest-rank summaries; separate memory and event-loop evidence; unchanged budgets and retained misses | Partial: final n10/n20 complete; strict measured gate FAILED,14 misses |
| QA-03, EXEC-05 | Independent observer/provenance review, actual native image inspection, separate raw financial/restart review, exact commands/results and honest limits | Completed bounded financial/visual/provenance review; strict performance failure retained |

Expected full200 fixture before/after: Inventory 3,422→422, AP 3,329→329, GRNI 50 unchanged, AR 299 unchanged, trial-balance side 4,630→1,630 IRR. W1 item A 225 pieces / 3,292 IRR → 25 pieces / 292 IRR; W2 remains 12 / 130. Sales 199, sales returns 100, COGS 58 and earnings 41 remain unchanged. All fixtures are synthetic.

No schema, accounting policy, legal/tax rule, license, deployment or paid service change is proposed. Treasury and other roadmap modules remain outside this follow-up. The acceptance table was defined before observation. Current bounded functional acceptance passes; the wider performance gate remains open.

## Synthetic demonstration

1. Use a disposable copy of `return200-billed` from the declared synthetic dataset. In **خریدها**, search for `M6-PERF-return200-billed-BILL` and open its original-cost return. Enter the document and supplier-credit references, dates and explicit rial display.
2. Enter `1` for each of the 200 original source rows across eight pages. Change row 200 to `2`, return to page 1, then request Preview. Verify that row 200 appears, retains `2` and focus, and shows the quantity error above the fixed footer. Resize to 620×800 and check the complete field remains readable.
3. From page 1, activate Preview with Space followed immediately by Tab. Verify that the user's new focus remains selected after the deferred reveal. Correct row 200 to `1`.
4. Preview all sixteen pages containing 400 original-source and stock-movement details. Each quantity is one piece at its original 15 IRR cost; the journal is Dr AP 3,000 / Cr Inventory 3,000. Post once, inspect the distinct linked physical-return and supplier-credit records, then close and reopen the profile.
5. Compare the stopped before, after and reopened States against the literal totals above and the original record prefixes. Use the independent raw checker; screenshots establish layout, not accounting correctness.

## Retained follow-up diagnostics

The first C9/runtime1 pilot exited139 in AT-SPI state inspection before entering a document. Its real faulthandler stack is preserved. The missing-child guard is fail-closed: a missing current accessibility node is rejected before any native C call. The historical crashed argument remains unknown.

C9/runtime2 entered all 200 quantities but stopped at its first passive validation observation. The targeted closed diagnostic identified one newly visible validation label prepended to the same panel; all sixteen original siblings, qualified roles and parent links remained exact. Root and independent reviewer inspected settled standard and narrow native images: the whole invalid row-200 field, retained `2`, focus and error are readable above the footer. This diagnostic entered only one quantity and did not post; it is not full200 financial acceptance. Runtime2's initial editor gap of74.873073ms remains a strict50ms miss; later gaps were below50ms. Its complete twenty-line postings/restarts and all200 inputs are retained under that original execution scope.

## Current completed workflow and measurement

The [independent functional review](../docs/assets/accounting-m6/review/observer-native-candidate8/candidate9-runtime3-pilot1-functional-review.json) passes all200 ordered keyboard inputs, all400 exact preview strings, whole-State restart and actual standard/narrow/focus images. The separate [raw financial review](../docs/assets/accounting-m6/review/observer-native-candidate8/candidate9-runtime3-pilot1-200-raw-review.json) passes872 assertions without importing production calculators or GUI code. The [final quiet run](../docs/assets/accounting-m6/performance/candidate9-runtime3-final1/README.md) repeats the complete workflow and closes with all76 known newly owned workers inactive, stable source/binary/runtime/fixture guards and zero GTK/GLib warnings.

| Final measured scope | Observed | Budget / result |
| --- | --- | --- |
| Six startup cohorts, initial n1 each | Worst1,849.169ms | 3,000ms / PASS |
| Six cohorts, repeated n10 each | Largest cohort p951,830.120ms | 3,000ms / PASS |
| Initial steady RSS, n11 per cohort | Largest cohort p95130,392,064bytes | 250,000,000bytes / PASS |
| Billed / unbilled source queries, n20 each | Visible p95408.375 /459.068ms; focused732.719 /810.288ms | 200ms / MISS |
| Filtered source paging, n2 descriptive each | Visible347.639 /366.370ms; focused691.922 /759.669ms | 150ms / MISS |
| 200-line Next / Previous, n7 each | Visible p95932.926 /869.512ms; focused1,591.976 /1,529.049ms | 150ms / MISS |
| Native editor maximum gaps: billed20 / unbilled20 / billed200 | 58.204589 /47.600661 /96.359074ms | 50ms / MISS / PASS / MISS |
| All three native previews and postings | All six maxima≤31.306ms, positive ticks and terminal gaps | 50ms / PASS |

Total:39 measured checks,25 PASS,14 MISS, zero missing declared bounded scopes. Nearest-rank summaries retain every sample; n10 and n20 p99 are the respective worst samples. Hardware: shared Debian13.6 KVM executor, EPYC9V74, five visible vCPUs with four-CPU cgroup quota,16GiB, GTK4.22.5/libadwaita1.9.4, private X11/Cairo display1450×1050 at scale1, natural client1400×940. Later descriptive RSS/HWM observations reach166,084,608bytes in a source-picker workflow; they are distinct from the initial-RSS gate and do not establish continuous peak memory. These are fresh-process/uncontrolled-OS-cache starts, not measured OS-cache-cold versus OS-cache-warm benchmarks. Search/paging endpoints include real keys, debounce, rendering and complete current AT-SPI membership/readiness/focus observation. No observation overhead is subtracted or timing limit waived.

The200 editor's96.359074ms maximum occurred at gap1166; its initial84.666043ms and later52.356328ms gaps also exceed50ms. Broader continuous idle/read-only frame behavior, production reference-load tests and attribution of individual gaps to application work versus shared scheduling remain unverified. The overall performance gate stays FAILED; this follow-up closes the bounded native functionality and evidence gap, not all PERF/G4 requirements.

## Native images, samples and migration attribution

Before: [historical C6 clipped last field](../docs/assets/accounting-m6/performance/visual-invalid-field-candidate6/README.md). After: [complete200 standard field](../docs/assets/accounting-m6/performance/candidate9-runtime3-final1/observer/return200-billed-workflow/invalid-last-row-settled-min2s.png), [narrow field](../docs/assets/accounting-m6/performance/candidate9-runtime3-final1/observer/return200-billed-workflow/invalid-last-row-narrow-settled-min2s.png), [user Tab focus](../docs/assets/accounting-m6/performance/candidate9-runtime3-final1/observer/return200-billed-workflow/invalid-user-tab-focus-retained.png), [last preview page](../docs/assets/accounting-m6/performance/candidate9-runtime3-final1/observer/return200-billed-workflow/last-preview-page.png), [posted return](../docs/assets/accounting-m6/performance/candidate9-runtime3-final1/observer/return200-billed-workflow/posted.png).

Exports are unchanged: separately identified [actual physical-return PDF](../docs/assets/accounting-m6/native/run-illjtffj/M6-actual-physical-return.pdf) and [actual supplier-credit PDF](../docs/assets/accounting-m6/native/run-illjtffj/M6-actual-supplier-credit.pdf) retain their C8 chooser attribution; they are sample documents, not exports of this200-line run. Schema7, Store, accounting policy and export sources are unchanged. [Independent migration/fault/restore evidence](../docs/assets/accounting-m6/review/diagnostics/current-probe2/manifest.json) remains attributed to the unchanged kernel/Store. C9 qualification separately reran the actual preserved M5 writer-refusal opt-in. The actual200 after/reopened States are current restart evidence, not a new backup/restore or hardware-power-loss claim.

Remaining gaps: the14 measured performance misses and shared-hardware/observer attribution; full five-year reference dataset; Windows/Wayland, assistive technology and physical-printing studies; accountant opening-to-close review; durable purchase drafts, broader inventory and treasury/reporting/closing/permission workflows tracked in the [roadmap](roadmap.md). Actual legal/tax integrations remain deferred. No universal competitor parity or production-readiness claim is made.

## Separate dark appearance check

The [closed dark observation](../docs/assets/accounting-m6/performance/candidate9-dark200-observation/closed/README.md) uses the same held C9 binary with a synthetic copy and `ADW_DEBUG_COLOR_SCHEME=prefer-dark` on its owned test process. Six passive captures at minimum0.5/1/2seconds cover standard1400×940 and narrow620×800. Root inspected the actual two-second images: the complete row200 field, retained `2`, focus and error are readable above the fixed footer. The original whole State remains unchanged and all three known test workers closed with exact guards. This entered only one quantity and did not post; the complete200 financial workflow remains attributed to the pilot and final runs. This scope establishes dark appearance, not another performance measurement or a persistent-setting change.

## Independent final review and delivery scope

The [final independent review](../docs/assets/accounting-m6/review/observer-native-candidate8/candidate9-runtime3-final1-review.json) recomputes all39 gates from actual raw arrays:25 PASS,14 MISS; checks66 startup observations, both20-query/two-pager cohorts and all native phase records including terminal gaps. It separately verifies all200 inputs,400 preview details,872 raw accounting assertions, exact current88 runtime files,189 portable evidence files,11 stopped read-only SQLite States and all76 known inactive workers. Root and reviewer personally inspect the five actual full-run images. This closes the current200 functional/evidence gap while retaining FAILED performance acceptance.

The local development checkpoint is `sarv-m6-long-document-local`. Commit/push/PR merge are authorized by the user's latest requests; exact remote integration is verified after publication. All work uses synthetic data and cloud-executor checks. Hosted Actions are not triggered. Previous failed attempts and their original receipts are preserved rather than overwritten.

[Independent dark review](../docs/assets/accounting-m6/review/observer-native-candidate8/candidate9-dark200-review.json) passes the six current passive observations, two personally inspected actual images, exact sidecar/runtime/State guards and known-worker closure. Final and dark reviews have no remaining review blocker; the overall strict performance verdict remains FAILED.
