# M4 bounded native performance evidence

The frozen M4 freeze3 candidate was measured after native and independent review completed. Functional procurement and quotation workflows pass; performance acceptance remains partial. Core lookup and unchanged quotation paging improve against historical M3 observations, while document focus navigation, one resource-post heartbeat, global long-quotation heartbeat and PDF-busy heartbeat miss their unchanged proposed limits. No performance budget was waived and no timing run was repeated to obtain a better result.

## Executable, source and executor

- Immutable executable: `/workspace/sarv-evidence/bin/sarv-M4-freeze3`; SHA-256 `6eed73ccede51feadd3353393315667329f8136205ad46c973c9e9ece6da44b4`.
- Authoritative production aggregate: `ec50e593693bff141271b490cf2cce74005c3b3117324d7285fa9e6f631b56b9`. Sorted UTF-8 relative path bytes, then path/NUL/raw bytes/NUL, matches freeze3. Ordered per-file provenance is retained before/after.
- Matched historical helper digest: `aee5c54380d2d201489b6e5dc19f3785cbfbb9277c1beaa5f90548af06372ae5`; its pathlib-component order differs from freeze ordering. It represents the same files and is retained independently.
- Production source, executable and all observer dependencies were unchanged across the repeated measurement. The unchanged M2 harness asserts executable equality before/after; the provenance chain also confirms production equality immediately before and after its run.
- Actual executor: Debian 13.6, five visible AMD EPYC 9V74 KVM vCPUs; `cpu.max=400000 100000` (four CPU equivalents), 16 GiB cgroup RAM, overlay filesystem, GTK 4.22.5/libadwaita 1.9.4, Rust 1.99. Private X11/Cairo/Xvfb/Openbox, 1450×1050 scale 1.
- Root explicitly held other Sarv GUI/build/test work before GO. Repeated M4 and M2 scale ran sequentially; other task work was documentation/staging. Shared host scheduling and cache state were uncontrolled. Load was sampled only after all runs, not continuously; CPU pressure data was unavailable.

Raw results, application logs, source files hashes, phase traces, native screenshots and sample plot are in [the portable bundle](../docs/assets/accounting-m4/performance/README.md). No SQLite profile, binary or runtime cache is committed. Application logs contain zero GTK/Adwaita/GLib warning or critical lines. The external observer retained AT-SPI stale-cache notices when owned processes stopped; these are distinguished from application diagnostics.

## Startup and memory

Startup includes process dispatch, mapped owned window, visible usable Save, real XTest Tab focus roundtrip and PNG completion. Accessibility/image overhead is included. First on a fresh SQLite backup may include initialization/migration. Repeated runs are new-process starts on an initialized profile with warm/uncontrolled OS pages; physical/cache-cold startup was not measured. Copying/hashing warms pages; no cache flush was performed.

| Fixture | First on fresh backup ms (n=1) | Repeated p50 ms | p95 / worst ms (n=10) | Steady RSS p95 KiB | VmHWM p95 KiB |
|---|---:|---:|---:|---:|---:|
| core-small-posted | 737.509 | 769.409 | 817.881 | 94100 | 99524 |
| quote-scale | 1335.472 | 1013.046 | 1156.314 | 173948 | 173948 |
| procurement-small-posted | 862.630 | 789.328 | 847.016 | 97256 | 102568 |

All three repeated startup p95 values meet the proposed 3,000-ms target. Initial steady RSS meets the proposed 250,000,000-byte target for each fixture. Application-only memory is sampled every 10 ms during startup and once after 500 ms idle; helper processes are excluded.

After the core workflow RSS/HWM is 129,792 KiB; after full procurement it is 138,976 KiB. After the complete quote scale workflow RSS is 294,816 KiB and HWM 339,136 KiB. These later points are reported separately and cannot establish initial-memory or leak acceptance.

## Real-key searches

The matched core observer preserves historical M3 dispatch/readiness: twenty alternating CU-A/CU-B queries from real Ctrl+A/ASCII keys, exact one-row ID and visible sensitive Edit action, then native Tab to a focused usable action. Discovery/initial focus occurs outside timing; key dispatch, debounce, rendering, five-millisecond accessibility polling and result-focus navigation remain inside. Each read-only query preserves the exact SQLite payload/generation.

| Stream / boundary | n | p50 ms | p95 ms | p99 / worst ms | Proposed 200-ms p95 |
|---|---:|---:|---:|---:|---|
| Core visible Edit | 20 | 86.786 | 93.154 | 104.812 | PASS |
| Core focused Edit | 20 | 116.242 | 127.684 | 127.868 | PASS |
| Procurement visible Detail | 20 | 97.836 | 109.185 | 155.576 | PASS |
| Procurement focused Detail | 20 | 179.791 | 214.167 | 234.349 | MISS |
| Read-only party filter | 20 | 79.841 | 92.565 | 92.594 | Read-only filter; no action acceptance |
| Read-only item filter | 20 | 78.806 | 81.884 | 99.540 | Read-only filter; no action acceptance |
| Read-only mixed master filter | 20 | 76.880 | 91.547 | 94.910 | Read-only filter; no action acceptance |

Historical M3 core visible/focused p95 values were 202.976/244.176 ms; current values are 93.154/127.684 ms with the same timing boundaries. This matched method is evidence of this run, not a controlled causal experiment or a 10,000-party speed claim. Procurement focused-action p95 **214.167 ms misses 200 ms**; native Tab and whole-page focus scrolling are included, with no endpoint change to conceal navigation cost.

Procurement queries alternate REC-A/REC-B in Documents, CU-A/CU-B in Resources, SKU-A/SKU-B for items and CU-A/SKU-B for mixed masters. A zero-result warmup and exact alternating one-result output prevent stale readiness. Purchases is scoped by its unique heading; Ctrl+F reveals its showing/visible/sensitive SearchEntry before timing. Six operational documents and four resource events are counted separately, all ten remain searchable. Master rows are read-only and searches retain focus; their figures do not certify selectable party/product picker interaction.

## Exact fixture and worker responsiveness

The frozen native forms staged a 1,251/1,251-IRR detailed opening; the observer preview/post used real Space and preserved durable state during preview. Core observed preview readiness was 61.737 ms and durable post 6.310 ms (n=1). Its accepted 16-ms GTK timer phase maxima were 16.098 ms for preview and 41.248 ms for post, both within the strict 50-ms proxy. These timings measure different endpoints and are not population tail estimates.

Fresh isolated procurement profiles import the actual two-row synthetic catalogue (SKU-A 15 IRR and SKU-B 20 IRR, piece, 2026-03-22T09:00:00Z), create four additive resources and use explicit priced product selection. The external observer verifies a newly reviewed 12-piece PO/180 IRR with one financial opening journal only: PO is nonfinancial. PO observed preview was 55.646 ms, durable post 12.472 ms. Catalogue bytes/hash and frozen price provenance are retained.

The separate full native workflow passes independent raw-state assertions: ten records (4 Resources, 1 PO, 2 receipts, 2 bills, 1 transfer), five journals including opening and four stock movements; receipts 75/105 IRR, bills 30/150 IRR, transfer 6 pieces/70 IRR with no journal. W1 closes at 30 pieces/350 IRR, W2 at 12 pieces/130 IRR, total inventory 42/480, AP329, GRNI50 and trial totals 1,431/1,431. Cash/bank/AR/cheques/capital remain literal expected values; advances/COGS/loss/P&L remain zero and opening is byte-for-byte equal. The optional opening-GRNI bill is not part of this measurement.

Procurement phase traces retain operation, preview/post endpoint, every timer gap, tick count and result. **21 of 22 accepted phases meet 50 ms; one Resources post is 50.761 ms and misses.** The intentionally rejected 74-versus-75 receipt preview has three ticks and a 22.853-ms maximum but is excluded from accepted-command responsiveness. All accepted phases have observed ticks; no absent/zero-tick phase is accepted. Remaining accepted operation maxima are 41.951 ms or lower. The smoke’s 160-ms sequencing delay is excluded from worker latency claims. Durability and arithmetic are checked separately from timer responsiveness.

## Preserved complete quotation scale probe

The unchanged `scripts/native-performance-M2.py` ran once against the same exact freeze3 executable. Its 15 functional checks pass: real 50,000-row import, all 500 catalogue pages, cold/warm indexed pickers, catalogue composer, 1,000 canonical quote lines, exact 100,802,501 IRR, save duplicate suppression/lock recovery/input focus, unchanged-generation paging, newer-invalid-input protection and PDF completion. Functional success does not accept the older probe’s 250-ms PDF guard as the performance budget.

| Strict scale endpoint (n=1 workflow) | Actual ms | Budget ms | Result |
|---|---:|---:|---|
| Global heartbeat including setup/screenshots/paging | 497.934 | 50 | MISS |
| Maximum of 17 unchanged editor page clicks | 140.156 | 150 | PASS |
| PDF-busy GTK heartbeat | 130.625 | 50 | MISS |

Historical M3 values were 478.029/182.544/133.855 ms respectively. The current unchanged-page maximum meets 150 ms; global and PDF-busy 50-ms limits remain missed. The global probe includes synchronous screenshot capture (retained main-thread trace) and scheduling, so it is not a pure continuous CPU profile; it still fails its declared endpoint. The 497.934-ms gap is labelled stage30, but heartbeat labels the current stage after transitions: the preceding stage28 callback captures an unknown selector (87.090 ms), changes the synthetic fixture and refreshes it (192.465 ms), and advances30; stage30 opens the large composer (155.062 ms). This composite observation cannot assign all 497.934 ms to one product operation.

Explicit save dispatch is 5.471 ms and its save-busy heartbeat maximum 44.801 ms. Edited first-page navigation takes 356.982 ms, but its source already uses `save_form_async_then`: this is elapsed asynchronous navigation, not proof of GTK blocking. The 188–194-ms changed-store/review observations are direct synthetic fixture mutations inside the probe, not ordinary Save dispatch. Some dependent commands and row rebuilds remain synchronous according to the source/probe limits; their individual contribution to these composite gaps is not established. PDF completion is 1,756.640 ms with 68 busy timer ticks; its 130.625-ms maximum remains a separate responsiveness miss. That gap is labelled stage9 after the preceding stage8 captured Settings (100.831 ms), switched back to Order and resized the window. Screenshot/resize/layout and shared scheduling confound isolation of PDF work; no stack trace establishes a pure PDF-side GTK pause.

## Reproduction and remaining limits

Activate `/workspace/sarv-environment/activate.sh`, use a fresh output directory and run:

```sh
dbus-run-session -- /usr/bin/python3 scripts/native-performance-M4.py \
  --binary /workspace/sarv-evidence/bin/sarv-M4-freeze3 \
  --expect-sha 6eed73ccede51feadd3353393315667329f8136205ad46c973c9e9ece6da44b4 \
  --samples 10 --search-samples 20 \
  --output artifacts/performance-m4/NEW-repeated
```

Run the unchanged M2 harness separately with the same binary/hash and a new output path. It creates a fresh synthetic profile. Repeated startup requires the retained synthetic M3 `native-before-current` SQLite scale fixture; it clones via SQLite backup and asserts original source equality. The bundle excludes databases and executable.

Nearest-rank p99 equals worst for n=10/n=20. n=1 post/preview/full-workflow and memory points are not tail or leak estimates. Missing physical/cache-cold, reference four-core/8-GB/SSD, five-year/100,000-document/million-journal-line/10,000-party, Windows/Wayland, physical power-loss/disk-full, packaged/physical-print and user-study evidence remains untested. Results cannot support universal speed/parity, production readiness or statutory/tax compliance.
