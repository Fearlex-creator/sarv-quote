from pathlib import Path
import json,hashlib
d=Path(__file__).resolve().parent;r=d.parents[4]
a=json.loads((d/'analysis.json').read_text());j=json.loads((d/'performance.json').read_text());q=json.loads((d/'native-scale-verification.json').read_text());p=json.loads((d/'source-provenance.json').read_text());w=json.loads((d/'application-warning-check.json').read_text());s=j['sales_measurement'];h=j['hardware']
f=lambda n:f'{n:.3f}'
rows=[]
for name,v in a['startup'].items():
 rows.append(f"| {name} | {f(v['first_process_ms'])} | {f(v['startup']['p50'])} | {f(v['startup']['p95'])} | {f(v['startup']['p99'])} | {f(v['startup']['worst'])} | {v['rss_kib']['p95']} | {v['hwm_kib']['p95']} |")
lookup=[]
for name,v in a['search'].items():
 lookup.append(f"| {name} visible / focused |20| {f(v['visible']['p50'])} / {f(v['focused']['p50'])} | {f(v['visible']['p95'])} / {f(v['focused']['p95'])} | {f(v['visible']['p99'])} / {f(v['focused']['p99'])} |200| {'PASS' if v['visible_budget']['passes'] and v['focused_budget']['passes'] else '**MISS**'} |")
for name in ('visible','focused'):
 v=a['long_navigation'][name];lookup.append(f"| 1,000-line current page {name} Edit |39|{f(v['p50'])}|{f(v['p95'])}|{f(v['p99'])}|150|{'PASS' if a['long_navigation'][name+'_budget']['passes'] else '**MISS**'}|")
action_rows=[]
for row in a['additive_action_scopes']:
 action_rows.append(f"|{row['id']}|{row['operation']}|{row['ticks']}|{f(row['max_gap_ms'])}|{f(row['terminal_gap_ms'])}|{row['strict_status'].upper()}|")
broad=[]
for name,v in a['legacy_broad_strict_budgets'].items():broad.append(f"|{name}|{f(v['actual_ms'])}|{v['budget_ms']}|{'PASS' if v['passes'] else '**MISS**'}|")
phase=[]
for name,v in a['native_timer_phases'].items():
 details='; '.join(f"{x.get('operation',x.get('phase'))} {f(x['actual_ms'])}ms" for x in v['accepted_misses']) or 'none'
 phase.append(f"|{name}|{v['measured_accepted']}|{details}|{len(v['excluded_or_unmeasured'])}|")
source_max=max((span['ms'] for spans in a['source_trace_spans_correlated_with_active_scopes'].values() for span in spans),default=None)
readme=f'''# M5 candidate12 final performance observations

One quiet C12 n10/n20 epoch and one sequential unchanged M2 invocation completed on {p['utc'][:10]} UTC. Functional observations and all source/executable/observer guards pass. **Performance acceptance is PARTIAL; the strict gate fails and M5 performance remains open.** Every sample, warning and strict miss is retained. No final epoch was repeated.

[Raw M5 report](performance.json), [raw M2 report](scale-performance.json), [native scale report](native-scale-verification.json), [strict recomputation](analysis.json), [sequential provenance](source-provenance.json), [Sample plot](latency-samples.svg) and [file hashes/bytes](evidence-files.json) make the result reviewable. [Analysis generator](analyze.py) recomputes nearest-rank p50/p95/p99/worst, all terminal-included scoped maxima, raw command phase arrays, membership receipts and source-span correlations; it launches no application. `repeated/` and `scale/` retain all available native manifests, traces, logs and screenshots. Complete original ignored profiles remain under `artifacts/performance-m5/final-candidate12/` and `final-candidate12-scale/`; portable copies exclude executables, SQLite/runtime caches and large prerequisite CSVs.

## Frozen attribution and observation methods

Release `{j['binary']}`, SHA256 `{j['binary_sha256']}`; production aggregate `{a['production_source_sha256']}`. The [root freeze](root-freeze-manifest.json) records all 36 source hashes in UTF-8 relative-path byte order, path/NUL/raw bytes/NUL. The [qualified capture](qualified-verification-before.json) records all 53 verification files. Before/after every suite and the whole sequential epoch, all 36 source files/aggregate, binary,53 verification files, manifests and the raw-byte coordinator hash agree. M5 observer SHA256 `{a['external_observer_sha256']}`; unchanged M2 observer SHA256 `{p['before']['observers']['native-performance-M2.py']}`. [Raw coordinator](sequential-provenance-driver.py) preserves exact commands and guards.

Root issued quiet GO after frozen correctness/native/identity/preserved gates completed. Owned GUI/build/test/render work stopped; no source or observer edits/builds/extra GUI checks/heavy curation ran in the measurement window. Older user desktops remained untouched and uncontrolled. Small status reads accompanied progress updates. `SARV_SALES_FOCUS_QA` and `SARV_SALES_FOCUS_DIAGNOSTIC` were absent; no diagnostic on-disk focus receipts ran in benchmark scopes. Only synthetic profiles were used. The release embeds unmodified Vazirmatn v33.003 under SIL OFL1.1 and registers it per process; the native Fontconfig configuration includes local system fonts. [Font provenance and license](../../../../../assets/fonts/README.md) declares the original file hashes.

C12 production corrections since immutable [C8 final evidence](../final-candidate8/README.md) cover current Sales Edit pager focus; retained quotation parent through hide/show; manager numeral/direct-label scopes; split opt-in source traces; retained Sales detail pager/labels; narrow-height correction; background focus handoff before quotation workspace disable; root settings application once after initial/restore pages and direct customer/form/totals numeral formatting. Financial source remains unchanged since C5. External `m5-cached-v3` and compiled `m5-action-v1.2` predicates, limits and timing boundaries remain the fixed C8 methods. C9/C10/C11 qualification and diagnostics are separately attributed; no final C9/C10/C11 epoch ran. The [never-executed C9 preparation](preparation/candidate9-preparation-status.json) and [original raw driver](preparation/candidate9-unexecuted-driver.py) remain separately preserved. C8 remains immutable and failed. Changes do not establish identical controlled causal speed comparisons; Rust and fixture correctness establish neither superior speed, production readiness, competitor parity nor tax compliance.

Cached Sales discovery occurs outside query/page timers. Timed readiness refreshes owned PID, roles, defunct state and every current parent→child membership through the root parent, exact text/count/title/action/page marker, sensitivity/showing/visibility, positive allocation and requested actual focus. Zero-result readiness requires exact0/statusready/actioninsensitive and current title/action ancestry absent; retained stale leaf flags remain raw. Positive A/B nodes and ancestry reappear. Persistent slot mapping has separate native identity proof; no visible line UUID or painted-pixel claim is invented. No timed BFS fallback exists.

Independent 20 ms quotation scopes start before actual Save/page/choice dispatch or second internal-helper `State::preview`. Exact adopted generation, all 100 current row IDs/queries, first amount, literal total, pending flags, allocated/visible/sensitive required focus and two later ticks terminate readiness. Terminal gaps count. They exclude synthetic injection, screenshots and post-scope disk verification; they do not inspect every amount or prove painted pixels. v1.2 preserves Quantity-return focus after choice. Broad samplers and strict budgets remain unchanged; additive scopes cannot replace broad misses.

## Executor, datasets, startup and memory

Actual executor: five visible AMD EPYC9V74 KVM vCPUs; four CPU equivalents (`cpu.max={h['cgroup_cpu_max']}`);16GiB cgroup RAM (`memory.max={h['cgroup_memory_max']}`); Debian13.6; overlay filesystem; Rust1.99.0; GTK4.22.5/libadwaita1.9.4. Private Xvfb/Openbox1450×1050, X11/Cairo, scale1. SQLite WAL/FULL/3s busy timeout; consistent backup clones. Raw loadavg before `{h['loadavg_before']}` and after `{h['loadavg_after']}`, cumulative CPU accounting, OS/libraries and filesystem are retained in `performance.json`. Shared virtual scheduling is uncontrolled; this does not certify a dedicated four-core/8 GB/physical SSD reference host.

[Deterministic dataset plan](../dataset/dataset-plan.json) declares generator/seed and raw input hashes; the quotation fixture has50,000 actual imported products,1,000 canonical lines and exact100,802,501IRR. Six startup cohorts are core posted, procurement posted, quotation50,000 imported products/1,000 lines, Sales posted,120 raw Sales drafts and one 1,000-line raw Sales draft. Each has one initial stopped-backup launch and 10 new processes. Startup ends at mapped owned window, visible usable Save, actual Tab roundtrip and completed PNG. Pre-PNG and capture cost remain raw. Initial backup launch is process-cold with uncontrolled OS pages/possible migration; repeated launches use initialized backups with warm/uncontrolled OS pages. Hashing/cloning warms pages; no physical cold-cache/cache-flush claim. Application-only `/proc` RSS/HWM uses10ms sampling and500ms idle; helper memory is excluded.

|Fixture|First ms(n1)|p50 ms(n10)|p95 ms|p99 ms|Worst ms|Initial RSS p95 KiB|HWM p95 KiB|
|---|---:|---:|---:|---:|---:|---:|---:|
{chr(10).join(rows)}

Startup p95 budget remains3000ms; initial RSS p95 remains250,000,000bytes. Later workflow RSS/HWM is separately named in raw reports and does not inherit the initial-memory limit or establish leak acceptance. Later RSS/HWM observations(KiB,n1) are core135628/135628, procurement138884/139604, fullSales142332/142332, draft135176/135176, pagedsearch124984/124984, longeditor131424/131424 and fullquotation338848/384676. Search/long memory remains separately sampled rather than pooled with initial acceptance. All raw startup samples and memory points remain retained.

## Search and long-document endpoints

Four lookup streams each use 20 alternating actual ASCII queries. XTest dispatch/debounce/layout/5ms polling and result focus are inside endpoints; initial focus/discovery/Ctrl+F reveal are outside. Exact durable payload/generation equality holds after read-only actions. Raw120 drafts are60 PERF-A/60 PERF-B with25 rows per page and all three filtered pages reached, distinct from posted-ledger capacity. One1,000-line raw draft is net20,000IRR,40 pages of25 rows, with 39 sequential transitions. Exact data/state equality and markers/current actions remain unchanged.

|Current endpoint|n|p50 ms|p95 ms|p99/worst ms|Budget ms|Result|
|---|---:|---:|---:|---:|---:|---|
{chr(10).join(lookup)}

Initial long opening visible/focused={f(a['long_open_visible_ms'])}/{f(a['long_open_focused_ms'])}ms(n1). Loaded39 transitions occur within one process and are descriptive, not independent launches. Paired focused-after-visible deltas: p50={f(a['long_navigation']['focus_after_visible_delta']['p50'])},p95={f(a['long_navigation']['focus_after_visible_delta']['p95'])},worst={f(a['long_navigation']['focus_after_visible_delta']['worst'])}ms; no marginal-p95 subtraction. Component focus attempt, real Tab fallback, poll, scroll/layout and scheduling remain combined; no per-phase timings/Tab counts exist. Read-only Sales continuous heartbeat remains unmeasured.

## Native command timers and quotation action scopes

Every raw 16 ms command phase array, including terminal segments and intentionally rejected/zero-tick phases, remains available. Accepted phases require observed ticks; missing or zero-tick results cannot pass by implication. The following summarizes exact accepted misses; rejected/unmeasured entries remain separately in analysis.

|Stream|Measured accepted|Accepted strict50ms misses|Excluded/unmeasured|
|---|---:|---|---:|
{chr(10).join(phase)}

Exact M5 financial proof remains6 Sales records(2orders/2fulfillments/2returns),9 journals,4 typed stock movements, trial1630/1630IRR, inventory422,AR299,AP329,GRNI50,Sales199,returns100,COGS58,profit41,W1=25pieces/292IRR. Original-cost returns23/35 and credits40/60 remain exact; stock cutoffMarch27 and movementsMarch28–31/report cutoffMarch31 remain. Separate native QA owns broader keyboard/cancellation/identity proof. External invalid Save/restart/cancel/correction/preview/order succeeds; n1 workflow observations do not establish a representative20-line invoice tail benchmark.

All 15 unchanged M2 functional checks and 22 additive endpoints complete. Every raw gap and terminal segment remains in native report. {a['additive_page_miss_count']}of18 paging scopes and {a['additive_miss_count']}of22 total scopes miss 50 ms.

|Scope ID|Operation|Ticks|Max gap ms|Terminal gap ms|50ms result|
|---:|---|---:|---:|---:|---|
{chr(10).join(action_rows)}

The second internal-helper PDF scope is probe preview/background export, distinct from actual native file-picker PDF. Source traces correlate wall spans with active scopes; largest retained active source span={f(source_max) if source_max is not None else 'unmeasured'}ms. Paging miss scopes8/9/16 correlate `review_100_row_widget_reuse`49.089/48.773/49.478ms respectively; nested setter/chrome/focus spans remain separately raw. [Analysis](analysis.json) preserves every correlated name/value. Nested spans cannot be summed and are not isolated CPU. Tracing, native readiness and GTK/scheduling remain in observed work; no complete product/observer causal split is inferred.

## Broad strict gate and retained history

|Original broad endpoint|C12 ms|Budget ms|Result|
|---|---:|---:|---|
{chr(10).join(broad)}

Global includes setup/fixtures/paging/capture; persistence-busy includes picker and Save, rather than isolated Ctrl+S CPU. Synchronous fixture injection and screenshots contaminate broad probes; additive exclusions waive no broad miss. The250ms functional PDF guard is not a50ms performance limit. Strict performance remains open even when functional checks and individual scopes pass.

[C8 immutable final](../final-candidate8/README.md), [C2 immutable final](../final-candidate2/README.md) and [M4 strict evidence](../../../../../requirements/performance-M4.md) preserve all historic misses. C2 retains raw120draft p95331.616/345.278ms; long419.622/690.369ms; global320.116878/PDF117.240539/persistence100.135268ms. C2 unchanged quote page138.524176ms passed; M4 page161.263/global280.552/PDF127.266/persistence98.725ms misses remain. Earlier resource-post50.761/focused procurement214.167ms misses remain unchanged. Source/protocol/cache differences do not permit a causal speed claim or selective rerun.

[Application warning scan](application-warning-check.json) records {w['application_GTK_warning_critical_count']} application GTK warning/critical entries; all lines/logs remain retained. Shutdown AT-SPI cache notices stay separately in observer logs. Diagnostic warning reproduction and fixed-build native qualification remain separate; final scan claims only its own observed logs.

Unmeasured: dedicated reference4core/8GB/physicalSSD; genuine physical cache-cold; five-year100,000documents/millionjournal lines/10,000parties; representative20-line invoice/ledger/annual-report/import performance; read-only Sales heartbeat; Windows/Wayland; physical printing; power loss/disk-full; packaging; screen readers; long-duration leaks and representative user studies. Nearest-rank p99 at n10/n20 equals worst; n1 workflow/memory points are not tail estimates. Raw drafts do not establish posted capacity. Payments, advances, tax submission, service sales, damage and independent unbilled deliveries remain deferred. No production/competitor/tax acceptance follows.
'''
(d/'README.md').write_text(readme)
files={}
for path in sorted(d.rglob('*')):
 if path.is_file() and path.name!='evidence-files.json':
  b=path.read_bytes();files[path.relative_to(d).as_posix()]={'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
(d/'evidence-files.json').write_text(json.dumps({'algorithm':'SHA256 raw file bytes; inventory excludes itself','files':files},indent=2)+'\n')
print('report/inventory written',len(files))
