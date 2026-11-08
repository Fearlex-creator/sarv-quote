# Release gates

No tag was created and the Windows workflow was not started. The existing GitHub release `v0.1.0` points at main `f5888a16cc675eaaf58ca165363a39513b6695ac`, which does not contain this branch. Publishing this branch would attach a new release before the gates below are green.

## Green on this branch

- `cargo test -p sarv-quote --offline`: 275 listed, 272 passed, 3 ignored, 0 failed.
- Desktop typecheck, lint, and 14 unit tests.
- `cargo audit` reports no advisories on the lockfile that uses calamine 0.38, zip 9.4.2, and rust_xlsxwriter 0.102.
- `pnpm --dir desktop audit --prod` reports no known vulnerabilities. The shadcn CLI is not a production dependency.
- Shop-lock denial for a cashier journal, a signed-out workspace, and a legacy agent token. An owner-granted accountant token can read and cannot create users.
- Graph connector contract against a local mock. `live_verified` is false.
- A Linux deb was produced: `desktop/src-tauri/target/release/bundle/deb/Sarv_0.1.0_amd64.deb`, sha256 `98f7f9dbc83f08362e51779458bbf231a85467644d014c13729eec5f31cd4ecb`. A copy is `/opt/cursor/artifacts/sarv-linux/Sarv_0.1.0_amd64.deb`.

## Not green

| Gate | What is missing |
| --- | --- |
| PDF raster tolerance | `pnpm test:pdf --offline --no-fail-fast` passed 314, failed 1, ignored 3. `final_raster_has_every_monetary_cell_and_matches_independent_pdf_raster` compared cairo's page with poppler 24.02 (`pdftoppm`). Native ink in the first money cell was 186 and poppler's was 265. The test allows a gap under one fifth. Both rasters show the Persian quotation. Files: `/opt/cursor/artifacts/sarv-pdf/native-page.png` and `/opt/cursor/artifacts/sarv-pdf/poppler-page.png`. |
| Dev-tool advisories | `pnpm --dir desktop audit` (dev included): high `zod` &lt; 4.6.6 via `eslint-plugin-react-hooks` (GHSA-mrhx-c7xc-m3w5) and moderate `minimatch` via ESLint (GHSA-g3hv-75wg-m249). Not reachable from the production bundle. Thresholds were not raised. |
| Historical binaries | Ignored tests need `SARV_M3_COMPAT_BINARY`, `SARV_M4_COMPAT_BINARY`, and `SARV_M5_COMPAT_BINARY`. The schema 4/5/6 commits are `bdc304cb139e3d258b107b9ce18c7f4682ec5a89`, `24ae57bc179336ad21d7d1d52501f49e80f3330b`, and `eb8f8c29648fd4a206a26d65ae4eaca5cf945a1b`. Building them in isolated worktrees stopped: `libadwaita-sys` requires libadwaita &gt;= 1.9 and this machine has 1.5.0. GitHub release `v0.1.0` is a Windows Tauri zip, not those GTK binaries. Actions artifacts for this repo were empty. |
| Graph against Microsoft | The connector and a local mock pass. A real tenant still needs the owner to register an app and set `SARV_GRAPH_CLIENT_ID`. No secret is in the repo. |
| Windows artifact of this branch | `release-windows.yml` publishes on `v*` or a manual dispatch. Neither was run for this branch. Chrome does not prove Windows. |
| Production host | There is no separate server to deploy. The product is the local profile. |

## Browser run

Playwright projects are `chrome` (system Google Chrome), `firefox` (Playwright Firefox 1555), and `webkit` (Playwright WebKit 2370). One invocation of `pnpm --dir desktop exec playwright test` passed 78 and failed 0 (26 per browser, 12.3 minutes).

An earlier combined run on the same product, before two test waits and the opening-stock id fix, was 74 passed and 4 failed. The agent spec asked Firefox and WebKit for a Chromium-only clipboard permission. The retail sale then failed because opening-stock line ids `ln-0` and `rc-0` were global, so the second company in the shared QA profile was rejected. Line ids now include the request key. Clipboard checks use an in-page clipboard. A later WebKit-only miss was the sale toast's 5 second wait while checkout was still running; that wait is 20 seconds. The 78-pass run is after those fixes.

## Environment names

These are names only. Values stay on the machine.

- `SARV_DATA_DIR` — profile directory. Default is the app data dir.
- `SARV_AGENT_BIN` — optional path to `sarv-agent`.
- `SARV_GRAPH_CLIENT_ID`, `SARV_GRAPH_TENANT`, `SARV_GRAPH_REDIRECT_URI` — owner Graph app. Absent means the connector stays off.
- `VITE_QA_URL`, `VITE_QA_TOKEN` — browser test adapter only. Not used by the installed app.
- `SARV_QA_DATA_DIR`, `SARV_QA_TOKEN`, `SARV_QA_SEED`, `SARV_QA_PORT` — QA process only.
- `WEBKIT_DISABLE_DMABUF_RENDERER` — Linux webview. The app sets it when unset.

## Rollback

Keep a copy of `sarv.sqlite3` before opening a new build. JSON backup and restore remain the in-app path and do not include the shop lock. To undo a shop password, use the recovery code or restore a SQLite copy taken before bootstrap. Do not delete posted invoices to correct a sale; use the return and reversal commands.
