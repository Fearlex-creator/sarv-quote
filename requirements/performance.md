# Performance evidence: bounded G1 checkpoint

PERF-01 and PERF-02 remain **partial**, not accepted for the full reference profile. Measurement is evidence for the declared cloud executor only. The product requirement proposes four CPU cores, 8 GB RAM, SSD, five fiscal years, 100,000 documents, 1,000,000 journal lines, 50,000 items and 10,000 parties. The current slice has neither that complete fixture nor that hardware certification.

## Reproduction and boundaries

The harness is `scripts/accounting-performance.py`. Source `/workspace/sarv-environment/activate.sh` and run under an isolated `dbus-run-session`, passing the release binary SHA-256 explicitly. It never builds, selects a regular desktop process, writes to its source fixture, or reads live business data. It clones two allowlisted synthetic SQLite profiles with the consistent SQLite backup API. It compares the initialized payload and generation after every sample to ensure measurement did not change business state.

Declared executor: 5 x86-64 virtual CPUs, AMD EPYC 9V74 under KVM; Debian 13.6; overlay filesystem; approximately 17.6 GiB kernel-visible RAM. Cgroup limit, OS/CPU details and filesystem output are stored in the JSON evidence. These facts do not establish an SSD or a dedicated desktop's scheduling characteristics. The native stack is Rust 1.99, GTK 4.22.5, stable libadwaita 1.9.4, X11/Cairo, private Xvfb 1450 × 1050 at scale 1 with Openbox. Existing concurrent executor tasks may introduce contention.

Startup begins immediately before application process dispatch. It ends after the owned native window is mapped, the editor's Save control is accessible, a real XTest Tab key receives a native changed-focus response, and a real X11 screenshot completes. The observer stops traversal at relevant controls; it does not enumerate unrelated document rows. This is a usable/rendered-screen proxy with observer overhead: accessibility roundtrips, polling and screenshot acquisition are included. Inspect the accompanying native screenshot rather than inferring visual quality from its timing.

The first process on each copied fixture is reported separately; an old-schema migration may be included. Then ten new application processes use the initialized profile and the same already-running private display/accessibility session. All ten are **process cold, OS cache warm or uncontrolled**. Binary hashing and consistent fixture cloning intentionally warm those file pages before launch; library/font cache state is uncontrolled. No cache is flushed; none is described as physical/machine cold. There is no second application activation masquerading as a fresh process. Warm lookup samples occur inside the final scale-fixture process after the indexed picker is prepared.

Search begins at native AT-SPI editable-text replacement dispatch and ends at the exact expected product's visible list item with AT-SPI selectable and sensitive states. The observer traverses the picker container rather than the whole main window. It includes debounce, GTK update and accessibility observer overhead. Twenty samples alternate between the last two exact product codes to prevent stale results from satisfying readiness. These tests establish exact-code lookup only; broad aliases, party lookup and ledger search are separate acceptance obligations.

RSS belongs to the Sarv application PID alone. A sampler reads `/proc/PID/status` every 10 ms from process launch; kernel VmHWM records peak resident memory and VmRSS after 500 ms idle records steady memory. The X server, desktop manager, accessibility registry and observers are excluded. KiB values are reported separately from MiB conversions. This is resident process memory, not total installation or desktop memory.

Quantiles use nearest rank. Startup n=10 and lookup n=20 are small samples: p99 equals the worst observed value, not a reliable estimate of the population's 99th percentile. Unlike datasets and startup/migration phases are not pooled. The evidence stores every sample, executable SHA-256, Git HEAD/dirty state and a deterministic hash of source Rust/CSS/Cargo files before and after.

## Fixtures

| Fixture | Shape | Limit |
|---|---|---|
| G1 independent reviewer | 1 company, 2 periods, 2 accounts, 3 journals, 4 requests, 10 audit rows; no catalogue or quote lines | Kernel slice, not a full annual ledger or service-accounting business |
| Existing quote scale | 50,000 products, 1,000 draft lines, no customers or posted accounting | Old quotation schema migrated on a private copy; not the full PERF-01 fixture |

Scale fixture generator and seed/source are in the native responsiveness test (`src/ui/responsiveness.rs`). The G1 fixture was created by the independent reviewer probe. Fixture provenance, counts, original and migrated payload hashes are retained in the JSON. The source synthetic data remains untouched.

## Baseline observations

The preserved baseline artifact is `/workspace/sarv-evidence/native-before-current/native-responsiveness-verification.json`. It contains **one** run of each named workflow, not repeated samples and not quantile claims. Exact-code lookup was 273.915 ms, broad indexed lookup 285.365 ms, composer exact-code lookup 262.875 ms and unknown-line lookup 255.178 ms; all include debounce. Cached picker opening was 9.200 ms; initial index/first results 136.929 ms. A 1,000-line save plus first page construction took 265.704 ms and unchanged editor page clicks ranged from 162.041 to 207.920 ms. Its 20 ms GTK heartbeat reached a worst gap of 810.166 ms and 111.412 ms while PDF export was busy. PDF completion was 1709.234 ms.

The baseline native probe's functional `passed` outcome does not pass the proposed performance budgets. Its 250 ms PDF-busy heartbeat threshold is looser than PERF-01's proposed 50 ms continuous-block budget. A timer gap can include executor scheduling delays as well as main-thread work, so it is an observed responsiveness gap rather than a CPU profiler's proof of a single continuous blocking function. Report misses rather than recasting the budget. Source inspection found an important timing limit: native-probe exact and broad lookup steps deliberately wait 250 ms (`p.advance(..., 250)`) before observing the results. Their 255–285 ms readings include that fixture delay and cannot establish query-to-first-ready latency or prove a 200 ms budget miss. The repeated external observer below measures actual readiness without that deliberate 250 ms floor.

## Native save and long-document observations

The final functional artifact is `/workspace/sarv-evidence/native-after-final/native-responsiveness-verification.json`, confirmed by its owner against release SHA-256 `65bedf489894bd6435b57a9919fadf0a31e2e1d9809f97089212dbf43bfc4378`. All twelve native checks passed, including locked-worker navigation, double-save commit-once, invalid-newer-input preservation, restart, every 1,000-line editor page and exact 100,802,501 IRR retained total. These are single workflow observations, not p95 results.

| Observed operation | Baseline | Post-save checkpoint | Interpretation |
|---|---:|---:|---|
| Explicit 50k catalogue / 1k-line save dispatch on GTK thread | Not separately measured | 6.510 ms | Worker dispatch leaves GTK responsive; this is not durable-commit completion |
| Worst heartbeat while new save worker busy | Not available | 46.337 ms across 26 ticks | Under 50 ms in this phase only; global target remains unaccepted |
| Whole-probe worst heartbeat | 810.166 ms | 970.604 ms | Remaining synchronous snapshot/widget/composer phases exceed budget; no overall improvement claim |
| Unchanged editor page worst action time | 207.920 ms | 342.736 ms | Above proposed 150 ms navigation budget in both observed runs |
| 1,000-line PDF request to completion | 1709.234 ms | 2123.870 ms | Both completed; shared executor and n=1 prohibit a speed conclusion |
| Worst PDF-busy heartbeat | 111.412 ms | 148.553 ms across 84 ticks | Under probe's loose 250 ms functional threshold; above proposed 50 ms requirement |

The final deferred changed-page duration (490.806 ms) includes worker persistence, completion, widget rebuild and the probe's next 30 ms poll; its older synchronous-click duration (316.664 ms) has a different measurement boundary. Do not subtract them as an equal-boundary before/after speed comparison. The new exact-copy async-save workflow fixes the observed SQLite-lock UI pause and preserves newer edits, but does not establish a globally nonblocking UI. Catalog acceptance, some dependent commands, snapshot clones and row widget rebuilds retain measured gaps.

Failed intermediate artifacts `native-after` and `native-after-v2` are preserved for debugging; they are not acceptance evidence. Prior passed `native-after-v3` on SHA `ae35fe5d…` observed 59.002 ms save-busy maximum; it remains a real prior observation, rather than being erased in favour of the final 46.337 ms observation. The final source also fixes explicit-save review refresh. Native lookup timer readings remain subject to the deliberate 250 ms probe floor explained above.

## Results and remaining gates

Final raw evidence: `/workspace/sarv-evidence/performance-final-checkpoint/performance.json`. Immutable release: `/workspace/sarv-evidence/bin/sarv-quote-final-checkpoint`, SHA-256 `65bedf489894bd6435b57a9919fadf0a31e2e1d9809f97089212dbf43bfc4378`. Source Rust/CSS/Cargo hash `115c2e7425e4f1bc2388fd9720c678fbddc96ba3312634b405b0a5d79bdfe6f4` and binary hash stayed identical throughout measurement. The source HEAD was `4aa82c2988fb2ea48a2fa2ae9824c06187f3d76a` plus the checkpoint's uncommitted implementation; the integrating local commit subsequently identifies that work. Cgroup memory limit was 16 GiB.

Reproduction (use a new output directory each time):

```sh
source /workspace/sarv-environment/activate.sh
dbus-run-session -- /usr/bin/python3 /workspace/sarv-quote/scripts/accounting-performance.py --binary /workspace/sarv-evidence/bin/sarv-quote-final-checkpoint --expect-sha 65bedf489894bd6435b57a9919fadf0a31e2e1d9809f97089212dbf43bfc4378 --output /workspace/sarv-evidence/performance-rerun
```

| Repeated operation, ms | n | p50 | p95 | p99 / worst | Bounded result |
|---|---:|---:|---:|---:|---|
| Small G1 fresh-process usable rendered startup, warm/uncontrolled OS cache | 10 | 776.503 | 885.615 | 885.615 | Below proposed 3 s budget on this executor/fixture |
| 50,000-product / 1,000-line quote fresh-process usable rendered startup, warm/uncontrolled OS cache | 10 | 1111.394 | 1428.802 | 1428.802 | Below proposed 3 s budget on this executor/fixture |
| Same-process indexed exact-code product lookup, including debounce/visible selectable result | 20 | 174.035 | 193.580 | 198.957 | Below proposed p95 ≤ 200 ms budget on this exact-code fixture |

Previous repeated observer evidence is preserved at `/workspace/sarv-evidence/performance-final/performance.json` for SHA `ae35fe5d…`: exact-code lookup n=20 was p50 251.070 ms, p95 265.979 ms, p99/worst 268.389 ms and missed the proposed 200 ms budget. The final measured run follows the bounded picker delay change to 80 ms and bidi isolation fix. Its p95 is below 200 ms on the same fixture; this is not a claim about every query shape or the full reference dataset.

First-on-clone process startup was 767.255 ms for G1 and 1522.834 ms for the old-schema quote fixture. Both are single observations with uncontrolled OS cache; the latter includes migration on a private copy. They do not establish OS-cache-cold quantiles.

| Application resident memory, MiB | Steady p50 | Steady p95 / worst | Kernel peak p95 / worst |
|---|---:|---:|---:|
| Small G1 startup/500 ms idle, n=10 | 84.26 | 85.16 | 91.75 |
| Scale quote startup/500 ms idle, n=10 | 145.64 | 146.50 | 149.52 |
| Scale quote after indexed picker/query workload, one observed point | 190.04 | No quantile claim | 190.04 |

These application RSS measurements are below the proposed initial 250 MiB budget locally. They do not measure report/import/PDF peak memory or the full reference dataset. Every initialized payload and generation remained identical after each sample. The quote source was schema 1; the copied measurement profile migrated to schema 2. The untouched source fixture and its original hashes are recorded separately.

Native screenshots were visually inspected after capture: `/workspace/sarv-evidence/performance-final-checkpoint/g1-small/startup-0.png`, `/workspace/sarv-evidence/performance-final-checkpoint/quote-scale/startup-0.png`, and `/workspace/sarv-evidence/performance-final-checkpoint/quote-scale/picker-search.png`. They show usable Persian forms, a rendered quote page, a visible focus ring after a real Tab key, and the exact expected selected tail product. The picker source timestamp and SCALE code now retain readable left-to-right order within the Persian row; this inspection does not accept all of IR-01 or native-reader UX. A startup-time result is not an overall visual-quality endorsement.

Observer-development pilots under `performance-observer-pilot*` are excluded from final results: one used unsupported GTK AT-SPI focus acquisition, and an earlier harness enumerated the whole accessibility tree and inflated large-document timing. The final harness uses real XTest Tab and bounded traversal. Final process logs contain no Sarv application diagnostic warnings; the external observer emitted stale AT-SPI cache warnings when an already-terminated owned process disappeared, and Xvfb emitted its expected connection-close message at cleanup. These are disclosed rather than counted as product focus diagnostics.

The full million-line ledger, annual trial balance/P&L, durable 20-line invoice posting, 10,000-row staged import, broad party lookup, report rendering, OS-cache-cold startup, supported desktop hardware, Wayland, physical printing and packaged distribution remain **untested here**. The small G1 workflow does not implement an invoice module, so no invoice-posting time can be claimed. No superior-speed claim or production performance acceptance follows from Rust, a passing functional probe, or this bounded benchmark.
