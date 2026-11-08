# Spec coverage

How `ACCOUNTING-SPEC.md` and `FRONTEND-UI-GUIDE.md` map onto Sarv today. Written 2026-11-03 from a
code survey, updated after the second pass. **No checklist box in the two spec files is ticked:**
the spec says to tick an item only when it is merged with passing tests *and fully matches the item
as worded*, and no item meets that bar yet. Partial work is listed here instead.

Legend: **Done** = implemented and tested · **Partial** = some of the item · **Present** = exists in
Sarv in its own form · **Not started** · **Deferred** = decided not to do now (ADR).

2026-11-08 addendum: shop lock, `.xlsx` import/export, and a till that posts through the existing
sales kernel are covered by `tests/shop_excel_pos.rs` and `desktop/tests/retail-excel-pos.spec.ts`.
The historical checklist below is unchanged. Tafsili, Moadian e-invoicing, payroll runs, and a live
Microsoft 365 connection are still not done. See `docs/REQUIREMENTS-MATRIX.md`.

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
| E2E harness that works out of the box (`pnpm --dir desktop test`) | `playwright.config.ts` | 26 Chrome tests on 2026-11-08, including Excel and the till |
| Licence AGPL-3.0-only for the core (ADR-0006) | `LICENSE`, `Cargo.toml` | — |

Run on 2026-11-08: `cargo test -p sarv-quote --offline` (272 listed, 3 ignored historical-binary tests, 0 failed; PDF feature not re-run) · `pnpm --dir desktop run check` ·
`run test:unit` (14) · `run lint` · `run test` (26 Chrome tests).

## Accounting spec

| § | Area | Status |
| --- | --- | --- |
| 3.1 | Jalali calendar, fiscal year | **Present** (`calendar.rs`, years 1200–1499, tested). Spec's 1300–1500 golden table not matched |
| 3.2 | COA group → kol → moein, **tafsili** levels | **Not started** (no tafsili/dimension model). Largest structural gap |
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
| 5.10–5.17 | Payroll, assets, FX, projects, manufacturing, POS, budgets, consolidation | Not started (single currency) |
| 5.18 | Reports | **Partial**: trial balance, profit & loss, **balance sheet, AR/AP aging**, position, party balances, control reconciliation, CSV. Cash flow, drill-down chain, saved views, scheduled reports not started |
| 5.19–5.20 | Approvals, OCR, AI | Not started (an AI-suggest component exists in the UI) |
| 5.21 | Import / export | **Partial**: opening-balance CSV, JSON backup + restore preview, CSV/PDF export. Hesabfa/Sepidar/Holoo importers, ledger-text export not started |
| 5.23 | Audit, backup | **Present**: audit events, rolling backups, restore preview, integrity checks |

## Frontend guide

| § | Area | Status |
| --- | --- | --- |
| 1 | Stack | **Present** with differences: Vite/React/TS strict, shadcn-style UI, Tailwind v4, cmdk. No TanStack Router/Query/Table, react-hook-form/zod, i18next (a custom fa/en dictionary is used), Storybook (ADR-0002) |
| 2 | Onboarding | **Present** (setup wizard with opening-balance confirmation). Not matched step-for-step |
| 3 | App shell | **Present**: collapsible sidebar, command menu, dark mode. Company switcher, FY selector, notifications, sync indicator, quick-create chords (`N` then letter) differ: Sarv uses Ctrl/⌘+Alt+key |
| 4 | Dashboard | **Partial** (overview) |
| 5 | Module screens | **Partial**: sales, purchases, payments, banking, inventory, accounting, reports. No cheques, payroll, assets, Moadian screens |
| 6–7 | Close, locks, audit, sharing | **Partial**: period lock, backups. No year-end wizard, approvals, roles, share links, comments |
| 8 | Design system | **Present** under ADR-0001. Density toggle not started |
| 9 | Components | **Partial**: money input, Jalali date picker, data grid, status states. No `EntryGrid` with Excel paste/undo, account/tafsili pickers |
| 10 | State layer | Different (single `command()` bridge). Transport abstraction for web build not started |
| 11–12 | Offline/sync UX, accessibility, shortcuts | Shortcuts partial; sync UX not started; axe checks not set up |
| 14 | Checklist | 0 of 81 ticked; see above for partial work |

## Open items and next slices
1. **Statutory values**: someone qualified must review every `verify = true` value and the spec's §9 "Open verify
   items", then the owner flips `reviewed` (ADR-0004). Until then nothing may feed a real invoice, payslip or filing.
2. **Tafsili levels and dimensions** (spec §3.2): the largest structural gap and the foundation for most
   Iranian reports. Needs its own design pass (it touches the chart, journal lines and the audited replay).
3. **Accounting-heavy save cost** (ADR-0007 "Known limit"): the fast path still re-validates the whole accounting
   group on an accounting write; validating only appended audit events needs replay-state caching.
4. **Cheque extras**: cheque books, batch actions, due-date calendar, dashboard card.
5. **Moadian tax-invoice aggregate** (outbox, provider adapters, wizards) and payroll runs, which build on `iran::*`.
