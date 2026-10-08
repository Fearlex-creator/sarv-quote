# M5 native functional evidence

Candidate1 was rejected for large-list keyboard scope after a separate real 120-draft/60-filtered/25-per-page pilot could not reach Next through 160 Tabs. The retained failure is in `rejected-diagnostics/candidate1-large-list-pager/`. Its guarded release still passed 54 native functional checks in `candidate1/sales-independent-native.json`. The UI owner personally inspected 14 final native rasters listed with hashes in `candidate1/raster-inspection.json`. This is Linux/private-X11 GTK4/libadwaita evidence; the root integrator owns overall milestone acceptance and the separate performance report.

The frozen binary SHA256 is `db63cb2eef2d80bc6656e9f51a1ea9e262cc8ef27eb1df44c3a9fea58f8636d4`. Every production file and the aggregate `4da38731bf7d6208d39f233a4f7ef1f654cac798d437e726659333ec3da38d08` matched the root manifest before and after the native workflow. Observer SHA256 `8b61b8e68e457bc72a8fabcdadab27bb066090ee98444a67ba5083d9b77bf0fd` also remained unchanged. The manifest retains base HEAD `24ae57bc179336ad21d7d1d52501f49e80f3330b` and the source file hashes; it is a dirty-source freeze, not a claim that HEAD alone contains M5.

Reproduce from `/workspace/sarv-quote`:

```bash
source /workspace/sarv-environment/activate.sh
SARV_QA_BINARY=/workspace/sarv-evidence/bin/sarv-M5-candidate1 \
SARV_SALES_SOURCE_MANIFEST=/workspace/sarv-quote/docs/assets/accounting-m5/checks/candidate1/manifest.json \
SARV_SALES_SEED_PROFILE=/workspace/sarv-quote/artifacts/performance-m4/freeze6-repeated/procurement-posted-workflow \
dbus-run-session -- /usr/bin/python3 scripts/sales-qa.py
```

Each invocation creates a new disposable profile and its own D-Bus session/X11 server. It reads the actual M4 synthetic database with SQLite backup, renders the archived M4 binary before upgrading the copy, and uses the normal native forms and worker/Store boundary. The original M4 core/opening and procurement state remain exactly equal after the full M5 sequence.

The native sequence selects SKU-A from the imported 20-rial catalog, saves revisions of the raw order draft while retaining newer GTK edits, accepts the 199-rial nonfinancial order, posts paired partial invoices 100/99 with costs 58/58, returns credits 40/60 with attributable original costs 23/35, and converts an authentically approved zero-tax quotation through the native selector and explicit warehouse mapping. Independent raw-SQLite checks establish final Inventory422, AR299, AP329, Sales199, Returns100, COGS58 and balanced trial1630/1630. Every journal is balanced, paired IDs are distinct, and returns do not reopen the original order quantity. Controls CSV/PDF are retained in `candidate1/full/`.

Actual XTest keys prove Alt0/CtrlF search, exact Tab/Enter source actions and drilldown, over-return rejection with retained quantity6 and unchanged books, Escape cancellation, and a complete saved-draft-after-restart path: line Edit, nested picker CtrlN suppression/Escape, modal CtrlS, reviewed CtrlEnter order acceptance, paired sale100/cost58, and original-cost return40/cost23. The stage path uses its own independent database copy. The invalid raw quantity `-۱` and fractional-rial price `0.1` survive save/restart and are shown unchanged in the reopened line editor. Native standard/narrow light/dark captures and narrow editor captures are retained.

Page, source-detail and order-preview workers publish truthful completed/error/stale/closed statuses. A source read cancelled while its worker is pending is explicitly verified to preserve the books and clear loading status. Real-key checks require the exact requested control to be focused; they never substitute ancestor focus or AT-SPI clicks.

The original `stage/M5-real-keyboard-posted-return.png` was captured after DB durability while the GTK receipt was pending. Its retained copy is labeled `candidate1/stage/M5-real-keyboard-durable-return-dispatch.png`. The completed outcome is `candidate1/posted-return-inspection/M5-real-keyboard-completed-return-detail.png`: a separate read-only restart of the same final stage profile, real Alt0/CtrlF/Tab/Enter drilldown, exact40/23 return, idle completion footer and identical financial state. `inspect-posted-return.py` and the supplement JSON retain the repeatable procedure and repeated source/binary checks.

Failed diagnostics are retained under `rejected-diagnostics/` and do not establish acceptance. They led to actual quotation acknowledgment timing, workspace shortcut capture, truthful loading completion, sticky review and direct GTK row/pager traversal fixes; observer-only corrections covered field captions and waiting for the UI close receipt after DB durability.

This evidence does not establish Wayland/Windows, a screen-reader task, physical printing, high contrast/enlarged-text usability, native accountant/Persian-reader task approval, statutory compliance, settlements/refunds, or performance budgets. Those claims require their own evidence. Native catalog price selection is driven through actual GTK form signals; the keyboard supplement specifically proves picker entry/cancellation and preservation, not every catalog-selection combination.
