# M4 native evidence — freeze 3

The immutable release passed **30 native workflow checks**, **eight short-window cases**, and **eight actual refresh/global-footer completion checks**. [Before](provenance-before.json) and [after](provenance-after.json) verify every production source file, binary and both observers remained unchanged. [Portable file hashes](files-sha256.json) cover exact artifact bytes. Historical freeze2 evidence remains unchanged in the sibling `final` directory; this set records the subsequent completion-status fix.

Binary SHA-256: `6eed73ccede51feadd3353393315667329f8136205ad46c973c9e9ece6da44b4`.
Production source aggregate: `ec50e593693bff141271b490cf2cce74005c3b3117324d7285fa9e6f631b56b9`.

## Checks and observed behavior

- [Full native result](procurement-independent-native.json): **30 passed, zero failed**. Native forms build the connected purchase workflow from a retained M3 opening seed; independent raw SQLite arithmetic checks effects. External XTest enters rejected fractional rials, adds/edits/removes an unposted line, cancels with Escape, uses Ctrl+F for `REC-A`, then restarts and captures both pages at 1280×900 and 650×760 in light/dark.
- [Short native result](short-native-layout.json): **8 passed**, covering purchases/inventory, light/dark, 650×550 and 900×560. Sixteen actual Tab focus observations verify current page identity and full control containment in both GTK root and nearest scroll viewport. After 250 ms settle, a harmless Shift event records existing GTK compute_bounds. Requested X screenshot dimensions and GTK client dimensions differ by window margins and are recorded separately. AT-SPI SCREEN x0/y0 values are diagnostic only, never numerical placement evidence.
- In every short case, actual Return on the focused refresh button is verified by the native key event. The settled accessible hierarchy must contain both page/global completion labels **«منابع و گزارش پایان روز آماده است.»** and no **«خواندن منابع و کنترل پایان روز…»** label. **8/8 passed**. Current screenshots show the completed global footer, replacing the stale reading status observed in freeze2.
- [Raw key events](keyboard-events.jsonl) retain actual page, focused widget/name/label, root bounds, scroll clip and viewport. All sixteen short screenshots are included.
- **All five M4 app logs are empty**: full workflow, light/dark restarts and light/dark short runs. Runs used `G_DEBUG=fatal-criticals`; warnings/criticals were not filtered. Infrastructure messages are retained in full/short runner logs.
- [Financial comparison](raw-financial-comparison.json): opening structure exactly preserved; **10 source records, 5 journals, 4 movements**. PO 12×15 rial=180 has no journal. Receipts 75/105; bills 30/150. Transfer 6 pieces/70 rial has paired movements and no journal. Final W1=30 pieces/350 rial, W2=12 pieces/130 rial; Inventory GL480, AP329, GRNI50; trial debit/credit **1431/1431**. Unrelated balances remain unchanged. Cancellation, restart and all eight read-only refreshes preserve posted accounting exactly.

The environment is private X11/Xvfb/Openbox, GTK4.22.5, stable libadwaita1.9.4 and Cairo on the declared cloud executor. This proves the observed bounded workflow, not performance budgets, production readiness, tax compliance, Windows/Wayland behavior or screen-reader speech.

## Before/after and PDF samples

The immutable M3 binary captured its own schema4 opening profile: [standard before](M3-before-light-1280.png), [narrow before](M3-before-light-650.png). Freeze3 after: [standard light purchases](M4-after-procurement-light-1280.png), [narrow dark purchases](M4-after-procurement-dark-650.png), [standard dark inventory](M4-after-inventory-dark-1280.png). [Transfer review](procurement-transfer-preview.png) displays original/final paired quantities and values, explicitly without a journal. [Short light inventory refresh](inventory-light-650x550-focused-refresh.png) and [short dark purchase action](procurement-dark-900x560-focused-action.png) show fully revealed focus and the completed global footer. Both pages intentionally share the documents tab; inventory keeps its own heading and transfer action.

The owner personally inspected fresh standard/narrow light/dark images, both short sizes/appearances, before/after comparisons, transfer preview and first-page PDF renders. Persian captions and mixed-script codes were readable and observed focused controls were fully visible. Other rows require scrolling in short windows.

Fresh native background exports: [order PDF](M4-order.pdf), [four-page control PDF](M4-controls.pdf), [control CSV](M4-controls.csv). [Order preview](M4-order-rendered.png) and [control preview](M4-controls-rendered.png) render first pages. Control PDF displays toman (Inventory48 toman=480 rial); stored financial checks use integer IRR. These samples contain order/control data, not resource-role boolean fields. The independent reviewer separately verifies Persian resource-role PDF labels and unchanged canonical source snapshots.

## Reproduction and manual demonstration

```sh
source /workspace/sarv-environment/activate.sh
G_DEBUG=fatal-criticals SARV_QA_BINARY=/workspace/sarv-evidence/bin/sarv-M4-freeze3 dbus-run-session -- /usr/bin/python3 scripts/procurement-qa.py
```

This creates a fresh disposable synthetic profile under `artifacts/procurement-native`. Run the included short observer with the same binary and `SARV_M4_NATIVE_FIXTURE` pointing at that run's `full` profile. It copies the synthetic database read-only into another isolated profile. No database, executable, runtime directory or customer data is included here.

For a manual walkthrough in an isolated synthetic copy, select `F2-core-v1`; Alt+8 opens purchases. Inspect `PO-M4-12`, `REC-A`, `REC-B`, `BILL-A`, `BILL-B` and the transfer. Ctrl+F searches; Tab reveals focus, Enter activates buttons and Escape cancels an unposted form. Alt+9 opens inventory. Choose end-of-day 2026-03-27 and control/detail for exact reconciled quantity/value and GL roles. Switch rial/toman; inspect frozen catalog price provenance and document exports. Activate end-of-day refresh and observe both page/global ready status. New orders support staged line add/edit/remove and retained validation failures, but staged unposted orders are not durable drafts; discarding loses those unposted inputs. Posted history stays immutable.

Sales/delivery/COGS/returns, payments, cheque lifecycle, closing and statutory integrations remain later milestones. Financial, migration/recovery, independent review and performance evidence are tracked separately.
