# Actual final freeze6 native quotation proof

Immutable executable `/workspace/sarv-evidence/bin/sarv-M4-freeze6`, SHA-256 `9be5f0ba6a7fe6f000b722042d426987e79647578ab9ca803a9869ee1a343d5e`, production aggregate `4545e5f018a80a9f403c686ecd19ee2ec66bbc683c6a8d80bb293ba6dd98fc14`. [Manifest](../../checks/freeze6-manifest.json) and [results](summary.json) pin all five actual runs. All32 source files and binary matched after collection. Private X11/Xvfb, actual XTest/AT-SPI actions, `G_DEBUG=fatal-criticals`, no LD_PRELOAD shim and synthetic profile clones; every generated application/GTK/Adwaita/GLib log scan is clean.

- [Full28](full28/product-verification.json): searchable customer/product pickers and source prices, exact quotation revisions, PDF/CSV, persistent settings/restart and visible900×560 totals/focus.
- [Disclosure4](disclosure4/product-verification.json): edited metadata, multiline notes/CSV and exact history survive collapse/reopen.
- [Modal3](modal3/product-verification.json): actual nested Back/Cancel/native Close preserve inputs, full saved state and keyboard interaction.
- [Validation3](failure3/product-verification.json): failed async review preserves data/edits, name/alias fields become keyboard editable, Cancel restores usable main focus.
- [Escape6](escape6/product-verification.json): focused name-field ancestry belongs to Composer; Shift-Escape retains edits; Escape closes only currency popup; nested confirmation closes only inner from real focused Save; name-field plain Escape and native Close preserve exact state/main focus.

Latest initial Window events may predate modal keys. Actual AT-SPI focused-field ancestry establishes ownership; later actual events report `focus_scope=dialog`. Exact header/FlowBox focus was **not witnessed**. SearchEntry can own Escape as stop-search; nested dismissal is tested from real Save. Window-only screenshots omit the popup surface; visible accessible-option open/closed states and actual Escape establish popup ownership. [Earlier5 original scope/observer failures](../freeze5/README.md) remain unchanged, not rerun or relabeled6.

Personally viewed actual6 narrow totals/focus: draft **305.4 toman** is fully visible, while the approved exported quote remains separately **2,070 IRR**. The actual6 Composer frame visibly focuses the edited name before plain Escape. This specific matched-document visual proof does not close the [separate candidate-debug unmatched1280px width warning](../../native/picker-async-diagnostics/README.md), an unresolved narrow/long-document gap. Clean logs here do not erase warnings from failed historical runs.

[Independent picker17](../../review/final-freeze6/native/picker-persistence-native.json) separately challenges atomic current-form/product/customer selection, writer locks, stale query, newer invalid input, full history, generation CAS, retry/cancel/restart. [Root quotation23/held-close3](../../preserved-freeze6/README.md) has its own exact6 provenance. These are distinct counts; no universal picker/timing/platform acceptance follows.

Reproduction with environment activated:

```bash
source /workspace/sarv-environment/activate.sh
SARV_QUOTE_DIAG_BINARY=/workspace/sarv-evidence/bin/sarv-M4-freeze6 G_DEBUG=fatal-criticals dbus-run-session -- /usr/bin/python3 artifacts/m4-quote-diagnostics/repeat-final/full28.py --profile artifacts/m4-preserved/quote-mcjjhntx --expect-sha 9be5f0ba6a7fe6f000b722042d426987e79647578ab9ca803a9869ee1a343d5e
```

Then use the printed full28 clone as `--profile`, add `--after-approval`, and execute `disclosure4.py`, `modal3.py`, `failure3.py`, `escape6.py` under the same binary/fatal/hash environment. Each creates a fresh clone. Observer copies and summary hashes retain exact provenance; original protected synthetic source is required. No SQLite database, executable or runtime cache is included. Earlier2/4/5 data, screenshots and timings keep their original source/binary attribution.
