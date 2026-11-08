# Release gates

No tag was created and the Windows workflow was not started. Publishing would attach a GitHub release before the gates below are green.

## Green on this branch

- Engine tests against disposable SQLite profiles, including migration fixtures already in the crate.
- Desktop typecheck, lint, 14 unit tests, production `tsc` is part of `pnpm --dir desktop run build` when that command is run.
- Chrome Playwright, including login-free seeded books and one new company that imports `.xlsx`, opens a till, sells, and checks the trial balance.
- Shop-lock denial for a cashier journal and for a signed-out workspace.

## Not green

| Gate | What is missing |
| --- | --- |
| Firefox and WebKit | Not installed here. Do not treat Chrome as all engines. |
| Tauri bundle | WebKitGTK was not used to build a desktop package in this environment. |
| Windows artifact | `release-windows.yml` needs a `v*` tag or `workflow_dispatch`. Neither was run. |
| Graph sync | Owner must register an application and supply client id, tenant, and redirect. No secret is in the repo. |
| Historical binary migration proofs | Ignored tests need `SARV_M3_COMPAT_BINARY`, `SARV_M4_COMPAT_BINARY`, and `SARV_M5_COMPAT_BINARY`. |
| PDF feature tests | `pnpm test:pdf` needs cairo/pango. Not run in this pass. |
| Production host | There is no separate server to deploy. The product is the local profile. |

## Environment names

These are names only. Values stay on the machine.

- `SARV_DATA_DIR` — profile directory. Default is the app data dir.
- `SARV_AGENT_BIN` — optional path to `sarv-agent`.
- `VITE_QA_URL`, `VITE_QA_TOKEN` — browser test adapter only. Not used by the installed app.
- `SARV_QA_DATA_DIR`, `SARV_QA_TOKEN`, `SARV_QA_SEED`, `SARV_QA_PORT` — QA process only.
- `WEBKIT_DISABLE_DMABUF_RENDERER` — Linux webview. The app sets it when unset.

## Rollback

Keep a copy of `sarv.sqlite3` before opening a new build. JSON backup and restore remain the in-app path and do not include the shop lock. To undo a shop password, use the recovery code or restore a SQLite copy taken before bootstrap. Do not delete posted invoices to correct a sale; use the return and reversal commands.
