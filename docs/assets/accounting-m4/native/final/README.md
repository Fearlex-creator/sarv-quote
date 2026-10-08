# M4 final native evidence — freeze 2

The immutable release passed the 30-check native workflow and eight short-window cases. These are synthetic profiles on private X11/Xvfb/Openbox, GTK 4.22.5 and stable libadwaita 1.9.4 with Cairo rendering. They establish the bounded procurement workflow and observed native interaction; they do not establish production readiness, tax compliance, performance budgets, Windows/Wayland behavior or universal visual accessibility.

Binary SHA-256: `614da6fa4ee6d334606a8d3182f1d91d52c9b494579c81cfe10cf41c763f9713`.
Production source aggregate: `895386c1b3e66d58460c3e704487e8450833cb7ea581cc370c0fa9febf1ec8f2`.
[Before](provenance-before.json) and [after](provenance-after.json) checks verify every production source file, immutable binary and unchanged full harness. [Files](files-sha256.json) records portable artifact hashes.

## Exact checks and financial result

- [Full native result](procurement-independent-native.json): **30 passed, zero failed**. Native form smoke performs the connected workflow; external XTest then independently enters invalid fractional IRR, adds/edits/removes an unposted line, cancels with Escape, searches `REC-A` using Ctrl+F, restarts and captures both pages at 1280×900 and 650×760 in light/dark appearances.
- [Short-window result and actual key/focus bounds](short-native-layout.json): **8 cases passed**, 16 actual Tab focus observations. Purchases and inventory pages, light/dark, 650×550 and 900×560. Each primary action and footer refresh control was focused through actual Tab keys. After 250 ms settle, a harmless Shift-key event verified the actual page and focus label and captured GTK compute_bounds within both the root window and nearest scroll viewport. Requested X screenshot sizes differ from GTK client sizes by window margins; both are recorded. The page scrolls to reveal focus. All 16 screenshots and the raw GTK keyboard-events JSONL are retained. The prior AT-SPI SCREEN x0/y0 numerical assertion was unsupported and rejected; [coordinate diagnostics](coordinate-diagnostics/README.md) preserve the original evidence and the observer-only outer/client-size correction.
- All five final M4 application logs are **empty**: full workflow, light/dark restarts, light/dark short-window runs. Runs used `G_DEBUG=fatal-criticals`; no GTK/Adwaita warning or critical was hidden or filtered. Infrastructure D-Bus/X-server messages remain in the full/short runner logs. The old rejected freeze-1 evidence and focus diagnostics are preserved in sibling directories.
- [Raw financial extraction](raw-financial-comparison.json): original opening structure unchanged; **10 source records, 5 journals, 4 stock movements**. PO 12 pieces × 15 rial = 180 is nonfinancial; receipts 75/105; bills 30/150; transfer 6 pieces/70 rial has two movements and no journal. Final W1 is 30 pieces/350 rial, W2 is 12 pieces/130 rial. Inventory GL 480, AP 329, GRNI 50, trial debits/credits **1431/1431**. Unrelated balances are unchanged. Cancellation and restart preserve the posted aggregate exactly.
- [Native form manifest](procurement-native-verification.json) records exact rial/toman conversion, retained fractional-rial errors, catalog price/unit provenance and explicit override reason checks. Smoke polling is observation, not a performance measurement.

## Images and PDF samples

Actual historical M3 screenshots were captured from the retained immutable M3 binary and its own schema-4 profile: [1280 px](M3-before-light-1280.png), [650 px](M3-before-light-650.png). Current purchase examples: [light standard](M4-after-procurement-light-1280.png), [dark narrow](M4-after-procurement-dark-650.png). [Transfer review](procurement-transfer-preview.png) visibly shows paired source/destination quantities and values with no financial journal. [Short primary action](procurement-light-650x550-focused-action.png) and [short dark footer](inventory-dark-650x550-focused-refresh.png) show revealed focus and the labelled resource history after operational documents.

The native evidence owner personally viewed these before/after images, standard and narrow light/dark layouts, four short-window images covering both window sizes/appearances, the transfer preview and both PDF first-page renders. Persian text, mixed-script item codes, field captions and focused actions were readable in those observed frames. Content outside the short viewport requires scrolling; this is not a claim that every row is simultaneously visible.

[Purchase order PDF](M4-order.pdf) and [four-page control report PDF](M4-controls.pdf) were generated through the native background export workflow. [Order render](M4-order-rendered.png) and [control render](M4-controls-rendered.png) are first-page previews. The control PDF deliberately displays toman (e.g. inventory 48 toman = 480 rial); stored/independently checked amounts above are integer IRR. [Control CSV](M4-controls.csv) is retained verbatim. Separate export evidence/review checks cover all-page financial content and bounds.

## Reproduce and demonstrate

From the repository, with the activated cloud runtime and retained M3 binary available:

```sh
source /workspace/sarv-environment/activate.sh
G_DEBUG=fatal-criticals SARV_QA_BINARY=/workspace/sarv-evidence/bin/sarv-M4-freeze2 dbus-run-session -- /usr/bin/python3 scripts/procurement-qa.py
```

This creates a new isolated synthetic directory under `artifacts/procurement-native`; it does not use the user's accounting data. Then run the portable short observer with `SARV_M4_NATIVE_FIXTURE` pointing to that run's `full` profile and the same immutable binary. The observer copies the SQLite fixture read-only into another disposable profile. Neither database nor executable is included in this portable bundle.

For a manual demonstration using an isolated synthetic copy, choose company `F2-core-v1`, enter purchases with Alt+8, inspect `PO-M4-12`, `REC-A`, `REC-B`, `BILL-A`, `BILL-B`, and the transfer. Ctrl+F searches the active page; Tab reveals focused controls; Enter activates a button; Escape cancels an unposted form. Alt+9 opens inventory. Select end-of-day 2026-03-27 and control/detail to inspect reconciled quantity/value and GL roles. Switch rial/toman and confirm exact display conversion. Open document details to inspect frozen source/price and export. A new purchase form accepts staged lines and retains rejected values, but unposted staged orders are not durable drafts; cancelling/discarding loses that unposted draft. Posted history is immutable.

Scope remains partial: sales/deliveries/COGS/returns, payments, cheque lifecycle, closing and statutory integration belong to later tracked milestones. This bundle is native evidence, separate from financial, migration/recovery, independent review and performance evidence.
