# M4 bounded native performance evidence

Freeze6 is the current measured candidate, observed on **2026-10-08UTC**. Native procurement and quotation workflows pass. Search, startup, initial RSS and all22 accepted procurement timer phases meet proposed limits in this run. Performance acceptance remains partial: complete quotation global/page/PDF limits and the broad persistence-busy50ms proxy remain missed. No budget or external timing endpoint was changed; each distinct frozen source was measured once. Original3/5 results and reports remain immutable;4 was never measured.

## Exact attribution and executor

- Immutable executable: `/workspace/sarv-evidence/bin/sarv-M4-freeze6`; SHA-256 `9be5f0ba6a7fe6f000b722042d426987e79647578ab9ca803a9869ee1a343d5e`.
- Authoritative production aggregate: `4545e5f018a80a9f403c686ecd19ee2ec66bbc683c6a8d80bb293ba6dd98fc14`. Sorted UTF-8 relative-path bytes, path/NUL/raw bytes/NUL; per-file provenance before/after retained.
- Historical helper digest: `3f7d6a155b1ff97b3ac1334ca1191f0cd61d708064114772d28edbc0c17ce262`. Pathlib component sorting uses the same files with a different order; retained separately from the freeze aggregate.
- Production source, executable and observers stayed unchanged during the repeated suite. The unchanged external M2 asserts binary before/after; ordered production provenance immediately before and after its sequential run also matches.
- Actual hardware: five visible AMD EPYC9V74 KVM vCPUs, four CPU equivalents (`cpu.max=400000 100000`),16GiB cgroup RAM, Debian13.6, overlay filesystem, Rust1.99, GTK4.22.5/libadwaita1.9.4. Private X11/Cairo/Xvfb/Openbox1450×1050, scale1.
- Root declared other Sarv GUI/build/test/render/staging quiet before GO; other owners did cheap text work. M4 repeated then unchanged M2 ran sequentially. No shim or controlled cancellation experiment entered timings. Before/after loadavg and cumulative cgroup CPU stats are retained, not continuous monitoring or isolated host scheduling.

[Portable freeze6 evidence](../docs/assets/accounting-m4/performance/freeze6/README.md) contains raw samples, logs, every phase/gap/result, source/observer hashes, selected native screenshots and standalone SVG plots. Application logs contain zero GTK/Adwaita/GLib warning/critical lines. AT-SPI shutdown stale-cache notices belong to the observer and remain separate. No database, executable or runtime cache is committed.

## Startup and memory

Startup includes process launch, mapped owned window, visible usable Save, real XTest Tab focus roundtrip and completed PNG. AT-SPI/image overhead is included. First on a fresh SQLite backup may include initialization/migration. Repeated runs start new processes on an initialized backup with warm/uncontrolled OS pages. Physical/cache-cold launch was not measured; copying/hashing warms pages and no cache flush was performed. Deferred review/rows may remain pending at this endpoint.

| Fixture | First fresh-backup ms (n1) | Repeated p50 ms | p95/worst ms (n10) | Steady RSS p95 KiB | VmHWM p95 KiB |
|---|---:|---:|---:|---:|---:|
| core-small-posted | 784.976 | 799.655 | 865.763 | 94068 | 99516 |
| quote-scale | 1477.881 | 1041.515 | 1250.606 | 173756 | 173756 |
| procurement-small-posted | 837.930 | 828.253 | 862.787 | 97200 | 101716 |

All repeated startup p95 values meet3,000ms and initial steady RSS meets250,000,000bytes. Application-only `/proc` memory is sampled every10ms and after500ms idle; helpers are excluded. Later core RSS/HWM is130,424KiB; procurement RSS135,920/HWM136,868KiB; full quotation RSS268,672/HWM319,140KiB. These later points are separate and do not establish initial-memory or leak acceptance.

## Real-key searches

Core keeps the historical M3 observer: twenty alternating CU-A/CU-B real Ctrl+A/ASCII queries, exact current one-row ID and visible sensitive Edit, followed by native Tab to focused usable Edit. Discovery/initial focus occurs outside timing; dispatch, debounce, rendering,5ms accessibility polling and result navigation remain inside. Exact payload/generation equality is asserted after every read-only query.

| Stream / endpoint | n | p50 ms | p95 ms | p99/worst ms | 200-ms proposed p95 |
|---|---:|---:|---:|---:|---|
| Core visible Edit | 20 | 83.262 | 93.643 | 93.965 | PASS |
| Core focused Edit | 20 | 114.998 | 124.378 | 129.721 | PASS |
| Procurement visible Detail | 20 | 102.817 | 128.108 | 184.364 | PASS |
| Procurement focused Detail | 20 | 115.157 | 141.436 | 197.064 | PASS |
| Read-only party filter | 20 | 81.040 | 90.003 | 92.996 | Filter only; no selectable-action claim |
| Read-only item filter | 20 | 80.548 | 98.014 | 100.581 | Filter only; no selectable-action claim |
| Read-only mixed_master filter | 20 | 82.510 | 95.139 | 96.917 | Filter only; no selectable-action claim |

Procurement discovery scopes the unique Purchases page; Ctrl+F reveals its showing/visible/sensitive SearchEntry before timing. Hidden Inventory entries cannot satisfy readiness. Queries alternate REC-A/REC-B, CU-A/CU-B, SKU-A/SKU-B and CU-A/SKU-B. Zero-result warmup and exact alternating one-result output reject stale readiness. Six operational documents and four resource events remain distinct while all10 records are searchable. Master rows are read-only; their timings do not certify selectable picker workflows.

Historical core visible/focused p95: M3=202.976/244.176ms,3=93.154/127.684,5=88.487/123.870,6=93.643/124.378. Freeze3 focused procurement missed200 at214.167ms; diagnosis found seven Tabs. Product source changes reduce the filtered-result route to one Tab; identical observer measures5=114.922 and6=141.436ms. Default/backward/stale/master routes are independently checked. Neither small four-party fixture nor matched method proves10,000-party capacity, universal speed or controlled causality.

## Exact accounting and operation timers

Core native forms stage1,251/1,251IRR. Real Space preview readiness is62.015ms; durable post11.633ms (n1). Preview preserves durable state. Accepted16ms timer maxima preview16.089/post46.340ms meet strict50. Endpoint durations differ from heartbeat gaps and are not tails.

Fresh procurement profiles import the actual two-row synthetic catalogue SKU-A15IRR/SKU-B20IRR per piece, dated2026-03-22T09:00:00Z. Native four additive resources and explicit product/price selection retain frozen source provenance. External Space previews/posts PO12pieces/180IRR in53.206/12.542ms. Preview leaves durable state unchanged; PO is nonfinancial, with only opening journal.

Full native flow passes independent raw-state arithmetic:10 records (4Resources,1PO,2receipts,2bills,1transfer),5 journals including opening,4 stock movements. Receipts75/105IRR, bills30/150IRR, transfer6pieces/70IRR with no journal. W1=30pieces/350IRR, W2=12pieces/130IRR; stock42/480, AP329/GRNI50, trial1,431/1,431. Opening remains exact; cash/bank/AR/cheques/capital are literal expected values, advances/COGS/loss/P&L zero. Optional opening-GRNI bill is a different fixture, not measured here.

All22 accepted procurement phases have observed ticks and meet50ms; maximum43.674ms. The intentionally rejected74-versus-75 receipt preview (3ticks,27.464ms) is excluded from accepted-command acceptance. Missing/zero-tick phases would be unmeasured. Every operation/gap/result is retained separately; no pooled p95. Smoke160ms sequencing delay is excluded. Durability/financial arithmetic are checked independently of timer responsiveness.

## Complete quotation scale and remaining limits

Unchanged external `scripts/native-performance-M2.py` ran once on6, with the same budgets and semantic readiness endpoints. All15 functional checks pass: real50,000-row import/all500pages, cold/warm indexed pickers and composer,1,000canonical lines/exact100,802,501IRR, duplicate-safe explicit save/lock recovery/newer-invalid-input protection/focus, generation-preserving paging and busy keyboard/settings navigation/PDF. Its older250ms functional PDF guard is not performance acceptance.

| Strict full-scale endpoint (n1 workflow) | Freeze3 ms | Freeze5 ms | Freeze6 ms | Budget ms | Current result |
|---|---:|---:|---:|---:|---|
| Global heartbeat including setup/paging/screenshots | 497.934 | 469.552 | 280.552 | 50 | MISS |
| Maximum17 unchanged page clicks | 140.156 | 161.505 | 161.263 | 150 | MISS |
| PDF-busy heartbeat | 130.625 | 122.511 | 127.266 | 50 | MISS |

Historical M3 values were478.029/182.544/133.855ms. Freeze3 paging passed;5/6 miss. No rerun was used to obtain better numbers or excuse shared scheduling. Global/PDF50ms targets remain missed.

Current large-composer open dispatch15.859ms and copy-to-real100-native-row readiness169.550ms are separate. Freeze3 synchronous open-and-ready155.062ms;5 async16.888/177.251ms. In-app stage30 dispatches/stage130 awaits actual100rows without resetting phase start. Reduced dispatch does not imply faster total readiness or a tail result.

Actual unknown-line choice dispatch7.660ms; raw key `50000_unknown_line_choice_durable_and_native_review_ready`=540.580ms. That endpoint waits for persistence completion/callback and validates durable canonical chosen_code/exact amount. Callback dispatches `refresh_review()` asynchronously: final review rows are **not independently awaited**. It proves durable adoption/callback readiness, not full-render latency. Real product/customer opening and selection now use detached persistence and atomic current-form-plus-choice CAS. Other dependent commands and catalogue edit/delete remain future work; no blanket main-thread-work claim.

Warm-picker dispatch10.684ms versus actual30-native-row existence after preparation33.410ms is also separate. This original row-existence boundary does not add a paint/visible-focus acceptance claim. Necessary in-app readiness adapters await actual async picker/results/choice, with external M2 unchanged and no arbitrary waits or budget adjustment.

Explicit Save dispatch6.473ms. Raw `maximum_heartbeat_gap_ms_while_save_busy`=98.725ms misses50. The underlying persistence_busy flag now covers picker preparation/selection as well as Save, so this broad aggregate cannot isolate Ctrl+S blocking or establish a Save regression. Freeze5's46.677ms is retained with its narrower scope. Edited-page339.217ms is elapsed async `save_form_async_then` navigation, not blocking proof. Direct probe mutations/refresh dispatch remain distinct: unknown fixture179.659ms; restore188.299ms.

Global280.552ms is labelled stage126 after preceding stage26 captures the warm picker93.308ms and directly mutates the synthetic unknown-line fixture/refresh179.659ms. Those probe tasks total272.967ms; this does not isolate real user-command CPU. Stage30 is237.540ms versus5's469.552ms, after preceding direct restore. Heartbeats label current stage after transitions, so gaps cannot be assigned solely to that stage's operation. Screenshot, fixture, GTK/layout and scheduling confounds stay visible; strict endpoint still fails.

PDF completion1758.951ms. Its127.266ms busy gap is labelled stage9 after stage8 synchronously captures Settings113.942ms, switches to Order and resizes. Pure PDF/layout cost is not isolated. All large-gap phases and main-thread traces remain raw; screenshot costs materially affect the proxy but no miss is waived.

## Preservation, reproduction and untested scope

Original3 executable6eed73cc…44b4/sourceec50e593…56b9 and original5 executable7b390bb6…81a4a/source6b1996a4…aa36c each retain70-file manifests; all hashes reverified after6. Original resource-post50.761ms and focused214.167ms misses remain. Full reports are `performance/freeze3-report.md` and `performance/freeze5-report.md`. Freeze4 was never measured. Candidate6 changes the actual remaining synchronous picker save; its single measured run is not a rerun of5.

Activate `/workspace/sarv-environment/activate.sh`, private D-Bus and fresh ignored outputs:

```sh
dbus-run-session -- /usr/bin/python3 scripts/native-performance-M4.py \
  --binary /workspace/sarv-evidence/bin/sarv-M4-freeze6 \
  --expect-sha 9be5f0ba6a7fe6f000b722042d426987e79647578ab9ca803a9869ee1a343d5e \
  --samples 10 --search-samples 20 \
  --output artifacts/performance-m4/NEW-repeated
```

Run unchanged M2 separately with the same binary/hash/new output. Repeated startup needs retained synthetic M3 `native-before-current`50k/1000 SQLite seed; clone uses SQLite backup and asserts original source equality. Fresh posted profiles are separate synthetic mutations; read-only measurements preserve exact payload/generation. Binaries/databases/cache are excluded.

Nearest-rank p99 equals worst at n10/n20; n1 workflow/phase/memory is not a tail/leak estimate. Unmeasured: physical/cache-cold, reference4core/8GB/SSD, five-year/100,000document/million-journal-line/10,000party scale, Windows/Wayland, physical power loss/disk-full, packaging/physical print and user studies. Financial tests do not establish visual quality; screenshots do not establish financial correctness. No universal parity/speed, production readiness or statutory/tax compliance claim follows.
