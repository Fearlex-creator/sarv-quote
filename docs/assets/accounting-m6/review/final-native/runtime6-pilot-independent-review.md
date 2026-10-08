# Independent candidate6/runtime6 pilot review

Reviewer: `m6_final_native_review`. Read-only native receipts, images, sources and SQLite review; no Cargo, GUI, product edits or replay. This is a development pilot, not a quiet final qualification. Review result: bounded financial/provenance evidence passes; performance and invalid-field visibility remain unaccepted.

## Exact reviewed evidence

- Production fingerprint: `39f5706031a0207f817c30978330f9e471db8863ae94a7d0e741b4cdbcfade4a`.
- Immutable candidate6 binary: `6951a758c0f60f8bfce84abf42b399ffe06c5105fda0dbc9978ac2b7262147b2`.
- Observer protocol `m6-scoped-v1.2`, source SHA256 `39409332a7caae630d45cf94858df7dc4baf9c0f56b377f47fbbe757b505fc0c`.
- Closed raw `artifacts/performance-m6/candidate6-runtime6-pilot1/observer/performance.json`: `fd9852a207959176c7f4d83cddaf9064df75a6f6727804603822c2f10769dd82`.
- Independent checker `check-runtime6-pilot.py`: `876fb6c6b7a619c977e6c9c8d9ed7ec60ca8b9437e7b2d6f47b32d84b3f22aa9`.
- Independent result `runtime6-pilot-independent-result.json`: `2b8b3d996bc249d4580eb6e164bfba3a32c5c8f139325ee6586c60793dd71e0c`, 132 passing assertions. Its `all_checks_pass` means verification of evidence and explicit limits; its `performance_accepted`, `invalid_field_settled_visibility_accepted` and `quiet_final` remain false.

All 80 portable inventory entries match their bytes and SHA256. Production/observer/binary before-and-after guards and protected dataset hashes match. Eleven stopped profiles have exact portable/raw SQLite States and pass SQLite integrity; 22 recorded owned processes are inactive. All 322 completed-boundary checkpoints are retained. Two actual X errors are precisely recorded and satisfy the reviewed narrow rejection: BadWindow=3, X_GetProperty=20, minor=0, exact `_NET_WM_PID` resource `0x800004`, owned private display and discovery scope. No unexpected error is waived.

## Independent functional and financial checks

The checker uses stdlib JSON and read-only SQLite with literal expectations, without Sarv command/report/kernel imports. It verifies 20 billed, 20 unbilled and 200 billed real quantity inputs; all preview source/allocation/movement details; exact journal sides, source line identities, generated unique identities, original-cost pool transitions, immutable prefixes, all three post restarts and whole-State compatibility fields.

| Case | Inventory before → after | Payables before → after | GRNI before → after | Trial balance debit/credit before → after |
|---|---:|---:|---:|---:|
| 20 billed | 722 → 422 | 629 → 329 | 50 → 50 | 1930 → 1630 |
| 20 unbilled | 722 → 422 | 329 → 329 | 350 → 50 | 1930 → 1630 |
| 200 billed | 3422 → 422 | 3329 → 329 | 50 → 50 | 4630 → 1630 |

All amounts above are IRR; liabilities are presented as positive balances. Every other literal account remains unchanged. In the 200-line case, all 200 receipt/bill source lines preserve exact order/identities; each return is quantity 1000 thousandths and original value 15 IRR, total 3000 IRR. All 400 preview details match those source and pool transitions. All fourteen page actions cover next pages 2–8 and previous pages 7–1. W1 starts at 225000 thousandths/3292 IRR and ends at 25000/292 through 200 exact out movements; W2 and earlier movement prefixes remain unchanged. The invalid input is corrected before the exact 200-line preview/post proof. These synthetic component cases do not accept full PUR, INV, G4 or production gates.

Six native preview/post phase raw gap summaries independently reproduce their terminal-boundary maximums below 50 ms (20 billed 16.103054/22.382303; 20 unbilled 22.286212/25.873401; 200 billed 16.084769/27.093449). Each is one pilot observation. Editor/read-only heartbeat gaps remain unmeasured. Six startup cohorts retain both initial and repeated observations and settled RSS arrays/counts; these small development samples do not qualify final percentile or quiet-environment gates.

## Timing misses and method limits

All twelve strict diagnostic misses independently reproduce from the retained raw arrays using nearest-rank percentiles. At n=2 or n=7, p95/p99 equal the worst observation.

| Scope | Correct visible worst ms | Focused usable worst ms | Budget ms |
|---|---:|---:|---:|
| Billed source query | 927.477 | 988.122 | 200 |
| Unbilled source query | 977.979 | 1037.847 | 200 |
| Billed filtered page | 874.935 | 960.153 | 150 |
| Unbilled filtered page | 1007.112 | 1100.769 | 150 |
| 200 editor next page | 1398.083 | 1733.403 | 150 |
| 200 editor previous page | 1413.335 | 1704.855 | 150 |

The endpoint observation performs accessibility discovery, descendant traversal and ancestry RPC work. These misses describe the defined observer-detected intervals; they do not establish application-only latency or a causal speed comparison. No RPC time subtraction, budget waiver or lost-array reconstruction was applied. Any revised discovery method must receive a separate protocol/epoch while preserving these results.

## Personally inspected 200-line images and remaining defect

- `invalid-last-row-focused.png`: `dfbbaa0636f5a0f98e9ca62245b509ea5bf5d2ca7719d7f7c891ecd3b1188265`.
- `last-preview-page.png`: `734e8e37e9bec2aa995a7a17288df04305135730947208b0ac35b359474c5165`.
- `posted.png`: `f883aae446f5ecfb42692474288ffbf35968c5422c01569d5036ede49c89033d`.

The final preview and posted state are readable. The captured last invalid quantity entry is clipped behind the fixed footer: its caption and top red edge appear, but the typed `2` is not readable. FOCUSED/text and an allocated rectangle are insufficient to prove visual revelation. A later root-owned settled diagnostic also reportedly reproduces the clipping; that separate receipt/images must qualify persistence independently. Candidate6 visual acceptance stays open pending the bounded product fix and actual settled verification.

Source cause is bounded: `ReturnRows.collect` binds the invalid page and calls `quantity.grab_focus()` without scheduling post-layout scroll revelation. The existing quote helper offers the adjustment algorithm, but its GtkWindow focus semantics should not be copied blindly into an owning libadwaita Dialog. A procurement-local helper can check the exact owning Dialog's current focus, preserve entry identity across the reused 25 slots, wait one frame, clamp both nearest scroller adjustments with a small margin for at most five frames, and stop on dialog closure, changed entry name or moved focus. Weak Dialog capture prevents a collect-closure ownership cycle; it must not regrab focus or alter the caret. This is a conditional implementation recommendation, not validation of future bytes.
