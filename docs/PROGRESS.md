# Progress

Branch `cursor/retail-excel-pos-8dc2`. The engine stays one SQLite book in `crates/sarv-quote`. Schema remains 7. New shop data is an optional `retail` object that serializes only when it has rows, so older books stay byte-identical when the object is empty. The shop lock lives in a separate `app_security` table and is not part of the JSON financial backup. SQLite is not encrypted at rest. The password gates commands, not the file on disk.

## Done in this slice

- Shop lock with Argon2id, roles, session cookies stored only as hashes, lockout, and a one-time local recovery code. No default password. The installed app asks for a password before showing books when it is not the QA preview.
- Agent access after a lock requires an owner grant of readonly, cashier, or accountant. A legacy token with an empty role does not read a locked shop. Grants cannot be owner. The accountant grant cannot mint Graph credentials or change the agent.
- Excel schema `sarv-excel-1` for products, customers, suppliers, and opening stock. Preview, row errors, all-or-nothing commit, replay by request key. Opening-stock line ids include the request key so a second company can commit the same sheet shape. Trial balance and sales register export. Formulas, macros, external links, and zip expansion past the declared and inflated caps are refused.
- Optional Microsoft Graph connector: PKCE, state, server-side refresh, revoke, and one item id. The requested scope is `Files.SelectedOperations.Selected`. `Files.ReadWrite.Selected` is not requested, because Microsoft limits that name to Office file handlers. A typed item id is not a file grant. The connector stays inactive until `SARV_GRAPH_CLIENT_ID` is set. `live_verified` is false. Local mocks cover the contract, including 403, 429, a mismatched state, and a private redirect. No Microsoft app was registered and no call to Microsoft was made.
- Till open/close, held carts, a touch keypad, and a POS screen that sells through the existing draft, invoice, stock, and payment commands. Cash and bank amounts are labeled as manual records. Checkout stays off while the browser is offline.
- Persian copy, a web app manifest, and thermal print CSS for the till receipt.
- The shadcn CLI is no longer a production dependency. The theme stylesheet is vendored.
- Requirements, Excel, access, POS, and release-gate notes in `docs/`.

## Still open

- Microsoft 365 sign-in is implemented and covered by a local mock. It stays `not_configured` until the owner sets `SARV_GRAPH_CLIENT_ID`. No call to Microsoft's service was made.
- Offline POS is refused. There is no IndexedDB sale queue.
- Till variance is recorded on the shift and is not a journal.
- Opening stock debits inventory and credits GRNI. It does not post equity and it does not create a supplier bill.
- JSON backup does not restore the shop lock. Copying the SQLite file does.
- Tafsili is not in this build. It would change journal-line identity and audited replay, so it is not a spare column. Moadian e-invoice submission, payroll runs, live bank feeds, and card terminals are not in this build. Law data stays unreviewed.
- One local profile. Roles apply inside that profile. This is not multi-shop isolation.
- Historical M3/M4/M5 executables are not in the git tree. They were built, unchanged, in an official Ubuntu 26.04 chroot (libadwaita 1.9.1, rustc 1.93.1) and the three ignored schema-rejection tests passed when those binaries were supplied. The default suite still ignores them. Hashes are in `docs/RELEASE-GATES.md`.
- Quote and invoice PDF money is checked by the extracted Persian amount and by the glyph box, not by counting antialiased ink. See `docs/RELEASE-GATES.md`.
- No `v*` tag was pushed. `release-windows.yml` was not dispatched, because that workflow publishes a release. `.github/workflows/windows-build.yml` builds this branch and uploads an artifact only. Dispatching it returned HTTP 404 because the file is not on the default branch yet.

## Evidence

- `cargo test -p sarv-quote --offline`: 276 tests listed, 273 passed, 3 ignored (`SARV_M3_COMPAT_BINARY`, `SARV_M4_COMPAT_BINARY`, `SARV_M5_COMPAT_BINARY`), 0 failed.
- `pnpm test:pdf --offline --no-fail-fast`: 316 passed, 0 failed, 3 ignored.
- `cargo audit`: no advisories after calamine 0.38, zip 9.4.2, and rust_xlsxwriter 0.102.
- `pnpm --dir desktop audit --prod`: no known vulnerabilities. `pnpm --dir desktop audit`: the high `zod` advisory is gone after the pnpm override to zod 4.6.7. One moderate remains (`minimatch` through ESLint, GHSA-g3hv-75wg-m249). ESLint is a dev tool. The override was not applied to minimatch, and audit thresholds were not changed.
- `pnpm --dir desktop run check`, `lint`, and `test:unit` (14 passed).
- `pnpm --dir desktop test` (builds the QA adapter, then Playwright): 78 passed, 0 failed, in one invocation. 26 Chrome, 26 Firefox, 26 WebKit.
- Linux package after this tree: `/opt/cursor/artifacts/sarv-linux/Sarv_0.1.0_amd64.deb` (sha256 `3aaa9c62594496f14f2113f745c88f78775e3375f5ada46416302f0d8db36708`). Extracted binary sha256 `2a1e4cb99d3ff0fa25f54b526755a55cfa6c4b4cb8fe79ea2882e221c32b7a7e`. First-run screenshot: `/opt/cursor/artifacts/sarv-linux/first-run.png`.
- Screenshots: `/opt/cursor/artifacts/sarv-retail/01-excel-desktop.png`, `02-excel-opening-preview.png`, `03-pos-desktop.png`, `04-pos-receipt.png`, `05-pos-mobile-rtl.png`.
- PDF rasters: `/opt/cursor/artifacts/sarv-pdf/native-page.png`, `/opt/cursor/artifacts/sarv-pdf/poppler-page.png`, `/opt/cursor/artifacts/sarv-pdf/cell-native.png`, `/opt/cursor/artifacts/sarv-pdf/cell-poppler.png`.
