# Release gates

No tag was created. The existing GitHub release `v0.1.0` points at main `f5888a16cc675eaaf58ca165363a39513b6695ac`, which does not contain this branch. `release-windows.yml` was not dispatched, because a manual run creates or updates that tag's GitHub release. The full frontend checklist (81 boxes) and the accounting checklist (156 boxes) stay unticked. An optional journal dimension is not a tafsili chart. There is no automated legal filing.

## Artifact identity

The Linux package was built from code `20fd9409b2f84f41810a8638d36664b786c9f42c`. Later commits, including documentation revision `2bb57d6b69d21c03d4c0b7f1f367984d903e8a8b` and this file, do not rebuild it. `sha256sum` of both `desktop/src-tauri/target/release/bundle/deb/Sarv_0.2.0-rc.1_amd64.deb` and `/opt/cursor/artifacts/sarv-linux/Sarv_0.2.0-rc.1_amd64.deb` is `4a32e020a259cb23d99bf3ed152a4f90482fe0c6871f86004a9e0683d1ff562f`. `sha256sum` of the extracted `usr/bin/sarv-desktop` is `cfb0e1c99f29cd6d7da24fcbf4bd22aa9f5a52474a6fa4083cd974273960de26`. An earlier note used `0617929f7aeaf0dba57baee5cec41453dba90c5b164efa06e424f5ff3692d708` and `f85737b91e4710881d1dbaea388444b065747d2401d4d3855fc21f0f9315df6f`. Those are the package from before the last rebuild. They are not this artifact.

The native exercise extracted that deb and ran the binary under Xvfb with `GTK_USE_PORTAL=0` and a session bus that does not start the document portal. `fuse3` and the xdg desktop portals on that host were unpacked and not configured. This was not `apt install` of the deb on a desktop with the portal configured.

The Windows zip is not code-signed (`signed` is false). GitHub Actions compiled it and uploaded it. The `.exe` was not launched, so the Windows runtime is untested. On code `20fd940`, the push artifact is https://api.github.com/repos/Fearlex-creator/sarv-quote/actions/artifacts/13629223688/zip (run https://github.com/Fearlex-creator/sarv-quote/actions/runs/42511252121) and the pull-request artifact is https://api.github.com/repos/Fearlex-creator/sarv-quote/actions/artifacts/13630255109/zip (run https://github.com/Fearlex-creator/sarv-quote/actions/runs/42511256432). Documentation commit `2bb57d6` compiled the same unsigned zip again: push https://github.com/Fearlex-creator/sarv-quote/actions/runs/42512580753 artifact https://api.github.com/repos/Fearlex-creator/sarv-quote/actions/artifacts/13630411171/zip, pull request https://github.com/Fearlex-creator/sarv-quote/actions/runs/42512584773 artifact https://api.github.com/repos/Fearlex-creator/sarv-quote/actions/artifacts/13630495198/zip. Neither pair is a public release.

## Green on this branch

- `cargo test -p sarv-quote --offline --no-fail-fast`: 278 listed, 275 passed, 3 ignored, 0 failed. The three ignored tests are not part of that pass count.
- Desktop typecheck, lint, and 14 unit tests.
- `cargo audit` reports no advisories on the lockfile that uses calamine 0.38, zip 9.4.2, and rust_xlsxwriter 0.102.
- `pnpm --dir desktop audit --prod` reports no known vulnerabilities. The shadcn CLI is not a production dependency.
- Shop-lock denial for a cashier journal, a signed-out workspace, and a legacy agent token. An owner-granted accountant token can read and cannot create users.
- Graph connector contract against a local mock. `live_verified` is false.
- Linux package `0.2.0-rc.1`, built from `20fd940` and not rebuilt afterwards. Path `/opt/cursor/artifacts/sarv-linux/Sarv_0.2.0-rc.1_amd64.deb`, sha256 `4a32e020a259cb23d99bf3ed152a4f90482fe0c6871f86004a9e0683d1ff562f`. Extracted `usr/bin/sarv-desktop` sha256 `cfb0e1c99f29cd6d7da24fcbf4bd22aa9f5a52474a6fa4083cd974273960de26`. This is not the published `v0.1.0` release. AT-SPI drove the extracted binary under Xvfb with `GTK_USE_PORTAL=0` on a disposable `/tmp` profile. That is not a clean desktop installation. Password setup, company creation, Excel product and opening-stock commit, a till sale, backup, sign-out, and restore into a second locked profile went through that UI. The production JavaScript has no QA URL. See `docs/PROGRESS.md`.
- `pnpm test:pdf --offline --no-fail-fast`: 319 passed, 0 failed, 3 ignored. `wrong_missing_and_clipped_money_fail_the_independent_check` replaces and deletes the extracted amount and whites out the right 24px of the glyph box. Those fixtures fail the text and geometry checks. The 4px threshold was not loosened. The renderer was not changed.

## PDF money cells

`final_raster_has_every_monetary_cell_and_matches_independent_pdf_raster` used to count pixels whose channels were all below 180 and require the poppler count to stay within 20% of cairo. On this machine the first amount cell was 194×226 in both rasters and both showed `۱۵٬۶۲۵.۴`. Cairo counted 186 ink pixels and poppler 24.02 counted 265. Channel-max bands were cairo `[0, 47, 46, 60, 33]` and poppler `[0, 119, 64, 57, 25]` for the bins below 40, 40–79, 80–119, 120–159, and 160–179. Neither raster had a pixel below 40. The extra poppler ink is mid-gray stem weight, not a missing or extra digit. Crops: `/opt/cursor/artifacts/sarv-pdf/cell-native.png` and `/opt/cursor/artifacts/sarv-pdf/cell-poppler.png`.

The ink ratio is no longer the assertion. The test now requires each line price, line total, discount, and grand total from `MoneyUnit::display` to appear in `pdftotext`, and the glyph box of pixels darker than 160 to agree within 4px. The measured boxes were cairo `(126, 25, 57, 13)` and poppler `(123, 22, 60, 13)`: the far edge stays put and the heavier stem grows the box by 3px. A dropped digit in that 57px run is wider than 4px. Draft and approved pages from the same cairo rasterizer must still match exactly, and each cell must still contain ink. A separate test feeds a wrong expected amount, deletes the real amount from the extracted text, and clips the glyph box. Those checks fail. The renderer was not changed.

## Not green

| Gate | What is missing |
| --- | --- |
| Dev-tool advisory | `pnpm --dir desktop audit` still reports moderate `minimatch` via ESLint (GHSA-g3hv-75wg-m249). ESLint 10 is the linter. Forcing minimatch 11 was not done. The high `zod` finding is patched. Thresholds were not raised. |
| Historical binaries in CI | The default `cargo test` still ignores the three compat tests, because the binaries are not in git and an Ubuntu 24.04 runner cannot link libadwaita 1.9. A local Ubuntu 26.04 chroot did build the unchanged commits and the three tests passed there. See below. |
| Graph against Microsoft | The connector and a local mock pass. A real tenant still needs the owner to register an app and set `SARV_GRAPH_CLIENT_ID`. No secret is in the repo. |
| Windows signed binary and a launched Windows runtime | The compile-and-upload job passed. The zip is not code-signed, and the executable was not launched. URLs are in Artifact identity above. Permissions are `contents: read`. No `pull_request_target`, secrets, or tag. The earlier `workflow_dispatch` HTTP 404 was the API limitation for a workflow that is not on the default branch, not an approval denial. `release-windows.yml` was not dispatched. |
| Production host | There is no separate server to deploy. The product is the local profile. |

## Browser run

Playwright projects are `chrome` (system Google Chrome), `firefox` (Playwright Firefox 1555), and `webkit` (Playwright WebKit 2370). After the Graph copy change, one invocation of `pnpm --dir desktop test` passed 78 and failed 0 (12.2 minutes).

## Historical schema binaries

The host is Ubuntu 24.04 with libadwaita 1.5.0 and has no Docker. `debootstrap --variant=minbase resolute` from `http://archive.ubuntu.com/ubuntu` produced a chroot with libadwaita 1.9.1-0ubuntu0.2, GTK 4.22.4, and rustc/cargo 1.93.1. The historical sources were not edited. `cargo build --release --locked` succeeded for each commit:

| Commit | Schema era | sha256 of `target/release/sarv-quote` |
| --- | --- | --- |
| `bdc304cb139e3d258b107b9ce18c7f4682ec5a89` | 4 | `d7b230635208a5d92605f0db8d754432dbb9a71227266622b4667f59689079bf` |
| `24ae57bc179336ad21d7d1d52501f49e80f3330b` | 5 | `406a0beee684c3a1a521d2918cc4e215855950f8d9ac6cec8bbe7877fdfa068b` |
| `eb8f8c29648fd4a206a26d65ae4eaca5cf945a1b` | 6 | `502d77fa11f067575553329815da97a028d4f2a43d0996134cc7f5f95175bc01` |

The binaries need glibc 2.43, so they were executed with the chroot's `ld-linux-x86-64.so.2` and library path. With that loader, `actual_m3_binary_rejects_current_schema_without_modifying_committed_row`, `actual_m4_binary_refuses_current_schema_without_changing_row`, and `actual_m5_binary_refuses_schema7_without_modifying_committed_row` passed. They stay `#[ignore]` in the default suite.

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
