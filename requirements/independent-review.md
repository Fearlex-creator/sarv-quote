# Independent review — bounded first accounting checkpoint

Reviewer: separate agent with no implementation ownership. Review baseline is
`4aa82c2`; final reviewed source/checkpoint hash and rerun results will be recorded
after integration is stable. All fixtures are synthetic. Both normative Markdown
attachments were read completely, including their accounting invariants, release
gates, exact-money policy proposals and acceptance rules. The user approved integer
IRR with exact rial/toman conversion and rejection of fractional rials; no chart,
recognition, tax, closing or valuation policy was approved.

## Review boundary and independent expected outcomes

This review covers the bounded company/custom-chart/fiscal-period/ledger-opening/
manual-journal/retry/reversal/lock/trial-balance workflow and preservation of native
quotation behavior. It does not establish full G1, OPEN-01, trading/service launch,
statutory compliance or production readiness. Accountant approval is pending.

The independent ledger fixture uses a custom asset account and equity account.
Opening Dr asset 1,251 IRR / Cr equity 1,251 IRR must produce two rows and totals
1,251 / 1,251, displayed exactly as 125.1 toman. A subsequent journal Dr asset
349 / Cr equity 349 must produce 1,600 / 1,600. An exact linked reversal of that
journal must restore 1,251 / 1,251 while preserving the original journal. Restart
and fresh-profile restore must preserve IDs, journal amounts, retry identity,
audit and trial balance. Different retry keys for the same source/revision/event
must produce one journal. A conflicting retry must fail without mutation.

Unbalanced, zero, negative, dual-sided, wrong-company, locked-period, out-of-date,
non-postable and unsupported control-account lines must be rejected without
committing state. Direct Store mutation must not remove or alter historical
journals/audit/request records, unlock periods or evade posting validation. A
verified whole-profile restore must remain a distinct explicit recovery action.

Migration review must start with schema 1 and preserve quotation IDs, totals and
revision snapshots; verified recovery must precede schema 2. Migration must be
repeatable and an interrupted upgrade must preserve the old or complete new state.
Process interruption is not evidence of physical hardware power-loss durability.

Native review must actually run GTK, interact with controls, inspect standard and
narrow layouts, keyboard focus and reported loading/error states, and compare the
visible amounts to exact fixture outputs. Screenshots alone cannot pass financial
correctness. Full screen-reader/native Persian reader, physical printing, Wayland,
Windows and accountant reviews remain untested unless separately evidenced.

## Early material findings

1. **Fixed and independently reproduced — linked reversal after account archive:** initial kernel
   rejected archived accounts even for exact reversal, stranding referenced
   history after the advertised archive operation. Requested original-snapshot
   reversal support or a clear archive constraint. Current reversal uses original
   snapshots and permits an archived referenced account; the scratch probe passed.
2. **Fixed and independently reproduced — second-year opening duplicates carry:** initial kernel
   allowed one opening per period while trial balance accumulates all company
   journals. Re-importing carried balances into another fiscal year would double
   them. With closing deferred, requested one initial company opening and explicit
   disclosure that later carry/opening migration is unsupported. Current kernel
   rejects a second company opening; the scratch probe passed.
3. **Implementation changed; final native verification pending — synchronous
   accounting saves:** initial GTK callbacks used synchronous aggregate Store
   changes. They now dispatch detached validation/serialization/durable commit.
   Chart/audit truncation beyond 100 records was also replaced with searchable
   pagination. Actual native latency and interaction remain to be measured.
4. **Fixed and independently reproduced — cumulative balance overflow:** individually valid journals
   could exceed the supported cumulative report range and strand trial balance.
   Kernel owner is adding pre-commit validation of all affected financial cutoffs,
   including future cutoffs after a backdated posting. The reviewer probe now
   independently rejects MAX+1 and a backdated overflowing earlier cutoff while
   allowing a valid reduction to MAX-1, with exact pre/post state equality on
   rejection. Raw result: `docs/assets/accounting-m1/review/ledger-range-probe.log`.
5. **Fixed and independently reproduced — mutable cached Store state:**
   direct modification of public `Store.state.accounting` followed by an ordinary
   `Store.change` bypasses transition checks against the cached prior state. The
   reviewer reproduced removal of all three persisted journals without using
   explicit restore. `cache-bypass-before.log` records actual successful change
   and zero persisted journals; `store-cache-probe.rs` asserts this must reject.
   Detached commits already compare the actual database state. Requested matching
   synchronous checks against the actual committed row, inside its transaction.
   Both synchronous paths now validate the actual committed row inside an
   immediate transaction. Expanded reviewer reproduction rejects ordinary,
   prepared and detached history removal and preserves all three journals;
   explicit verified restore still works. Result: `cache-all-after.log`.
6. **Fixed and independently rendered — supported long report codes:** PDF row sizing measured
   only account name; a valid long account code could wrap beyond that row and
   overlap following financial rows. Requested all-cell row measurements and
   rejection of unrenderable page headers/rows before publication. The generated
   24-fragment code fits without overlapping the next financial row; repeated
   company/cutoff/unit headers and exact total render on page 2. Independent text
   extraction and CSV comparison preserve all codes/markers and `1,251 IRR =
   125.1 toman`. Oversize row/header fail before publication with CSV fallback.
   Artifacts: `/workspace/sarv-evidence/independent-report-final/` and
   `docs/assets/accounting-m1/review/report-final-probe.log`.
7. **Fixed in source; save-specific runtime review belongs to its declared probe — queued quote save during
   accounting:** busy quotation autosave consumed its timer without resubmission
   after accounting completion. A bounded retry now retains the queued save.

## Rerun results

Independent standalone library/Store probe passed on the evolving implementation
with the exact expected `1,251 → 1,600 → 1,251 IRR` trial balance, `125.1 toman`,
one journal for a retry with a different key, conflicting/unbalanced/locked
commands rejected, direct history/direct unlock mutation rejected, archived exact
reversal allowed, second opening rejected, restart/fresh restore equality and
signed i64 range exact toman/Persian digit checks.

Reproducible probe source/manifest and raw result are under
`docs/assets/accounting-m1/review/`. Run from the project with its configured native
runtime/toolchain and a fresh synthetic evidence directory:

```sh
source /workspace/sarv-environment/activate.sh
CARGO_TARGET_DIR=/workspace/sarv-quote/target cargo run --offline \
  --manifest-path docs/assets/accounting-m1/review/Cargo.toml \
  --bin sarv-independent-review-probe \
  -- /tmp/sarv-independent-review-fresh
```

This is a separate reviewer fixture, not an implementation test copied from the
kernel. Additional bins `sarv-independent-cache-probe` and
`sarv-independent-report-probe` reproduce the persistence and PDF boundaries.

The reviewer independently reran the final targeted Rust tests after the Store
transaction/cache and report fixes:

```sh
cargo test --locked --offline --test accounting_kernel \
  --test accounting_migration --test detached_save \
  --test accounting_exports --test process_recovery
```

Result: **28 passed, 0 failed, 0 ignored**: kernel 12, migration 2, detached save 4,
export 1 and recovery 9. Raw log: `final-targeted-rerun.log`. Actual detached
journal process-kill checks now cover before commit and after durable commit but
before owner receipt: zero partial journal effects, exact 1,251 IRR balanced
posted state after commit, restart/integrity and fresh-profile restore.
Interrupted schema migration and physical power loss remain untested.

## Independent native rerun and visual assessment

The reviewer independently ran:

```sh
source /workspace/sarv-environment/activate.sh
dbus-run-session -- /usr/bin/python3 scripts/accounting-qa.py
```

**16 passed, 0 failed**, on release binary SHA-256
`ae35fe5d21095af53c194e75081d8b2087f7e31d96a3b0d1565a7f1aa1228f65`.
Reviewer profile: `artifacts/accounting-native/run-dy8ivtxi`.
Raw reviewer invocation/result: `docs/assets/accounting-m1/review/native-accounting-passed.log`.
Source hashes for accounting files are recorded in `reviewed-source.json`.

The financial portion uses real GTK widgets and their signals: company, fiscal
period, explicit custom chart, invalid unbalanced input retained, balanced opening
with double-click protection, manual adjustment, linked reversal, period lock
rejection/reopen reason, exact rial/toman reports, exports and restart/fresh-profile
restore. A separate actual XTest portion proves Alt+7 navigation, Tab to controls,
Space on the journal tab, Enter on the journal action and Escape dismissing the
unsaved journal without changing persisted state. It accepts 1280×900 and 650×760
native allocations. This does **not** claim that every financial field was entered
exclusively by real keyboard input or establish screen-reader usability.

The reviewer independently read the committed SQLite payload and recomputed
journal debit/credit sums from raw lines, without calling production accounting
calculations: journal debit totals **[1,251, 349, 349] IRR**, exact final custom
asset/equity balances **+1,251 / −1,251 IRR**, three historical journals retained,
third linked to the original adjustment, integrity check `ok`. Independent PDF
text extraction and CSV field checks confirm **1,600 rial before reversal** and
**125.1 toman after reversal**, with canonical amounts still **1,251 IRR**.
Result: `native-independent-amounts.json`.

Actual native standard/narrow report and journal-dialog screenshots and rendered
final PDF were inspected. RTL hierarchy, Persian text, canonical Gregorian dates,
explicit amounts/units and visible keyboard focus remain readable; amounts and
financial rows are not clipped in the reviewed fixture. Narrow tabs require
horizontal scrolling and the long journal form scrolls beneath persistent actions.
The inspection establishes this bounded fixture's rendering, not representative
Persian-reader/accountant task-study acceptance or general accessibility quality.

One initial native harness attempt failed because it selected/activated a duplicate
accessible node for the journal action via pointer coordinates; its screenshot
showed the journal page instead of a dialog. The harness now uses real Tab/Enter
and verifies the actual dialog and Escape close. The final separate reviewer rerun
passes; only passing final results are retained in the checkpoint review bundle.

## Review conclusion and limits

No unresolved material financial correctness/history-loss defect was found in the
bounded manual-ledger checkpoint after the fixes and independent reproductions
above. Engineering evidence supports this custom-account ledger slice; it does
not close full G1/OPEN-01, accounting-policy approval, service/trading launch or
production acceptance.

The reviewer did not accept the full performance budget or million-line fixture.
Native save/close, catalog lookup and later picker-only refinements have separate
declared probes/reports. Heartbeat gaps do not by themselves prove a hard
main-thread scheduling budget. Interrupted schema migration, physical power loss,
OS-cache-cold startup, full reference hardware/dataset, dark/high-contrast/enlarged
text matrix, full keyboard-only financial entry, screen readers, representative
native Persian-reader tasks, Iranian accountant approval, physical printing,
Wayland, Windows, packaging and statutory readiness remain unaccepted here.


## Final checkpoint delta review

Final product binary SHA-256:
`65bedf489894bd6435b57a9919fadf0a31e2e1d9809f97089212dbf43bfc4378`.
The reviewer compared the recorded hashes of `src/accounting.rs`,
`src/accounting_export.rs`, `src/store.rs` and `src/ui/accounting.rs`: all four
remain identical to the independently tested accounting checkpoint. The later
UI changes were reviewed in source:

- Product picker uses an 80 ms search delay and isolates displayed product codes
  and source dates. Stored codes/prices/source identities and matching rules are
  unchanged.
- Explicit Save and Ctrl+S queue the existing asynchronous save and then refresh
  the row review. Failure clears the continuation; edits newer than a worker's
  snapshot are validated/saved before the continuation runs. This restores row
  amount refresh while retaining dirty-input and concurrency protections.
- Restore preview explicitly discloses company/period/account/journal counts and
  replacement of all current books/history. After a successful verified restore,
  the accounting workspace is rebuilt so old company/period IDs cannot remain
  selected. The persistence replacement/safety boundary is unchanged.

No new material correctness issue was found in those changes. The earlier
independent 16-check native accounting run remains valid for the unchanged
accounting component; the integrating agents' final native save/quote/performance
runs are separately attributed rather than relabeled independent reproduction.

The reviewer independently checked nearest-rank calculations and stable
source/binary hashes in the final performance JSON. New-process startup p95 is
**885.615 ms** (small ledger, n=10) and **1,428.802 ms** (50,000 products/1,000 quote
lines, n=10). Exact-code selectable product lookup p95 is **193.580 ms**, worst
**198.957 ms**, n=20. These measurements include the declared observer/debounce
boundaries and use warm/uncontrolled OS cache; they do not establish machine-cold
startup or general alias/party lookup performance. The final native scale report
retains the **970.604 ms** whole-probe worst heartbeat and other page/PDF budget
misses. No overall 50 ms main-thread or full PERF gate pass is accepted.

The root retained the reproducible reviewer crate/logs/fixture in
`docs/assets/accounting-m1/review/`; its manifest points to the repository by a
relative path and includes a lockfile. The integrating local commit identifies
this source snapshot; the reviewed binary/source hashes retain the exact scope
of the independent runs.
