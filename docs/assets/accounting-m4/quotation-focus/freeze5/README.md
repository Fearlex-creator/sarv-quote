# Final freeze5 native quotation proof

Exact executable `/workspace/sarv-evidence/bin/sarv-M4-freeze5`, SHA-256 `7b390bb6287b6803081b743e676f196de17840e9e6346e0d4c507f4d3fa81a4a`; production aggregate `6b1996a41ed86807a91a18820f7fce4bb167e99399403b3fd25cf1b14e4aa36c`. [Source/build manifest](../../checks/freeze5-manifest.json) and [actual results](summary.json) bind every run. All source files and immutable binary matched after collection. Runs used private X11/Xvfb, actual XTest keyboard/AT-SPI actions, `G_DEBUG=fatal-criticals`, no interposition shim and synthetic profile clones. Every generated application/GTK/Adwaita/GLib log scan is clean.

- [Full28](full28/product-verification.json): preserved searchable customer/product pickers, source prices, quotation revisions, PDF/CSV, settings, restart and 900×560 document totals/focus.
- [Disclosure4](disclosure4/product-verification.json): edited metadata, multiline notes/CSV and exact saved history survive collapse/reopen.
- [Modal3](modal3/product-verification.json): real nested Back/Cancel/native Close preserve parent inputs, saved state and keyboard focus.
- [Validation failure3](failure3/product-verification.json): invalid review retains exact state and edits; name/alias fields become keyboard editable after async failure.
- [Escape6](escape6/product-verification.json): actual focused field's accessible ancestry belongs to the visible Composer; Shift-Escape preserves it; currency-popup Escape closes only that popup; nested confirmation Escape from real focused Save closes only inner; plain Composer Escape from name field preserves full saved state; native Close still works. Main focus remains usable.

The initial latest Window event remains `focus_scope=window`, because subsequent modal keys are not always observed there. Actual focused-field ancestry establishes modal ownership; later real native events contain `focus_scope=dialog`. Exact header/FlowBox keyboard focus was **not witnessed**; no such dynamic acceptance or geometry is claimed. The final screenshot shows actual name-field focus before plain Escape. GTK SearchEntry may own Escape as stop-search: the [original search-focused trace](escape-search-owned-history/product-verification.json) has three passing checks then timeout with inner search still focused. Nested-dialog dismissal is checked separately from its Save button. The [second original trace](escape-row-toggle-observer-history/product-verification.json) passes the first five checks then times out after the observer mistakenly collapses an already expanded source row. Corrected native-Close observation uses the visible New Catalog action. Both original reports/screens/logs are retained, without being called complete passes.

Personally inspected final5 `full28/editor-900x560-total.png`: focused discount, complete visible exact draft total **305.4 toman**, old source price and document distinction are readable. The approved exported quote remains separately **2,070 IRR**. The window-only currency-step and Composer-focus screenshots were also inspected. The currency popup itself is not visible in the window-only capture; its open/closed ownership is established by actual accessible visible-option states and Escape behavior, not that screenshot.

The retained observer copies operate on `scripts/product-qa.py` without production edits. Reproduction from the repository with the cloud environment activated:

```bash
source /workspace/sarv-environment/activate.sh
SARV_QUOTE_DIAG_BINARY=/workspace/sarv-evidence/bin/sarv-M4-freeze5 G_DEBUG=fatal-criticals dbus-run-session -- /usr/bin/python3 artifacts/m4-quote-diagnostics/repeat-final/full28.py --profile artifacts/m4-preserved/quote-mcjjhntx --expect-sha 7b390bb6287b6803081b743e676f196de17840e9e6346e0d4c507f4d3fa81a4a
```

Use the printed full28 clone path as `--profile`, add `--after-approval`, and run `disclosure4.py`, `modal3.py`, `failure3.py` and `escape6.py` with the same binary/fatal/hash environment. Each creates a fresh clone. These commands depend on the synthetic source profile and current scripts; `observers/` and summary observer hashes preserve the exact final observer provenance. No SQLite profiles, executable or caches are included in this portable bundle. Freeze2 and freeze4 results remain their original binary-specific evidence.
