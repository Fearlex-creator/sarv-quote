# M4 bounded native performance evidence

Freeze5 is the current measured candidate. Native procurement and full quotation functional workflows pass; performance acceptance remains partial. Core and procurement search, startup, initial RSS and all accepted procurement worker phases meet their proposed limits in this run. The complete long-quotation global, unchanged-page and PDF-busy limits remain missed. No budgets or timing endpoints were changed, and no run was repeated to obtain a better number. Original freeze3 observations remain immutable.

## Attribution and executor

- Immutable executable: `/workspace/sarv-evidence/bin/sarv-M4-freeze5`; SHA-256 `7b390bb6287b6803081b743e676f196de17840e9e6346e0d4c507f4d3fa81a4a`.
- Authoritative production aggregate: `6b1996a41ed86807a91a18820f7fce4bb167e99399403b3fd25cf1b14e4aa36c`. UTF-8 relative-path byte order, path/NUL/raw bytes/NUL; exact per-file provenance retained before/after.
- Matched historical helper digest: `824fdddee6a5203e9648e89a9e74d34d3324b864f124153eef253b20bc6cc4e6`. Pathlib component sorting represents the same files with a different order and is retained separately from the authoritative freeze digest.
- Production source, executable and observer dependencies were unchanged through the repeated suite. The unchanged M2 harness asserts binary before/after; production provenance immediately before and after the sequential M2 run is also equal.
- Actual hardware/runtime: five visible AMD EPYC 9V74 KVM vCPUs, four CPU equivalents (`cpu.max=400000 100000`), 16 GiB cgroup RAM, Debian 13.6, overlay filesystem, Rust1.99, GTK4.22.5/libadwaita1.9.4. Private X11/Cairo/Xvfb/Openbox, 1450×1050 scale1.
- Root held other Sarv GUI/build/test/render work quiet before GO; other work was cheap documentation/staging. M4 repeated then unchanged M2 scale ran sequentially. No shim or controlled cancellation diagnostic was included in timing. Loadavg and cumulative cgroup CPU stats before/after are retained; these are not continuous load samples or isolation of shared host scheduling.

[Portable freeze5 evidence](../docs/assets/accounting-m4/performance/freeze5/README.md) includes raw samples, logs, every worker timer gap/result, provenance, selected native screenshots and a standalone SVG plot. Application logs contain zero GTK/Adwaita/GLib warning/critical lines. External AT-SPI shutdown stale-cache notices are retained separately. No SQLite profile, executable or runtime cache is committed.

## Startup and memory

Each startup includes process launch, mapped owned window, visible usable Save, a real Tab focus roundtrip and completed PNG capture; AT-SPI/image observer overhead is included. First on a fresh backup may include initialization/migration. Repeated runs are new processes on an initialized backup with warm/uncontrolled OS pages. Physical/cache-cold launch was not measured; hashing/backup warms pages and no cache flush was performed.

| Fixture | First on fresh backup ms (n1) | Repeated p50 ms | p95 / worst ms (n10) | Steady RSS p95 KiB | Kernel HWM p95 KiB |
|---|---:|---:|---:|---:|---:|
| core-small-posted | 732.531 | 733.248 | 841.789 | 94632 | 99888 |
| quote-scale | 1265.150 | 981.016 | 1105.932 | 174432 | 174432 |
| procurement-small-posted | 783.566 | 786.395 | 870.600 | 97952 | 102260 |

All repeated startup p95 values meet 3,000 ms. All initial steady RSS p95 values meet 250,000,000 bytes. Application-only `/proc` memory is sampled every10ms during startup and after500ms idle; helpers are excluded.

Later workflow RSS/HWM: core 133,028 KiB; procurement 141,512 KiB; complete quote RSS 252,328, HWM 305,068 KiB. These later points do not establish initial-memory or leak acceptance.

## Real-key search

Core preserves the M3 observer: twenty alternating CU-A/CU-B real Ctrl+A/ASCII queries, exact one-row current ID, visible sensitive Edit, then native Tab to focused usable Edit. Discovery and initial focus are outside timing; key dispatch, debounce, rendering, five-ms polling and result navigation remain inside. Exact payload/generation equality is asserted after every read-only query.

| Stream / boundary | n | p50 ms | p95 ms | p99 / worst ms | 200-ms proposed p95 |
|---|---:|---:|---:|---:|---|
| Core visible Edit | 20 | 84.757 | 88.487 | 91.192 | PASS |
| Core focused Edit | 20 | 110.264 | 123.870 | 124.057 | PASS |
| Procurement visible Detail | 20 | 98.002 | 101.989 | 105.395 | PASS |
| Procurement focused Detail | 20 | 112.667 | 114.922 | 122.971 | PASS |
| Read-only party filter | 20 | 76.375 | 90.792 | 93.414 | Filter only; no selectable-action claim |
| Read-only item filter | 20 | 79.611 | 92.986 | 94.990 | Filter only; no selectable-action claim |
| Read-only mixed_master filter | 20 | 77.913 | 91.537 | 92.567 | Filter only; no selectable-action claim |

Procurement scope is the unique Purchases page. Ctrl+F reveals a showing/visible/sensitive SearchEntry before timing; no hidden same-caption Inventory entry can satisfy readiness. Queries alternate REC-A/REC-B, CU-A/CU-B, SKU-A/SKU-B and CU-A/SKU-B. Zero-result warmup and exact alternating one-result output reject stale readiness. Six operational documents and four resource events are counted separately, all ten are retained/searchable. Master rows remain read-only; their timings do not establish selectable party/product picker acceptance.

Historical M3 core visible/focused p95 was202.976/244.176ms; freeze3 was93.154/127.684; freeze5 is88.487/123.870. Freeze3 procurement focused Detail p95 missed200 at214.167; read-only diagnosis showed seven native Tabs. Freeze5 keeps the identical observer/budget and reaches the current filtered Detail through one native Tab, p95114.922ms. Default/backward/stale/master navigation is separately functionally checked; source fix, not observer shortcut, produces the route change. Same method does not establish a controlled causal experiment, production-scale lookup or universal speed.

## Exact workflow and timer phases

Core native forms stage exact1,251/1,251IRR. Real Space preview observed 61.120ms and durable post 6.241ms (n1). Preview leaves durable state unchanged. Accepted16-ms timer maxima are 16.082/39.248ms; both meet strict50ms. These endpoint durations differ from heartbeat gaps and are not population tails.

Separate fresh procurement profiles import actual two-row synthetic catalogue SKU-A15IRR/SKU-B20IRR per piece, source2026-03-22T09:00:00Z; frozen price provenance is asserted. Native additive resources and explicit product/price picker stage PO12pieces/180IRR. External Space preview/post observes 53.555/12.087ms. PO is nonfinancial: one opening journal remains; preview preserves durable state.

Full native flow passes independent raw-state arithmetic: ten records (4Resources, 1PO, 2receipts, 2bills, 1transfer), five journals including opening, four stock movements; receipts75/105IRR, matched bills30/150IRR, paired transfer6pieces/70IRR with no journal. W1=30pieces/350IRR, W2=12pieces/130IRR, total stock42/480, AP329/GRNI50, trial1,431/1,431. Opening remains exact; cash/bank/AR/cheques/capital remain literal expected values and advances/COGS/loss/P&L zero. Optional opening-GRNI billing is a separate fixture, not part of these timings.

All22 accepted procurement preview/post phases have observed ticks and meet strict50ms; accepted maximum40.801ms. The intentionally rejected74-versus-75receipt preview has three ticks/max26.201ms and is explicitly excluded from accepted-command acceptance. Missing/zero-tick phases would be unmeasured, not passing. Each operation/result/gap is retained separately, with no pooled p95. The smoke’s160-ms sequencing interval is excluded from worker claims. Durability and exact accounting are verified independently of timer responsiveness.

## Complete quotation scale and remaining pauses

Unchanged `scripts/native-performance-M2.py` runs once per changed frozen candidate with the same budgets/readiness endpoints. Freeze5 passes15 functional checks: real50,000-row import/all500pages, cold/warm indexed picker and composer, 1,000canonical lines/exact100,802,501IRR, duplicate-safe explicit save/lock recovery/newer-invalid-input protection/focus, unchanged-generation paging, busy keyboard/settings navigation and PDF completion. Its legacy250-ms functional PDF guard is not performance acceptance.

| Strict full-scale endpoint (n1 workflow) | Freeze3 ms | Freeze5 ms | Budget ms | Current result |
|---|---:|---:|---:|---|
| Global heartbeat including setup/paging/screenshots | 497.934 | 469.552 | 50 | MISS |
| Maximum of17 unchanged page clicks | 140.156 | 161.505 | 150 | MISS |
| PDF-busy heartbeat | 130.625 | 122.511 | 50 | MISS |

Historical M3 values were478.029/182.544/133.855ms. Freeze3 paging passed; freeze5 paging misses at161.505ms. The miss is retained, with no rerun to obtain a pass or claim that unchanged widget code proves constant speed. Global and PDF-busy50ms limits remain missed.

New large-composer open dispatch is 16.888ms; copy-to-real100-row readiness is 177.251ms, reported separately. Freeze3 synchronous open-and-ready was155.062ms. Background index preparation reduces synchronous dispatch; total readiness is not claimed faster. Probe stage30 now dispatches and stage130 awaits real100rows, preserving the original dispatch-to-ready endpoint and phase start. No throughput/tail claim follows from n1.

Actual explicit Save dispatch is 5.500ms and save-busy heartbeat 46.677ms, within50. Edited-page navigation is365.987ms elapsed and already uses `save_form_async_then`; it is not proof of blocking. Direct probe fixture mutations/refresh dispatch remain distinct (173.527ms restore, 191.448ms unknown-line setup).

The global469.552ms gap is labelled stage30, but heartbeat labels the current stage after transitions. Preceding stage28 captures the unknown selector84.376ms, invokes the actual product Choose callback, performs a direct synthetic restore173.527ms, and advances30. `src/ui/pickers.rs` still calls `save_form` and `store.change(chosen_code)` synchronously in that actual unknown-line Choose path. Its individual contribution is not instrumented; the composite gap cannot assign469.552ms solely to composer open, selection save or GTK layout. This remaining synchronous path is distinguished from async Save/page navigation; no unsupported rewrite or exact isolated duration is claimed.

PDF completes in 1726.096ms. Its122.511ms maximum is labelled stage9 while PDF is busy, after stage8 synchronously captures Settings113.583ms, switches to Order and resizes. Screenshot/resize/layout and scheduling confound pure PDF-side attribution. Raw named main-thread traces/all large-gap phases remain visible; screenshots materially contribute to the proxy but the strict endpoint still fails.

## Historical preservation, reproduction and limits

Original freeze3 executable SHA6eed73cc…44b4/sourceec50e593…56b9 remains backed by its original70-file manifest, all hashes reverified after freeze5. Original resource-post50.761ms and focused-search214.167ms misses remain recorded. Freeze4 was never measured: another genuine loading-status fix required freeze5. Freeze3 full report is retained separately. Current results replace neither old observations nor their limitations.

Activate `/workspace/sarv-environment/activate.sh`, use private `dbus-run-session` and fresh ignored output directories:

```sh
dbus-run-session -- /usr/bin/python3 scripts/native-performance-M4.py \
  --binary /workspace/sarv-evidence/bin/sarv-M4-freeze5 \
  --expect-sha 7b390bb6287b6803081b743e676f196de17840e9e6346e0d4c507f4d3fa81a4a \
  --samples 10 --search-samples 20 \
  --output artifacts/performance-m4/NEW-repeated
```

Run unchanged M2 separately with the identical binary/hash and new output path. Repeated startup needs the retained synthetic M3 `native-before-current` 50k/1000 SQLite seed; it uses SQLite backup and asserts original source equality. Fresh posted profiles are separate authorized synthetic mutations; read-only measurement profiles preserve exact payload/generation. Executables/databases/cache are excluded from portable artifacts.

Nearest-rank p99 equals worst at n10/n20. n1 workflow/preview/post/memory are not tails or leak checks. Unmeasured: physical/cache-cold startup, reference four-core/8GB/SSD, five-year/100,000-document/million-journal-line/10,000-party scale, Windows/Wayland, physical power loss/disk-full, packaging/physical printing and user studies. Financial tests do not establish visual quality; screenshots do not establish financial correctness. Results cannot support universal parity/speed, production readiness or statutory/tax compliance.
