# M2 bounded performance and native navigation evidence

PERF-01/PERF-02 remain partial. This checkpoint measures the existing G1 and quotation scale fixtures on the shared executor; it does not establish the full reference accounting dataset, production hardware or superior speed. The historical M1 report in `requirements/performance.md` remains unchanged.

## Measurement boundaries

The repeated observer is `scripts/accounting-performance.py`: an immutable release binary, private D-Bus/X11/Xvfb/Openbox session, read-only allowlisted synthetic source profiles, and SQLite consistent backups into a fresh output directory. It never runs against a regular desktop process or live business data.

Startup begins before process dispatch and ends after the owned native window maps, its Save control is accessible, a real XTest Tab changes native focus and screenshot acquisition completes. Accessibility polling and image capture are included. Each repeated sample is a new application process on the initialized copied profile, with OS pages warm or uncontrolled. The first process on each copy is separate and may include schema migration. No machine/cache-cold startup is claimed.

Exact-code product lookup begins at AT-SPI editable-text replacement dispatch and ends at the correct visible, sensitive, selectable list item. Twenty samples alternate the last two catalogue codes to prevent a stale result from satisfying readiness. This includes debounce, GTK rendering and accessibility observer overhead; alias, party and ledger lookup are separate obligations.

Nearest-rank quantiles use n=10 per startup fixture and n=20 for exact-code lookup. With these small samples, p99 equals the worst value and does not estimate a population tail reliably. Different fixtures, migration/startup phases and full native workflows are never pooled.

The application-only `/proc/PID/status` RSS sampler runs every 10 ms. Steady startup memory is sampled after 500 ms idle; kernel VmHWM captures peak resident memory. Helpers and the X server are excluded. Full native workflow RSS/peak and startup RSS/peak are different boundaries and are reported separately.

After the first process initializes each copied profile, payload and generation must remain identical after every measurement. Source fixture shapes/hashes, copied initialized/final shapes, binary hashes and Rust/CSS/Cargo source hashes before/after are recorded in JSON. Initial migration/default initialization on a disposable copy is excluded from this read-only equality baseline.

## Declared synthetic fixtures

| Fixture | Original source | Shape and limit |
|---|---|---|
| G1 small | `/workspace/sarv-evidence/independent-ledger-final/profile` | Schema 2, generation 8; 1 company, 2 periods, 2 accounts, 3 journals, 4 requests, 10 audit rows; no catalogue or draft rows |
| Quote scale | `/workspace/sarv-evidence/native-before-current` | Schema 1, generation 7; 50,000 products and 1,000 draft rows; no customers or posted accounting |

The quote source is the deterministic synthetic native responsiveness fixture from `src/ui/responsiveness.rs`; the G1 source is the earlier independent reviewer fixture. Original sources remain unchanged. Neither contains five fiscal years, 100,000 documents, one million journal lines or 10,000 parties.

## Untested scope

OS-cache-cold launch, the full reference fixture, dedicated four-core/8-GB/SSD hardware, annual reporting, durable invoice posting, 10,000-row staged import, broad party lookup, Wayland/Windows, packaged distribution, physical printing, screen-reader speech, high contrast and enlarged text remain untested in these observers. Timer gaps include scheduling and screenshot observer work; they are not a CPU profiler's proof of one continuous blocking function. Passing functional checks does not accept missed performance budgets.

## Native sidebar and narrow history

Final immutable executable: `/workspace/sarv-evidence/bin/sarv-M2-layout-final`, SHA-256 `6c545a5642c53ac633a715f66f5f24cdfd107ccd58261724b688a89f1793a50d`. `scripts/native-sidebar-qa.py` passed 19 checks against this executable: all preserved Alt+1…7 routes, unchanged saved payload after navigation, real Tab/Enter narrow-menu opening, focus on the current destination, Escape recovery to the menu toggle, arrow/Enter destination activation, native dark-appearance save and complete restart equality. Binary hash remained unchanged throughout the run.

The additional narrow-history checks use actual Tab to reach the accounting history tab at 650×760, GTK focus geometry from the real XTest event to establish the entire control is inside the native viewport, and Space plus AT-SPI selected state to prove activation. This corrects an observer assumption: GTK's custom-role FlowBox tab accessibility component returned x/y zero even while the screenshot and native focus geometry showed the tab at its actual location. The failed development attempt is retained under `artifacts/sidebar-native/run-c9svlc9m`; it is not acceptance evidence.

Final report, native keyboard event log and images: `/workspace/sarv-evidence/sidebar-M2-layout-final/`. Standard 1280×900 light/focus and dark screens, 650×760 light/dark sidebar focus screens, and `accounting-narrow-history-light.png` were inspected. They show restrained Persian RTL groups, one distinct current destination, a separate native focus outline, reachable footer utilities and fully visible history navigation. Native application logs were empty. The shared sidebar surface does not represent deferred purchases, stock, settlements or statutory modules as available destinations.

Reproduction:

```sh
source /workspace/sarv-environment/activate.sh
SARV_QA_BINARY=/workspace/sarv-evidence/bin/sarv-M2-layout-final dbus-run-session -- /usr/bin/python3 scripts/native-sidebar-qa.py
```

## Executor declaration

The executor exposes five x86-64 virtual CPUs, AMD EPYC 9V74 under KVM, Debian 13.6, an overlay filesystem and a 16-GiB cgroup memory limit. This is not an SSD or dedicated desktop declaration. Runtime: Rust 1.99.0, GTK 4.22.5, stable libadwaita 1.9.4; X11/Cairo, private Xvfb 1450×1050 at scale 1 with Openbox. Full raw OS/CPU/filesystem/memory details accompany repeated measurement JSON. Parallel native acceptance checks are completed before the repeated benchmark starts to reduce deliberate contention; shared-executor scheduling remains uncontrolled.

## Final repeated results and provenance

[Raw final evidence](../docs/assets/accounting-m2/performance/performance.json) is retained from `/workspace/sarv-evidence/performance-M2-layout-final/performance.json`, measured at 2026-10-07T17:33:21Z. The recorded source HEAD was `d640288d90d9f21789cd213ecbaa85e16c69bcb5` plus the integrating M2 changes. Binary SHA-256 `6c545a5642c53ac633a715f66f5f24cdfd107ccd58261724b688a89f1793a50d` and source Rust/CSS/Cargo SHA-256 `67ab0ceb08f7ba2aa9a91d99b016b1c5f1e48bd906d7ff58d75f88902644a363` were identical before and after measurement. The integrating checkpoint subsequently identifies that exact source.

| Observed operation, milliseconds | n | p50 | p95 | p99 / worst | Bounded result |
|---|---:|---:|---:|---:|---|
| G1 small new-process startup; warm/uncontrolled OS cache | 10 | 725.258 | 825.448 | 825.448 | Below proposed 3-second target at this observer boundary |
| 50,000-product / 1,000-line quote new-process startup; warm/uncontrolled OS cache | 10 | 1,088.792 | 1,274.927 | 1,274.927 | Below proposed 3-second target at this observer boundary |
| Same-process exact-code product lookup to visible selectable result | 20 | 169.683 | 184.853 | 190.657 | Below proposed p95 ≤200 ms for these exact-code queries |

The first process on a fresh clone took 769.768 ms for G1 and 1,391.028 ms for quote scale. These single observations include schema3 initialization/migration where applicable, with uncontrolled OS caches. They are excluded from repeated quantiles.

| Application resident memory | n / boundary | Steady p50, KiB (MiB) | Steady p95 / worst, KiB (MiB) | Kernel HWM p95 / worst, KiB (MiB) |
|---|---|---:|---:|---:|
| G1 small | 10; startup +500 ms idle | 87,448 (85.40) | 88,308 (86.24) | 94,708 (92.49) |
| Quote scale | 10; startup +500 ms idle | 170,544 (166.55) | 171,324 (167.31) | 171,324 (167.31) |
| Quote scale after picker/query workload | 1 point; same final process | 204,644 (199.85) | No quantile claim | 204,644 (199.85) |
| Complete native import/editor/picker/composer/PDF workflow | 1 separate reviewer process | 282,864 (276.23) | No quantile claim | 311,204 (303.91) |

Initial resident memory is below the proposed 250-MB initial budget on these small synthetic fixtures. The full-workflow point exceeds that numerical threshold, with a different workload and boundary; it is not an initial-RSS measurement. No global memory budget acceptance, leak diagnosis or full-reference-fixture claim follows from these observations.

The G1 copy migrated schema2/generation8 to schema3/generation9 before the read-only baseline; initialized and final payload SHA-256 are both `33cb7018fb60bce406c862e7636fe1edd1b6ddd4f23ef70c9e97cea5d63cf4f4`. The quote copy migrated schema1/generation7 to schema3/generation8; both initialized and final payload SHA-256 are `c363fda868a20cf9ed557e0736de1e78cb247f98ab44a59a7e901cc8131eddee`. The harness asserted exact payload and generation equality after every sample. Original source profiles remain unchanged, including their original schema/generation and payload hashes in JSON.

Reproduction uses a new output directory:

```sh
source /workspace/sarv-environment/activate.sh
dbus-run-session -- /usr/bin/python3 scripts/accounting-performance.py \
  --binary /workspace/sarv-evidence/bin/sarv-M2-layout-final \
  --expect-sha 6c545a5642c53ac633a715f66f5f24cdfd107ccd58261724b688a89f1793a50d \
  --samples 10 --search-samples 20 \
  --output /workspace/sarv-evidence/performance-M2-layout-rerun
```

[PNG plot](../docs/assets/accounting-m2/performance/latency-samples.png), [SVG plot](../docs/assets/accounting-m2/performance/latency-samples.svg) and the [rerunnable plot script](../docs/assets/accounting-m2/performance/render-plots.py) in the retained bundle plot every retained repeated sample and the relevant proposed budget without smoothing or pooled data. They were visually inspected. The actual GTK screenshots `g1-small/startup-0.png`, `quote-scale/startup-0.png` and `quote-scale/picker-search.png` were also inspected: Persian labels and focus are visible, the quote's first rows and exact amounts render, and the expected tail product's Latin code/source timestamp remain legible in the Persian picker. The startup screenshot can still show row preparation in progress; this observer proves mapped editor controls, native focus response and the captured screen, not completion of every deferred row widget or full home-module readiness.

All 22 final application process logs contain zero WARNING/CRITICAL diagnostics. The previously reproduced requested1396/available1390 Adwaita width warning is absent after restoring the sidebar minimum width from250 to240 pixels. External AT-SPI observer cache warnings occurred as already-terminated owned samples disappeared, and Xvfb emitted its expected cleanup connection-close message; these are separate from product diagnostics.

## Complete native workflow: remaining strict misses

The independent reviewer reran `scripts/native-performance-M2.py` against the same immutable final layout executable. Evidence: [final independent performance](../docs/assets/accounting-m2/review/scale-final/performance.json), its [native report](../docs/assets/accounting-m2/review/scale-final/synthetic-profile/native-responsiveness-verification.json) and [raw state check](../docs/assets/accounting-m2/review/scale-final/raw-state-check.json). All 15 functional checks passed, including actual 50,000-row import, every catalogue/editor page, real keyboard movement across reused global row indices, delete/Undo, stale worker/chunk input preservation, locked asynchronous save/restart and PDF workflows. Independent persisted arithmetic reproduces 1,000 lines, 50,000 products, last-row quantity3, exact 100,802,501 IRR and SQLite integrity `ok`.

| Observed boundary | Final layout6c545a, ms | Proposed budget, ms | Status |
|---|---:|---:|---|
| Maximum 20-ms heartbeat gap across complete probe | 535.536 | 50 | Miss |
| Worst unchanged editor page readiness, within one workflow | 184.400 | 150 | Miss |
| Maximum heartbeat gap while PDF worker busy | 118.792 | 50 | Miss |
| 1,000-line PDF request to completion | 2,334.477 | No declared PDF budget | Completed, n=1 |
| 50k catalogue /1k-line save dispatch on GTK main thread | 5.360 | No durable-commit equivalence | Dispatch only, n=1 |

The whole-probe timer includes observer screenshots and scheduling. Traced screenshot capture reached161.941 ms in this final flow, and 100-row widget reuse reached77.205 ms in the preceding adaptive-tab flow; instrumentation boundaries differ from a continuous CPU profile. A probe's functional PDF heartbeat threshold is250 ms, so passing it does not accept PERF-01's50-ms requirement. Save dispatch is not durable-save completion. Page-click readiness covers varied pages in one run, not independently repeated navigation p95. The overall responsiveness/navigation budgets remain unaccepted.

The fresh M1 baseline `/workspace/sarv-evidence/native-M2-before-fresh/performance.json` on SHA65bedf observed global880.529 ms, worst unchanged page280.146 ms and PDF-busy124.527 ms. Workload and implementation evolved in M2; these single runs are retained observations rather than a controlled causal speed comparison. The historical M1 repeated observer reported G1 startup p95885.615 ms, scale startup p951,428.802 ms and exact-code lookup p95193.580 ms. The final corresponding observations above are locally lower, while scale startup RSS increased from the historical146.50-MiB p95 to167.31 MiB. Scheduling/cache state is uncontrolled, so this does not establish superior speed or statistical improvement.

Earlier M2 measurements remain historical: SHA86e32433 in `/workspace/sarv-evidence/performance-M2-final/` had G1/scale startup p95833.386/1,186.432 ms and exact-code p95187.278 ms, and reproduced the six-pixel allocation warning. That report and its plot retain their own original provenance. Reviewer pilot5 full-flow292,836/322,932-KiB RSS/HWM and adaptive-tab278,788/302,516-KiB RSS/HWM likewise remain prior-build observations, not final-layout memory. Functional and repeated evidence are kept distinct; PERF-01/PERF-02 remain partial.
