# M6 native purchase-return acceptance brief

Prepared against M5 `eb8f8c29648fd4a206a26d65ae4eaca5cf945a1b`. This is a design and test contract, not an acceptance claim. The complete accounting blueprint and execution brief and sealed M4/M6 APIs were read before implementation.

The existing Persian GTK4/libadwaita Purchases workspace gains **برگشت خرید با بهای اصلی** and a source action on operational receipt/bill rows. A detached worker loads source eligibility and the indexed/paged picker searches source reference, supplier, item, warehouse and identity. One editor belongs to one unbilled operational receipt or one matched bill. Opening-only GRNI/AP and exhausted original bill lines are excluded. Unavailable source detail fails visibly.

The editor reuses at most25 persistent row widgets and keeps exact raw text/costs for every off-page line. Each edit recalculates only its bound line and adjusts a cached total/invalid count. Paging neither changes a draft nor invalidates an unchanged verified command; an invalid off-page field is revealed and focused. Live pager controls remain enabled, with boundary activation a no-op.

The editor shows original supplier, warehouse, source identity, original and remaining base quantities/value. Quantities are positive base-unit thousandths. Empty rows are omitted. Warehouses and values are read-only. Supplied costs are calculated with the sealed original/remaining cumulative half-up cursor, preserving exact final residuals and zero-value lines. The billed mode requires a separate credit reference and stable identity; the unbilled mode has neither. No receipt entitlement, PO received quantity, payment/refund or current-cost adjustment is introduced.

Native date controls convert Gregorian/Jalali without reinterpreting entered values. Rial/toman display changes do not change canonical values or the verified command. Preview uses the existing detached read-only Review worker. Return-only source/pool details reuse at most25 label slots and persistent pagers; the exact stock/AP-or-GRNI total and journal rows remain visible. The coordinated M6 performance observer exercises all200 off-page draft lines, an invalid last-page field, first-page retained text and all preview pages. Commit uses the shared detached atomic Store command boundary. Changes invalidate preview; invalid input is retained and focused. Closed dialogs and changed source/navigation callbacks are rejected. During BUSY, a new source command is gated. Lists, detail pages and source pagers clear focus before replacing live descendants and weakly restore surviving/matching controls only when newer focus has not won.

Posted Return documents show return/credit states and separate IDs, original source/cost, before/after pools and exact journal rows. Buttons drill into original bill/receipt documents. Original documents link back to returns/credits. Existing AP/GRNI/inventory cutoff controls remain the financial reconciliation route.

`purchase-returns-qa.py` is a separate standard-library observer. It requires a genuine synthetic M5 March31 seed, independent-reviewer April2 unbilled branch, frozen dynamic source manifest and exact binary. Actual native XTest on owned X11/private D-Bus performs indexed source selection, bad-input retention/cancel, Jalali/toman, read-only preview, atomic double-Enter posting, original/source drill-down, narrow/standard light/dark, keyboard focus and restart. Raw SQLite independently checks billed3/45 → Inv377/AP284/TB1585, final4/60 →317/224/1525 and separate unbilled1/15 →497/AP359/GRNI95/TB1705. The oracle checks immutable earlier openings/procurement/sales/quotes and original warehouse literal pools. Source, binary and observer guards run before and after proof. Fatal GTK criticals and focus/minimum-height diagnostics fail.

Run after integration GO, never during a quiet benchmark epoch:

```sh
source /workspace/sarv-environment/activate.sh
SARV_QA_BINARY=/absolute/frozen/sarv-quote \
SARV_PURCHASE_RETURNS_SEED_PROFILE=/absolute/synthetic/M5/profile \
SARV_PURCHASE_RETURNS_UNBILLED_PROFILE=/absolute/reviewer/native-unbilled-stage/profile \
SARV_PURCHASE_RETURNS_SOURCE_MANIFEST=/absolute/frozen-source-manifest.json \
dbus-run-session -- /usr/bin/python3 scripts/purchase-returns-qa.py
```

Raw binary/SQLite/native runs stay in ignored `artifacts/purchase-returns-native/run-*`. Portable screenshots/logs/receipts/full-State JSON and a verified hash manifest are copied into `docs/assets/accounting-m6/native/run-*`. The observer also captures the real archived M5 candidate12 purchase page from an untouched disposable schema6 copy for comparison.

Physical printing, actual screen-reader task use, Wayland/platform coverage, hardware power loss, professional accountant acceptance, full PUR/INV/treasury, strict PERF/G3/G4 and statutory capability remain outside this native proof. M5's documented performance misses remain explicit. Export/restore and adversarial kernel/CAS proofs belong to their separately coordinated evidence owners.
