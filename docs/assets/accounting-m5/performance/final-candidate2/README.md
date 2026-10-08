# M5 candidate2 final performance observations

Measured once on2026-10-08UTC. Functional observations complete; **performance acceptance is partial**. Startup/initial RSS, small posted-fixture lookups and accepted posting timer phases meet their proposed limits. The120-draft lookup,1,000-line Sales navigation and full-quotation global/PDF/persistence heartbeat limits remain missed. No budget, sample selection or semantic readiness endpoint was relaxed. Earlier diagnostic failures and the candidate1 native pager blocker remain separately retained.

## Attribution and executor

- Immutable release `/workspace/sarv-evidence/bin/sarv-M5-candidate2`, SHA256 `9b32342c5dd966fb0f1a3f43f55529990221090e7a27d7aa7ddf7b96b4bf9537`.
- Root production aggregate `30dc57763c904f740a42df7f863276b2c2f08e2c4c8510901603d6b6571fda5b`; sorted UTF-8 relative-path bytes, path/NUL/raw bytes/NUL. Per-file manifests and before/after source, observer and executable checks match. The historical pathlib-component digest is retained separately.
- Root declared all owned GUI/X11/D-Bus processes stopped and headless gates/review complete; no builds, production/observer edits, GUI checks or heavy filesystem work during measurement. Existing user/older desktops remained untouched and uncontrolled. This is not an exclusive physical host.
- One final n10/n20 repeated suite, followed sequentially by one unchanged `scripts/native-performance-M2.py` invocation on the same binary. This is the first final measured M5 epoch; n1/n2 diagnostic qualifications do not replace it. No final epoch was repeated to waive a miss.
- Actual executor: five visible AMD EPYC9V74 KVM vCPUs, four CPU equivalents (`cpu.max=400000 100000`),16GiB cgroup RAM, Debian13.6, overlay filesystem, Rust1.99.0, GTK4.22.5/libadwaita1.9.4. Private X11/Cairo/Xvfb/Openbox1450×1050, scale1. SQLite WAL/FULL/3s busy timeout; clones use consistent SQLite backup.
- Before/after load averages0.43/0.83/0.90 and0.59/0.70/0.84, and cumulative cgroup CPU stats are raw. Shared scheduling and caches are uncontrolled; snapshots do not isolate application CPU. Reference four-core/8GB/SSD hardware is unverified.

`performance.json`, `scale-performance.json`, `native-scale-verification.json`, `source-provenance.json` and `root-freeze-manifest.json` retain exact data. `repeated/` and `scale/` retain all native phase records, stage/seed manifests, logs and screenshots; raw embedded paths still identify their original immutable ignored profiles. `analysis.json` has strict checks. `application-warning-check.json` finds zero application GTK/Adwaita/GLib/Gdk/Gsk warning/critical lines; original observer AT-SPI process-shutdown cache notices remain in separate logs. Databases, executable and runtime caches are excluded. The deterministic `../dataset/` inputs and native generator remain prerequisites.

## Startup and memory

Launch boundary: process to owned mapped window, visible usable quotation-editor Save, real Tab focus change and completed PNG. Pre-PNG interactive time and capture latency remain separate raw fields. First on a fresh SQLite backup may initialize/migrate. Repeated runs are new processes on an initialized backup with warm/uncontrolled OS pages. Hashing and copying warm pages; no OS cache flush or genuine physical-cache-cold claim. Initial readiness does not mean every deferred workspace/review row has rendered.

| Fixture | Fresh-backup first ms(n1) | Repeated p50 ms | p95/p99/worst ms(n10) | Steady RSS p95 KiB | VmHWM p95 KiB |
|---|---:|---:|---:|---:|---:|
| Core posted |834.705|829.147|891.731|96356|101344|
| Quote50k products/1k lines |1469.511|1038.808|1127.879|176396|176396|
| Procurement posted |833.789|833.039|908.370|99072|104040|
| Sales posted |894.868|881.242|962.398|101152|105096|
| Sales120 raw drafts |819.483|862.122|937.626|99856|103816|
| Sales1k-line raw draft |887.054|841.380|945.128|100260|105128|

All repeated startup p95 values meet3,000ms; all initial steady RSS p95 values meet250,000,000bytes. Application-only `/proc` RSS is sampled every10ms and after500ms idle; helpers are excluded. Later workflow RSS/HWM points: core132040/132040KiB; procurement141928/142624; full Sales145464/145464; incomplete-draft Sales133776/133776;120-draft search110996/119224;1k-line editor130448/130448; full quotation256760/306028. Those later points neither receive the initial-memory budget nor establish leak acceptance.

## Real-key lookups

Each stream has20 alternating real Ctrl+A/ASCII queries. Timer includes dispatch, debounce, layout,5ms AT-SPI polling and result focus; discovery, current-workspace Ctrl+F reveal and initial exact Search focus occur before timing. Results require the current exact ID/reference and count, sensitive/showing/visible states, positive rectangle, then actual focus on the correct action. This is an accessibility readiness endpoint, not an independently observed painted-frame or pure UI/CPU latency. Exact payload/generation equality holds after every read-only query. Core/procurement retain their historical semantic endpoints; Sales draft and posted streams are distinct datasets.

| Stream / endpoint | n | p50 ms | p95 ms | p99/worst ms |200ms p95|
|---|---:|---:|---:|---:|---|
| Core visible Edit |20|84.740|92.744|95.437|PASS|
| Core focused Edit |20|113.726|123.581|124.508|PASS|
| Procurement visible Detail |20|101.862|119.333|125.187|PASS|
| Procurement focused Detail |20|115.481|132.405|142.625|PASS|
| Posted Sales visible Detail |20|115.478|125.686|128.504|PASS|
| Posted Sales focused Detail |20|130.595|139.172|148.096|PASS|
|120 raw Sales drafts visible Continue |20|268.270|331.616|343.347|**MISS**|
|120 raw Sales drafts focused Continue |20|280.688|345.278|355.353|**MISS**|

Read-only party/item/mixed procurement filters have p9587.338/84.797/83.634ms, respectively. Their four-party/one-item rows are not selectable picker-capacity proof. The paged Sales fixture contains60 exact matches per alternating group,25 rows/page; it is raw drafts, not120 posted invoices or a large ledger. Strict misses stay in the raw data; AT-SPI traversal, GTK work and scheduling are possible contributors, not isolated causes or accepted exclusions.

## Sales paging, raw entry and exact financial proof

All three filtered pages are reachable: B001, B051 and B101. Next-to-current visible usable Continue takes140.164/102.164ms(n2 distinct page transitions), both below150ms. Root's separate native QA proves pure Tab/Space Next/Previous at1280/650; this performance observer uses a native component focus request with real Tab fallback and exact focus validation.

The stopped fresh synthetic long profile contains1,000 unique raw lines, net20,000IRR, zero financial effects. First open to marker plus visible usable Edit is822.080ms and to focused usable Edit1045.773ms(n1). All40 native25-row pages are reached. The following39 real Next interactions preserve exact state/generation; the exact scoped current-page marker and visible/sensitive Edit reject dispatch-only readiness. Focus may scroll the row into view and is separately timed.

| Long-document endpoint | n | p50 ms | p95 ms | p99/worst ms |150ms p95|
|---|---:|---:|---:|---:|---|
| Current page / visible usable Edit |39|326.473|419.622|501.681|**MISS**|
| Focused usable Edit |39|578.957|690.369|766.204|**MISS**|

These39 page interactions occur in one process and are descriptive navigation samples, not39 independent cold launches. The slow n1 opening is retained without a population-tail claim. Accessibility traversal/layout/scroll/focus all remain inside their declared endpoints. No independent busy heartbeat was added for read-only Sales searches/pages; missing phase records mean main-thread continuous-block acceptance there is unmeasured.

External staged Sales n1 boundaries: invalid raw Save13.581ms to durable revision,42.113ms to usable Save callback; corrected Save32.183ms durable; preview52.233ms; order19.837ms durable and164.826ms to actual editor closure and usable underlying Search. Real invalid input survives restart; clean Escape preserves durable state. Preview is read-only; exactly one199IRR nonfinancial order consumes its exact saved revision without a journal or stock effect. These are neither a20-line invoice benchmark nor repeated posting tails.

Independent literal full-state proof preserves immutable core/procurement and prior journals:6 Sales records(2orders,2fulfillments,2returns),9 total journals,4 Sales stock movements. Direct order199; paired nets100/99 and original costs58/58; returns40/23 then60/35; authentic retained zero-tax quotation-linked order20. Final inventory422IRR, AR299, AP329, GRNI50, Sales199, returns100, COGS58, profit41, trial1630/1630; W1=25pieces/292IRR. The actual M4 stock cutoff isMarch27; financial movements areMarch28/29/30/31 and final report cutoffMarch31. Separate native QA proves intermediate balances and broader keyboard workflows; this report does not pool their timings.

Core n1 preview61.467ms and durable opening post6.228ms retain1251/1251IRR. Accepted16ms timer phases: core2(max40.976ms), procurement22(max40.405), external Sales4(max45.921), full Sales14(max45.127), incomplete-draft Sales2(max20.553). Every accepted phase has ticks and meets50ms. The intentionally rejected74-versus75 receipt preview25.613ms is separately retained and excluded from accepted-command acceptance. Each phase/gap/result stays raw; no pooled percentile. Timers include worker wait/UI/probe/scheduling and are responsiveness proxies, not continuous CPU traces.

## Complete quotation scale: strict misses and hypotheses

Unchanged external M2 ran once after the repeated suite; all15 functional checks pass. Actual50,000-product import and500 preview pages,1,000 canonical quote lines, exact100,802,501IRR, explicit Save/retry/lock/newer-invalid-input recovery, current focus, generation-preserving navigation and busy keyboard/settings/PDF all complete. The older250ms functional PDF guard never replaces the strict50ms performance limit.

| Strict endpoint | Historical M4 freeze6 ms | Candidate2 ms | Budget ms | Candidate2 result |
|---|---:|---:|---:|---|
| Global heartbeat including setup/paging/screenshots |280.552|320.117|50|**MISS**|
| Maximum17 unchanged quotation page clicks |161.263|138.524|150|PASS in this n1 workflow; historical MISS retained|
| PDF-busy heartbeat |127.266|117.241|50|**MISS**|
| Broad persistence-busy heartbeat |98.725|100.135|50|**MISS**, includes picker preparation/selection|

Current quotation page success does not remove M4's original miss or establish universal faster navigation. M4 freeze3 resource-post50.761ms and focused procurement214.167ms misses also remain in their original immutable evidence. No causal speed claim follows from different source/checkpoints or uncontrolled caches/scheduling.

Raw traced quotation100-row reuse spans reach60.158ms(16of21 observed spans exceed50ms). That is a source-named wall-clock observation on the GTK thread, not isolated CPU or proof of which work caused the largest heartbeat gap. Explicit Save dispatch5.683ms differs from edited-page async navigation358.538ms and later durable/callback endpoints.

The320.117ms largest gap is labelled stage60 after preceding stage4 captured catalogue-last101.032ms and directly persisted/filled/refreshed the synthetic1k-line fixture185.841ms. Stage126303.954ms follows warm-picker capture98.294ms plus direct unknown-line fixture/refresh196.628ms. Stage labels advance after transitions; gaps cannot be assigned solely to the labelled command. Those synchronous observer fixtures/screenshots are material confounds; the strict global limit still fails.

PDF completion1756.469ms; its117.241ms busy gap is labelled stage9 after Settings screenshot107.263ms and workspace/size changes. Pure PDF/layout cost is not isolated. Broad persistence100.135ms is labelled stage131 on the picker selection path, so it cannot isolate Ctrl+S blocking. Actual choice dispatch8.090ms versus raw `50000_unknown_line_choice_durable_and_native_review_ready`543.133ms proves canonical durable adoption/callback completion; final review rows are **not independently awaited** after asynchronous refresh dispatch. Warm picker11.549ms dispatch versus35.253ms30-row existence likewise has no added paint/focus claim. Composer15.310ms dispatch versus217.835ms actual100-row readiness is distinct. All original raw key names and traces stay unchanged.

## Reproduce and remaining scope

Source the environment, keep the root manifest/immutable executable and synthetic quoted seed, declare other owners quiet, and use fresh ignored outputs. The exact final commands were:

```sh
source /workspace/sarv-environment/activate.sh
dbus-run-session -- /usr/bin/python3 scripts/native-performance-M5.py \
  --binary /workspace/sarv-evidence/bin/sarv-M5-candidate2 \
  --expect-sha 9b32342c5dd966fb0f1a3f43f55529990221090e7a27d7aa7ddf7b96b4bf9537 \
  --source-manifest docs/assets/accounting-m5/checks/candidate2/manifest.json \
  --samples 10 --search-samples 20 \
  --output artifacts/performance-m5/candidate2-final-repeated
dbus-run-session -- /usr/bin/python3 scripts/native-performance-M2.py \
  --binary /workspace/sarv-evidence/bin/sarv-M5-candidate2 \
  --expect-sha 9b32342c5dd966fb0f1a3f43f55529990221090e7a27d7aa7ddf7b96b4bf9537 \
  --output artifacts/performance-m5/candidate2-final-scale
```

The sequential wrapper separately checked ordered production/observer provenance immediately before/after M2; unchanged M2 itself checks its executable. New runs must use new output paths, and are not authorized as selective reruns to erase this epoch's misses. `render-plots.py` creates standalone SVG/PNG from retained samples after measurement. No application is launched by plotting.

Nearest-rank p99 equals worst at n10/n20/n39; n39 p95 is second worst. n1 phase/workflow/memory points are not population tails. Unmeasured: reference4core/8GB/physicalSSD, genuine OS-cache-cold, five years/100,000documents/1,000,000journal lines/10,000parties, typical20-line invoice p95, ledger/annual-report/import budgets, read-only Sales continuous-block tracing, hardware power loss/disk-full, Windows/Wayland, physical print, screen readers, packaging, long-duration leaks and representative user studies. Raw drafts do not certify posted-ledger capacity. Payments, advances, service sales, damage, independent unbilled delivery, tax/statutory actions remain outside sealed M5 scope. No Rust-speed, competitor-parity, production-readiness or statutory claim follows.
