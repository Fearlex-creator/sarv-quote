# Progress

Branch `cursor/retail-excel-pos-8dc2`. The engine stays one SQLite book in `crates/sarv-quote`. Schema remains 7. New shop data is an optional `retail` object that serializes only when it has rows, so older books stay byte-identical when the object is empty. The shop lock lives in a separate `app_security` table and is not part of the JSON financial backup.

## Done in this slice

- Shop lock with Argon2id, roles, session cookies stored only as hashes, lockout, and a one-time local recovery code. No default password.
- Excel schema `sarv-excel-1` for products, customers, suppliers, and opening stock. Preview, row errors, all-or-nothing commit, replay by request key. Trial balance and sales register export. Formulas, macros, and oversized zip entries are refused.
- Till open/close, held carts, and a POS screen that sells through the existing draft, invoice, stock, and payment commands. Cash and bank amounts are labeled as manual records.
- Persian copy for the new screens, a web app manifest, and thermal print CSS for the till receipt.
- Requirements, Excel, access, POS, and release-gate notes in `docs/`.

## Still open

- Microsoft 365 / Graph is `not_configured`. No client id is stored and no token client is shipped.
- Offline POS is refused. There is no IndexedDB sale queue.
- Till variance is recorded on the shift and is not a journal.
- Opening stock debits inventory and credits GRNI. It does not post equity and it does not create a supplier bill.
- JSON backup does not restore the shop lock. Copying the SQLite file does.
- The local agent token, when the owner turns it on, still bypasses the shop lock. That is a separate owner credential.
- Tafsili, Moadian e-invoice submission, payroll runs, live bank feeds, and card terminals are not in this build.
- Firefox and WebKit were not installed in the verification environment. Chrome was.
- The Windows release workflow was not dispatched and no `v*` tag was pushed.

## Evidence

- `cargo test -p sarv-quote --offline`: 272 tests listed, 3 ignored because they need retained historical binaries (`SARV_M3_COMPAT_BINARY`, `SARV_M4_COMPAT_BINARY`, `SARV_M5_COMPAT_BINARY`), 0 failed.
- `pnpm --dir desktop run check`, `pnpm --dir desktop run lint`, `pnpm --dir desktop run test:unit` (14).
- `pnpm --dir desktop test`: 26 Playwright tests on Chrome. The retail flow is `desktop/tests/retail-excel-pos.spec.ts`.
- Screenshots from that flow: `/opt/cursor/artifacts/sarv-retail/`.
