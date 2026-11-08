# Spec coverage

How `ACCOUNTING-SPEC.md` and `FRONTEND-UI-GUIDE.md` map onto Sarv today. Written 2026-11-03 from a
code survey, updated after the second pass. **No checklist box in the two spec files is ticked:**
the spec says to tick an item only when it is merged with passing tests *and fully matches the item
as worded*, and no item meets that bar yet. Partial work is listed here instead.

Legend: **Done** = implemented and tested · **Partial** = some of the item · **Present** = exists in
Sarv in its own form · **Not started** · **Deferred** = decided not to do now (ADR).

2026-11-08 addendum: shop lock, `.xlsx` import/export, and a till that posts through the existing
sales kernel are covered by `tests/shop_excel_pos.rs` and `desktop/tests/retail-excel-pos.spec.ts`.
The historical checklist below is unchanged. Tafsili as a chart of accounts, Moadian e-invoicing,
payroll runs, and a live Microsoft 365 connection are still not done. See `docs/REQUIREMENTS-MATRIX.md`.

The frontend guide §14 still has 81 unticked boxes. The accounting spec still has its own unticked
boxes (156 `- [ ]` items). None of those boxes is ticked in this branch. A box is ticked only when
the merged code fully matches the item as written. Partial work is not a tick.

What this branch actually has, beyond the table below:

- Shop password, roles, and a first-run screen in the installed app. JSON restore is refused until
  that password exists, and a restore does not replace `app_security`. SQLite is not encrypted at rest.
- Optional journal-line `dimension`. Empty values are omitted, so older books stay compatible, and
  changing a posted line's tag fails replay. This is not kol/moein/tafsili and not a subledger.
- Excel opening stock is inventory and GRNI (goods received, not yet invoiced). It is not an opening
  capital or equity balance.
- Quote and invoice PDF money is tested, including a fixture that fails when digits are replaced,
  removed, or clipped. The renderer was not changed for that.
- POS checkout is online only. There is no offline queue.
- One local profile. Multi-shop isolation is not implemented.
- Microsoft Graph requests `Files.SelectedOperations.Selected`. `live_verified` stays false.

Still not built, and not claimed: statutory filing, payroll runs, fixed assets, foreign currency,
manufacturing, budgets, consolidation, full tafsili levels, multi-shop, and a Windows or macOS
signed installer. The version under test is `0.2.0-rc.1`, which is not the published `v0.1.0` release.

## Delivered (all tested)

| Item | Where | Tests |
| --- | --- | --- |
| Iranian ID validators (national, legal, postal, Sayad shape, Sheba mod-97 + bank name), advisory in the UI | `iran/{ids,sheba}.rs`, `desktop/src/lib/iran-ids.ts` | shared fixtures asserted by Rust and Node |
| Moadian Tax ID (6 + 5 hex date + 10 hex serial + Verhoeff) | `iran/tax_id.rs` | published Verhoeff vectors, tamper detection |
| Effective-dated law data with `verify` **and** an owner `reviewed` gate (all unreviewed); visible Settings → Law data tab | `iran/law.rs`, `features/law-data.tsx` (ADR-0004) | hand-calculated goldens, gate tests, browser test |
| **Cheque register in the ledger**: receive, issue, deposit, clear, bounce, return, void, endorse, opening cheques | `cheques.rs`, `payments.rs`, `procurement.rs`, `accounting.rs`, Payments screen (ADR-0005) | 14 Rust tests + browser test |
| **Incremental validation on save** with full validation at open, restore, backup, period close and a background check after imports (ADR-0007) | `domain.rs`, `store.rs`, `desktop_bridge.rs`, Overview `IntegrityNotice` | 7 agreement/safety-net tests, mutation-checked |
| **Balance sheet** and **receivable/payable aging** (read-only) | `statements.rs`, Reports tabs | 6 hand-calculated tests + browser test |
| Amount in Persian words under the invoice total; Persian search normalisation in the command menu | `lib/words-fa.ts`, `lib/normalize-fa.ts`, `lib/command-filter.ts` | 14 unit tests |
| Fast desktop builds (`--profile fast`, 6 s vs 97 s); one frontend build per release | `desktop/src-tauri/Cargo.toml`, scripts | measured |
| E2E harness that works out of the box (`pnpm --dir desktop test`) | `playwright.config.ts` | 78 tests on 2026-11-08: 26 Chrome, 26 Firefox, 26 WebKit. The shared QA profile stays unlocked, so the backup spec asserts that restore is refused. The packaged webview is the restore that succeeds |
| Licence AGPL-3.0-only for the core (ADR-0006) | `LICENSE`, `Cargo.toml` | — |

Run on 2026-11-08, current tree: `cargo test -p sarv-quote --offline --no-fail-fast` (278 listed, 275 passed, 3 ignored historical-binary tests, 0 failed; the ignored tests are not part of the pass count) · `pnpm test:pdf --offline --no-fail-fast` (319 passed, 0 failed, 3 ignored) · `pnpm --dir desktop run check` · `run test:unit` (14) · `run lint` · `run test` (78: 26 Chrome, 26 Firefox, 26 WebKit).

## Accounting spec

| § | Area | Status |
| --- | --- | --- |
| 3.1 | Jalali calendar, fiscal year | **Present** (`calendar.rs`, years 1200–1499, tested). Spec's 1300–1500 golden table not matched |
| 3.2 | COA group → kol → moein, **tafsili** levels | **Partial**: an optional journal-line `dimension` tag, omitted when empty, copied on reversal, and rejected by replay if changed after posting. Not a kol/moein/tafsili chart and not a subledger |
| 3.3 | VAT, effective-dated tax codes | **Partial**: rate data in `iran::law`; engine uses a per-quote basis-points setting |
| 3.4 | Moadian e-invoicing | **Partial**: Tax ID only. No tax-invoice aggregate, outbox, provider, wizards |
| 3.5 | Seasonal (TTMS) report | Not started |
| 3.6 | Sayad cheques | **Done for the lifecycle**: register, every transition posts balanced entries, replays identically (ADR-0005). Not built: cheque books/leaf ranges, batch actions, due calendar, bank-API plugin, guarantee cheques |
| 3.7 | Payroll, insurance, salary tax | **Partial**: calculator only. No employees, runs, DISK `.dbf`, payslips |
| 3.8 | ID validation | **Partial**: validators done; economic code not; advisory only (ADR-0003). Withholding, multi-rate FX not started |
| 4.1–4.3 | 26-crate workspace, axum/sqlx/Postgres, specta | **Deferred** (ADR-0002) |
| 4.4 | Ledger invariants | **Present** in the engine's own design: balanced entries, immutable posted entries via reversal, period locks, audited replay. Not matched: hash chain per entry, gap-free per-FY numbering (not verified), tafsili/dimension sets |
| 4.5 | Sync / Team mode | Not started |
| 4.6–4.7 | HTTP API, roles, 2FA | Not started. Local agent access (MCP/CLI) exists |
| 5.4–5.6 | Sales, purchases, inventory | **Present**: quotations/drafts, sales, returns, orders, bills, goods receipt, moving-average stock, transfers, partial billing. FIFO, batches/serials, multi-UoM not started |
| 5.7 | Treasury, reconciliation | **Partial**: cash/bank/transfers/ledger-vs-statement review. Statement import with bank mappers not started |
| 5.8–5.9 | Cheques, tax | cheques: see §3.6. Tax: see §3.3–3.5 |
| 5.10–5.17 | Payroll, assets, FX, projects, manufacturing, POS, budgets, consolidation | **Partial for the till only**: one online till posts through the existing sales and payment commands. Payroll, assets, FX, projects, manufacturing, budgets, and consolidation are not built |
| 5.18 | Reports | **Partial**: trial balance, profit & loss, **balance sheet, AR/AP aging**, position, party balances, control reconciliation, CSV. Cash flow, drill-down chain, saved views, scheduled reports not started |
| 5.19–5.20 | Approvals, OCR, AI | Not started (an AI-suggest component exists in the UI) |
| 5.21 | Import / export | **Partial**: opening-balance CSV, `.xlsx` products/customers/suppliers/opening stock, JSON backup + restore preview, CSV/PDF export. Hesabfa/Sepidar/Holoo importers, ledger-text export not started |
| 5.23 | Audit, backup | **Present**: audit events, rolling backups, restore preview, integrity checks |

## Frontend guide

| § | Area | Status |
| --- | --- | --- |
| 1 | Stack | **Present** with differences: Vite/React/TS strict, shadcn-style UI, Tailwind v4, cmdk. No TanStack Router/Query/Table, react-hook-form/zod, i18next (a custom fa/en dictionary is used), Storybook (ADR-0002) |
| 2 | Onboarding | **Present** (setup wizard with opening-balance confirmation). Not matched step-for-step |
| 3 | App shell | **Present**: collapsible sidebar, command menu, dark mode. Company switcher, FY selector, notifications, sync indicator, quick-create chords (`N` then letter) differ: Sarv uses Ctrl/⌘+Alt+key |
| 4 | Dashboard | **Partial** (overview) |
| 5 | Module screens | **Partial**: sales, purchases, payments, banking, inventory, accounting, reports. No cheques, payroll, assets, Moadian screens |
| 6–7 | Close, locks, audit, sharing | **Partial**: period lock, backups, and a shop password with owner/accountant/cashier roles. No year-end wizard, approvals, share links, or comments |
| 8 | Design system | **Present** under ADR-0001. Density toggle not started |
| 9 | Components | **Partial**: money input, Jalali date picker, data grid, status states. No `EntryGrid` with Excel paste/undo, account/tafsili pickers |
| 10 | State layer | Different (single `command()` bridge). Transport abstraction for web build not started |
| 11–12 | Offline/sync UX, accessibility, shortcuts | Shortcuts partial; sync UX not started; axe checks not set up |
| 14 | Checklist | 0 of 81 ticked. The boxes below are the audit, not ticks |

## Frontend checklist, all 81 boxes

None of these boxes is ticked. A tick would mean the merged code matches the box as written. This branch does not.

**Narrower form exists (the box is still unticked):**

- M0 stack: Vite, React, TypeScript, Tailwind v4, a shadcn-style kit, RTL, Vazirmatn. Not the named `DirectionProvider` setup.
- M0 theme tokens and light/dark. Density exists as a setting. A print stylesheet exists for the till receipt, not as a general print system.
- M0 fa/en dictionary and a `dir` switch. Not i18next.
- M0 one `command()` function. In the production bundle it calls `invoke("accounting")` when `window.isTauri` is set, and otherwise throws `desktop_required`. The HTTP QA adapter is compiled out of the production bundle. There is no separate Transport interface or contract test.
- M0 money, Jalali, Persian words, and ID validators have unit tests. They are not the full shared-golden list in the box.
- M1 sidebar, command menu, and shortcuts exist in Sarv's own shape.
- M1 onboarding creates a company, fiscal period, and a zero-balance opening. It is not steps 0–13 as written.
- M1 money input, Jalali date, ID inputs, empty/error states, and toasts exist in part.
- M1 sales, receipts, bills, payments, and returns exist. The till posts a sale and a cash receipt.
- M1 items and a stock quantity exist. Excel can load products and opening stock.
- M1 cheque lifecycle exists in the ledger. The full calendar and batch UI do not.
- M1 journal form, including an optional dimension column. Not a tafsili picker or COA tree editor.
- M1 reports: trial balance, profit and loss, balance sheet, aging, CSV.
- M1 period lock, backups, restore preview. JSON restore does not replace `app_security`.
- M1 shop password and roles. Not the team/invite model.
- M2 law-data screen with unreviewed badges.

**Not built, from the same 81 boxes:**

- specta `bindings.ts`, OpenAPI CI, Storybook, axe, tauri-driver in CI, visual-regression snapshots.
- Company switcher as a multi-company product, FY selector in the top bar, global search, notifications, sync indicator, deep links, single-instance, multi-window.
- Excel mapping memory, Hesabfa/Sepidar/Holoo import, sample-data toggle.
- EntryGrid with Excel paste, undo, and 1,000-line virtualisation. Account, tafsili, party, and item pickers as specified.
- DataTable server pagination, saved views, and URL filters.
- Dashboard cards for cash, AR, AP, cheques, expenses, Moadian, reconciliation, inventory, payroll, approvals, and cash-flow forecast.
- Moadian invoice tab, share links, incoming Moadian inbox, OCR, discount approvals, landed cost, requisitions.
- Warehouses as a full screen, transfers, stock count, batches, serials, bins, multi-UoM.
- Bank statement import, reconciliation workspace, cheque books, cash count, auto-reconcile, cash-flow forecast.
- Moadian queue, VAT worksheet, seasonal TTMS, saved reports, report builder.
- Payroll run, payslips, bank file, DISK. Fixed assets and depreciation.
- Year-end close wizard, amend-with-diff, per-record activity tab, approval inbox, close checklist, consolidation screens.
- Custom role matrix, read-only share links, public invoice page, customer portal, accountant portal, comments.
- Offline banner, sync inbox, field-level conflicts. The till refuses checkout while offline. It does not queue.
- WCAG 2.2 AA evidence and fa/en visual snapshots.
- The same bundle served by a server, tablet layouts as specified, PWA, OIDC.

That is 0 ticked, a narrower slice of the shop-accounting boxes, and the rest not built. Statutory filing, payroll calculations as legal output, and multi-shop isolation are outside this slice.

## Open items and next slices
1. **Statutory values**: someone qualified must review every `verify = true` value and the spec's §9 "Open verify
   items", then the owner flips `reviewed` (ADR-0004). Until then nothing may feed a real invoice, payslip or filing.
2. **Tafsili levels** (spec §3.2): a journal line can carry an optional dimension tag. The kol/moein/tafsili
   chart and subledgers are still not built. Those would be a new account model, not another tag.
3. **Accounting-heavy save cost** (ADR-0007 "Known limit"): the fast path still re-validates the whole accounting
   group on an accounting write; validating only appended audit events needs replay-state caching.
4. **Cheque extras**: cheque books, batch actions, due-date calendar, dashboard card.
5. **Moadian tax-invoice aggregate** (outbox, provider adapters, wizards) and payroll runs, which build on `iran::*`.
