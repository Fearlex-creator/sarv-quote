# Independent review — M2 calendar and native navigation

Reviewer has no implementation ownership. Baseline: `d640288d90d9f21789cd213ecbaa85e16c69bcb5`.
Both normative requirements and execution brief were read completely before review.
All probes use isolated synthetic profiles; the reviewer owns this report and
ignored scratch artifacts only. Final schema3 library/Store, native/editor and
adaptive accounting-tab and final layout reviews are complete. Final reviewed binary SHA-256:
`6c545a5642c53ac633a715f66f5f24cdfd107ccd58261724b688a89f1793a50d`.
Accepted proof is retained under [the portable review bundle](../docs/assets/accounting-m2/review/);
older scratch paths below are explicitly preliminary evidence.

## Independent expectations established before implementation review

IR-03, SET-01/02, ACC-02/04/05, MIG-01 and DATA-02/04 require calendar presentation
to leave the canonical financial day, accounting period, issued snapshots and
historical identity unchanged. The bounded M2 calendar range is Jalali years
1200 through 1499 inclusive. This is engineering calendar/navigation evidence,
not accountant approval, full G1, statutory capability or production acceptance.

The independent known-day expectations are:

| Jalali day | Canonical ISO day | Expected condition |
|---|---|---|
| 1403-01-01 | 2024-03-20 | Nowruz |
| 1403-12-29 | 2025-03-19 | Valid |
| 1403-12-30 | 2025-03-20 | Valid leap Esfand day |
| 1404-01-01 | 2025-03-21 | Nowruz, immediately follows prior row |
| 1404-12-29 | 2026-03-20 | Last valid day of common year |
| 1405-01-01 | 2026-03-21 | Nowruz |
| 1405-12-29 | 2027-03-20 | Last valid day of common year |
| 1406-01-01 | 2027-03-21 | Nowruz |

Strict rejection fixtures: month 0/13, day 0, first-half day 32, second-half day
31, common-year Esfand 30, invalid ISO Gregorian leap days, unsupported Jalali
1199/1500 and malformed/trailing date text. Persian/Arabic/Latin digits must
represent identical days where accepted. Epoch/day arithmetic must never infer
the input calendar from its year or silently normalize impossible dates.

Independent probes will check every valid supported day for inverse conversion,
ISO adjacency at month/year/leap transitions, fiscal start/end behavior including
1499, and separately compare known dates to an external calendar implementation.
Round trips alone cannot prove the calendar's external correctness.

Persisted pre-M2 ISO quotations, revision snapshots, fiscal periods and journal
dates must survive default settings, date-display toggles, settings save/restart,
CSV/PDF export and fresh-profile restore with identical financial dates, IDs,
amounts, counts, retry identity and history. Human-readable exports must disclose
calendar and canonical dates; machine-readable dates must remain unambiguous.

Native review will exercise explicit calendar selection, retain invalid input
on mode switches, correct it without retyping valid fields, and verify that
posting uses the intended canonical day and period. Real keyboard navigation,
Escape recovery, visible focus, standard/narrow native allocations, light/dark
appearance and mixed Persian/Latin codes will be reviewed independently.
Screenshots establish rendering only; raw persisted journal lines and dates
provide separate financial proof. Existing quote save/history and worker behavior
remain regression requirements. Unsupported screen-reader, native-reader,
physical printing, Windows/Wayland and hardware power-loss conditions will be
declared untested.

## Research and rerun results

External oracle: Node 24.19.0 `Intl.DateTimeFormat('en-US-u-ca-persian')`, ICU
78.3 / CLDR 48, UTC at noon. The reviewer generated every Gregorian day from
1821-03-21 inclusive through 2121-03-21 exclusive before testing implementation
output: 109,573 independently expected rows. ICU confirms every known day above,
including 1403 leap Esfand30 and common years 1404/1405. Oracle and runtime
metadata: [known days](../docs/assets/accounting-m2/review/icu-known-days.json)
and [all expected days](../docs/assets/accounting-m2/review/icu-all-days.csv).

The jalaali-js project documentation states that its Borkowski calendar agrees
with Intl within Gregorian 1800–2256 and diverges after 2256; this module's bound
falls within that comparison interval. Documentation fetched independently from
`https://raw.githubusercontent.com/jalaali/jalaali-js/master/README.md` on
2026-10-07. The official University of Tehran calendar page
`https://calendar.ut.ac.ir/Fa/Calendar.html` returned a Cloudflare interstitial,
so the review does not claim current official-calendar document verification.

Preliminary evolving-source independent reviewer probe passed all 109,573 external
expected rows, both conversion directions, all 300 fiscal-year ranges, strict
invalids/digits and both range endpoints. It separately posted 1,251 IRR on
1403/12/29 and 349 IRR on leap 1403/12/30: canonical cutoffs give exactly 1,251
and 1,600 IRR. Posting 1404/01/01 under the 1403 fiscal period rejects with exact
pre/post state equality. A persisted approved ISO quotation and its CSV remain
byte-for-byte unchanged after selecting Jalali settings; periods, accounts,
journals and audit remain unchanged. A distinct retry key returns the original
journal ID, adds only the expected retry record and retains financial history.
Reopen and fresh-profile verified restore equal the complete saved aggregate.
Scratch source/result: `artifacts/independent-m2/probe/` and `probe-run-1.log`.
The final frozen-source reruns below supersede these preliminary results.

Expanded reviewer probes additionally pass an actual 1499/12/29 financial posting
on canonical 2121-03-20, rejecting the successor Gregorian day without mutation.
Revising the old quotation retains its ISO snapshot; a newly created quotation
inherits Jalali presentation and retains canonical quote/expiry dates
2025-03-20/2025-03-21. Independent CSV fields and rendered PDF text show the
correct 1403/12/30 and 1404/01/01. The independently rendered trial report labels
both 1403/12/30 and 2025-03-20 and displays 160 toman for 1,600 IRR. Raw SQLite
line sums are balanced 1,251, 349 and 1 IRR on 2025-03-19, 2025-03-20 and
2121-03-20 respectively, with integrity `ok`. Reviewer raw state, CSV/PDF and
render: `artifacts/independent-m2/run-3/`.

The actual retained M1 migration fixture `upgraded-v2.json` also passes a separate
reviewer restore/preferences/reopen probe: all three historical quote/revision
JSON snapshots and CSV bytes remain identical. Canonical 2026-10-07 dates and
2026-10-21 expiry remain unchanged. Scratch bin `sarv-independent-m2-legacy` and
`legacy-run-1.log` record this separate compatibility check.

Preliminary reviewer-authored native probe passed 22 checks on binary SHA-256
`ff9716e36d024ae64028698269bb3a28669b21f60393a8460332ca5f6d520742`:
actual light/dark 1280×900 and 650×760 GTK windows; keyboard opening of the narrow
sidebar and activation of its accounting destination; Tab/Enter journal path;
retention of invalid common-year 1404/12/30 when changing calendars; Persian
1403/12/30 converting exactly to 2025-03-20 and back; and Escape preserving the
complete raw aggregate. The reviewer separately inspected screenshots showing
readable Persian labels, mixed source IDs, both equivalent days and visible focus.
Report and native images: `artifacts/independent-m2/native-751qx26w/`.
Earlier scratch attempts failed on harness role/duplicate-node assumptions,
not on persisted financial outcomes. The adapted probe uses actual native roles
and real Tab focus. The final native rerun below supersedes this pilot.

## Final frozen-source reruns and migration review

Frozen schema3 source independently passes the external 109,573-day calendar
oracle and complete leap/cutoff/pref/revision/CSV/PDF/restart/fresh-restore probes.
Retained outputs: [calendar/workflow probe log](../docs/assets/accounting-m2/review/probe-final.log),
[legacy probe log](../docs/assets/accounting-m2/review/legacy-final.log),
[full backup](../docs/assets/accounting-m2/review/backup.json),
[old ISO CSV](../docs/assets/accounting-m2/review/old-iso.csv),
[new Jalali CSV](../docs/assets/accounting-m2/review/new-jalali.csv),
[new Jalali PDF](../docs/assets/accounting-m2/review/new-jalali.pdf) and
[leap trial PDF](../docs/assets/accounting-m2/review/leap-trial.pdf).
New backup format is `sarv-accounting-backup-v3` with schema3.
The actual retained M1 quotation fixture still preserves all three snapshots
and CSV bytes after Jalali settings and reopening under schema3.

The reviewer independently reran:

```sh
source /workspace/sarv-environment/activate.sh
cargo test --locked --offline --test calendar --test calendar_workflow \
  --test settings --test accounting_kernel --test accounting_migration \
  --test detached_save --test accounting_exports --test process_recovery
```

**48 passed, 0 failed, 0 ignored**. Raw log:
[targeted test log](../docs/assets/accounting-m2/review/final-targeted-tests.log). The independent standalone
probe is rerunnable using its retained Cargo manifest, lockfile and ICU fixture:

```sh
CARGO_TARGET_DIR=/workspace/sarv-quote/target cargo run --offline \
  --manifest-path docs/assets/accounting-m2/review/probe/Cargo.toml \
  --bin sarv-independent-m2-probe -- /tmp/sarv-m2-review-fresh \
  docs/assets/accounting-m2/review/icu-all-days.csv
```

Separate reviewer bin `sarv-independent-m2-migration` opened a manually seeded
actual pre-M2 v2 fixture at generation17. It upgrades to schema3/generation18,
retains all three quotations and three journals, every date/ID/amount/history
field, publishes exactly one verified v2 recovery matching the original state,
reopens idempotently at generation18 and restores into a fresh profile exactly.
Retained [migration probe source](../docs/assets/accounting-m2/review/probe/migration.rs),
[result log](../docs/assets/accounting-m2/review/migration-final.log) and
[verified pre-M2 v2 fixture](../docs/assets/accounting-m2/migration/verified-before-schema3-v2.json)
provide rerun inputs. This bin expects a manually seeded generation17 SQLite
profile containing that fixture state; it checks the migration itself rather
than a restore that already upgrades the state.

**Finding addressed: same-schema enum downgrade ambiguity.** During the earlier
schema2 pilot, actual M1 binary `65bedf48` rejected the new `Jalali` variant while
leaving its copied payload unchanged. Review requested explicit incompatibility
and verified pre-M2 recovery. Integration now versions the capability as schema3
and v3 backups, verifies/preserves legacy recovery before upgrading and rejects
Jalali values in schema1/2. [Migration guidance](../migrations.md) explicitly
rejects automatic M2→M1 downgrade and warns that current ISO settings cannot
remove issued Jalali snapshots. The original unknown-variant probe is historical
pre-version evidence, not a defect in the final migration.

The reviewer actually ran M1 binary SHA-256
`65bedf489894bd6435b57a9919fadf0a31e2e1d9809f97089212dbf43bfc4378`
with the verified pre-M2 v2 recovery into a different fresh native profile:
complete recovered state equals the original **three quotations and three
journals**. A separate actual M1 run against schema3 books containing only ISO
date variants fails with unsupported-version, leaving generation18 and payload
exactly unchanged. Reports: [native financial/recovery report](../docs/assets/accounting-m2/review/native-financial/review.json)
and [ISO-only schema3 rejection](../docs/assets/accounting-m2/review/m1-reject-schema3-iso.json).

## Final native review and performance delta

Reviewer-authored native probe independently passed **24 checks** on binary
`8522389842bab5492af1578d3c8da6ced0161b535fdca9ec0dacbb04caa897c6`.
This includes the actual M1 recovery/rejection checks above plus light/dark,
standard/narrow, keyboard sidebar/journal navigation, invalid-date retention,
Persian leap conversion, visible focus and Escape/state preservation described
above. [Retained native financial report and screenshots](../docs/assets/accounting-m2/review/native-financial/review.json).
No screenshot is substituted for financial proof: independent canonical date,
raw line-sum, exact state and export checks are separate.

The pre-tab-fix binary differs from the frozen source recorded for that native
run only in `src/ui/editor.rs`. That delta packages unchanged chunk-start/end arguments as a Rust
`Range<usize>` and removes needless grid references; the same exclusive
start/end iteration remains. Financial/calendar/sidebar/persistence/export
component hashes remain identical. The preceding independent editor-scale rerun passed
against immutable binary
`460868dc58e28d3981e9be8f446f7a1f385640b7ab5c0492ede15a266c7b8e81`.

Source review separately examined disposable review/product-index caching keyed
by Store generation and the next source-time/seven-day validity boundary, the
single worker and edit-revision guards, and detached 20-row construction guarded
by generation, edit revision and requested page. Resolved-row reuse updates live
indices consumed by remove and Ctrl+Up callbacks; unresolved rows rebuild.
The final real native probes below independently exercise stale input, reused
indices and exact persisted outcomes. Source inspection is not a substitute for
those workflow checks or for hardware-loss testing.

The reviewer already independently reran the preceding immutable pilot5 scale
binary `210026e2` in a fresh profile: all **15** workflow checks passed, including
real XTest Ctrl+Up from reused row1000 to global row999, reused-row deletion and
Undo preserving exact order/identity, newer invalid input across both review
worker and aborted 20-row construction, save/restart/close and PDF workflows.
Independent raw-state arithmetic reproduces **100,802,501 IRR**, 1,000 lines,
50,000 products, last-row quantity3 and integrity `ok`. Raw report:
`artifacts/independent-m2/scale-pilot5/`.

That separate single run reproduces strict misses: global heartbeat **523.941
ms** (50-ms budget), unchanged-page readiness **156.807 ms** (150-ms budget),
PDF-busy heartbeat **111.586 ms** (50-ms budget), final RSS **292,836 KiB** and
HWM **322,932 KiB**. Timer gaps include observer/scheduling/screenshot work and
are a responsiveness proxy, not a continuous CPU trace or a p95 claim. The final delta results follow; the overall PERF gate is not accepted.

The final layout schema3 binary `6c545a56` independently passes all **15**
scale workflow checks again, including the exact same real keyboard, delete/Undo, stale worker/chunk,
save/restart/close and PDF protections. Independent final raw-state arithmetic
confirms schema3, **100,802,501 IRR**, 1,000 lines, 50,000 products, last-row
quantity3 and integrity `ok`. Real final GTK editor/picker/narrow screens were
inspected, including page10 with mixed Latin product codes and visible focus.

Final single-run strict results remain misses: global heartbeat **535.536369 ms**,
unchanged-page readiness **184.400279 ms**, PDF-busy heartbeat **118.791776 ms**;
RSS **282,864 KiB**, HWM **311,204 KiB** after the full workflow. These exceed
the proposed 50/150/50-ms and 250-MiB budgets at the declared boundaries. This
n=1 final run overlapped independent tab review and integrator validation on the
executor; it is not an isolated reference-hardware measurement or a p95 claim.
Observer/screenshot and scheduling work are included in the timer boundaries.
The preceding `86e32433` independent run also missed all three strict budgets
(555.755732/182.220889/117.020152 ms); its retained proof is historical.
The earlier `460868dc` run likewise missed them (558.786/159.520/110.621 ms).

Retained [final performance report](../docs/assets/accounting-m2/review/scale-final/performance.json),
[independently computed raw totals](../docs/assets/accounting-m2/review/scale-final/raw-state-check.json)
and [15 native functional checks](../docs/assets/accounting-m2/review/scale-final/synthetic-profile/native-responsiveness-verification.json)
record the final run. The functional pass does not accept the PERF gate.

Final immutable-bin scale invocation (supply an available immutable binary with
this exact SHA; the executor binary path is not part of the portable bundle):

```sh
dbus-run-session -- /usr/bin/python3 scripts/native-performance-M2.py \
  --binary /workspace/sarv-evidence/bin/sarv-M2-layout-final \
  --expect-sha 6c545a5642c53ac633a715f66f5f24cdfd107ccd58261724b688a89f1793a50d \
  --output /tmp/sarv-m2-review-scale-fresh
```

[Final reviewed source hashes and component-run attribution](../docs/assets/accounting-m2/review/final-reviewed-source.json)
record the final binary and the unchanged financial source. The integrator owns
the final source commit and retained checkpoint bundle.

### Narrow accounting-tab follow-up

Root visual inspection found the fifth accounting-tab caption clipped at650px
in the horizontal tab strip. Integration replaces only that strip with a native
wrapping FlowBox tab list and five role-Tab toggle buttons; the Stack page IDs,
direct page structure and all financial callbacks remain unchanged. Review
inspected the weak callbacks, selected-state synchronization and repeated current
tab activation. Hash comparison against the preceding `460868dc` source confirms
that only `src/ui/accounting.rs` changed.

The preceding immutable `86e32433` binary independently passes **64** native checks
in Light/Dark at1280×900 and650×760. Real XTest Tab/Space reaches and activates
all five tabs; activating a current tab again retains exactly one selected tab.
The complete raw financial aggregate remains equal in every view. All five full
Persian captions and visible focus were independently inspected in actual
screenshots. AT-SPI returns zero x/y origins for these custom-role tabs, so its
positive allocation dimensions and zero-origin bounds assertions are not used
as physical screen-fit proof; actual rendering and keyboard outcomes provide
that evidence. No complete screen-reader certification is claimed.

Retained [64-check report](../docs/assets/accounting-m2/review/native-tabs/review.json),
[650px Dark screenshot](../docs/assets/accounting-m2/review/native-tabs/Dark-all-five-tabs-650.png)
and [1280px Light screenshot](../docs/assets/accounting-m2/review/native-tabs/Light-all-five-tabs-1280.png)
record the correction. Accounting-tab source and all financial/calendar/editor
components remain byte-identical in the final layout binary; the full final scale
rerun above covers its unchanged editor worker/reuse protections.

### Final sidebar layout follow-up

Repeated scale/startup logs reproduced an Adwaita allocation warning on86e:
requested1396px versus1390px available. The final `6c545a56` correction changes
only `src/ui.rs` sidebar minimum width from250px to240px. Reversing that one
literal reproduces the previous reviewed file hash; every other reviewed
production source hash remains identical. Prior native financial and64-tab
proofs retain their own recorded binary provenance rather than being relabeled.

The reviewer reran all15 scale workflow checks on the final immutable binary.
Startup and the complete50,000-product/1,000-line workflow emit zero matching
width overflow warnings in170 native log/trace lines. Independent raw financial
arithmetic and integrity remain exact as reported above. Retained
[layout warning check](../docs/assets/accounting-m2/review/scale-final/layout-warning-check.json)
and [native log](../docs/assets/accounting-m2/review/scale-final/native.log)
record the affected behavior. The correction does not accept the strict PERF
budgets.

### Final retained measurement audit

The reviewer independently recomputed nearest-rank quantiles from the retained
raw samples, excluding first initialization index0 and using indices1…10 for
repeated startup/memory. All recorded p50/p95/p99/worst values match. Final p95
startup is825.448492ms for the small ledger and1274.926726ms for the quote-scale
fixture (n=10 each); exact-code picker lookup is184.853109ms (n=20 alternating
codes). Initial steady RSS p95 is88,308/171,324KiB respectively. These initial
memory samples are separate from the full-workflow282,864/311,204KiB RSS/HWM
point. Startup readiness is mapped controls, a real focus response and screenshot
completion; it does not prove completion of all deferred row construction.
OS caches and executor scheduling remain uncontrolled; no machine-cold,
full-reference-fixture, broad-lookup, population-tail or comparative speed claim
is accepted.

Current immutable binary and combined Rust/CSS/Cargo hashes match the observer's
before/after hashes. Actual copied-profile generation/payload hashes match both
initialized and final records, originals remain unchanged, and source catalog,
customer, quotation and accounting arrays remain exact after schema migration.
Both SQLite integrity checks pass. All22 retained application logs have zero
WARNING/CRITICAL diagnostics; helper accessibility warnings are separate.

Retained [raw repeated measurement](../docs/assets/accounting-m2/performance/performance.json),
[allocation warning check](../docs/assets/accounting-m2/performance/allocation-warning-check.json)
and [independent quantile/state/hash audit](../docs/assets/accounting-m2/review/repeated-performance-review.json)
provide the evidence. The observer's startup/lookup thresholds pass for these
specific fixtures and boundaries; overall PERF-01/PERF-02 remain partial.

## Findings and conclusion

**Independent bounded technical checkpoint accepted:** calendar/date/history
preservation, versioned migration/recovery, native navigation corrections and
editor scale functional protections have reviewable evidence with no unresolved
material correctness defect found in this scope. The retained measurement audit
passes for its declared observer boundaries. Overall PERF acceptance is withheld
because strict heartbeat/page budgets remain missed and the full reference
fixture is untested. This bounded engineering checkpoint does not
close full G1/OPEN-01, professional accounting acceptance, a trading/wholesaler
launch, statutory capability or production readiness. Delegated guide-based
engineering policies can proceed; this review does not block their implementation
on professional approval.

Interrupted schema upgrades, hardware power loss, genuinely machine-cold startup,
the full reference performance fixture, high-contrast/enlarged-text matrix,
complete keyboard-only financial entry, screen-reader use, representative native
Persian-reader/trader/accountant task studies, physical printing, Wayland,
Windows and packaging remain unaccepted here.
