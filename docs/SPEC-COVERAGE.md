# Spec coverage

How `ACCOUNTING-SPEC.md` and `FRONTEND-UI-GUIDE.md` map onto Sarv today. Written 2026-11-03 from a
code survey, not from exercising every screen. **No checklist box in the two spec files is ticked:**
the spec says to tick an item only when it is merged with passing tests *and fully matches the item
as worded*, and no item meets that bar yet. Partial work is listed here instead.

Legend: **Done** = implemented and tested · **Partial** = some of the item · **Present** = exists in
Sarv in its own form · **Not started** · **Deferred** = decided not to do now (ADR).

## Delivered in this pass (all tested)

| Item | Where | Tests |
| --- | --- | --- |
| National ID, legal ID, postal code, Sayad ID shape, Sheba mod-97 + bank name | `crates/sarv-quote/src/iran/{ids,sheba}.rs` | `tests/iran.rs` |
| Same validators in the UI, advisory hints on contact + business forms | `desktop/src/lib/iran-ids.ts`, `components/finance/*-fields.tsx` | `src/lib/iran-ids.test.ts` against the **same fixtures** as Rust |
| Moadian Tax ID (6 + 5 hex date + 10 hex serial + Verhoeff) and Verhoeff | `iran/tax_id.rs` | published Verhoeff vectors, tamper detection on every position |
| Effective-dated law data: VAT 10 % 1403–1405, salary tax 1405, insurance 7/20/3, payroll calc | `iran/law.rs` (**unwired**, `verify` flags, ADR-0004) | hand-calculated goldens |
| Cheque lifecycle state machine with balanced postings (spec §3.6) | `iran/cheque.rs` (**not in ledger yet**, ADR-0005) | every direction × state × action |
| Amount in Persian words, exact to 10^18, never via JS `number` | `desktop/src/lib/words-fa.ts`, shown under the invoice total (fa) | `words-fa.test.ts` incl. sign-loss regression |
| Persian search normalisation (ي/ك, ZWNJ, digits, diacritics) in the command menu | `lib/normalize-fa.ts`, `lib/command-filter.ts` | `normalize-fa.test.ts` |
| Fast incremental desktop builds (`--profile fast`, 6 s vs 97 s) and one frontend build per release | `desktop/src-tauri/Cargo.toml`, `tauri.conf.json`, `package.json` | measured |
| Save-latency probes | `crates/sarv-quote/examples/write_{latency,breakdown}.rs` | ADR-0007 |

Run: `cargo test -p sarv-quote` (235 pass) · `pnpm --dir desktop run check` · `pnpm --dir desktop run test:unit` (14 pass).

## Accounting spec

| § | Area | Status |
| --- | --- | --- |
| 3.1 | Jalali calendar, fiscal year | **Present** (`calendar.rs`, years 1200–1499, tested). Spec's 1300–1500 golden table not matched |
| 3.2 | COA group → kol → moein, **tafsili** levels | **Not started** (no tafsili/dimension model). Largest structural gap |
| 3.3 | VAT, effective-dated tax codes | **Partial**: rate data in `iran::law`; engine uses a per-quote basis-points setting |
| 3.4 | Moadian e-invoicing | **Partial**: Tax ID only. No tax-invoice aggregate, outbox, provider, wizards |
| 3.5 | Seasonal (TTMS) report | Not started |
| 3.6 | Sayad cheques | **Partial**: state machine + ID shape. Opening cheques exist in the ledger. No register/lifecycle (ADR-0005) |
| 3.7 | Payroll, insurance, salary tax | **Partial**: calculator only. No employees, runs, DISK `.dbf`, payslips |
| 3.8 | ID validation | **Partial**: validators done; economic code not; advisory only (ADR-0003). Withholding, multi-rate FX not started |
| 4.1–4.3 | 26-crate workspace, axum/sqlx/Postgres, specta | **Deferred** (ADR-0002) |
| 4.4 | Ledger invariants | **Present** in the engine's own design: balanced entries, immutable posted entries via reversal, period locks, audited replay. Not matched: hash chain per entry, gap-free per-FY numbering (not verified), tafsili/dimension sets |
| 4.5 | Sync / Team mode | Not started |
| 4.6–4.7 | HTTP API, roles, 2FA | Not started. Local agent access (MCP/CLI) exists |
| 5.4–5.6 | Sales, purchases, inventory | **Present**: quotations/drafts, sales, returns, orders, bills, goods receipt, moving-average stock, transfers, partial billing. FIFO, batches/serials, multi-UoM not started |
| 5.7 | Treasury, reconciliation | **Partial**: cash/bank/transfers/ledger-vs-statement review. Statement import with bank mappers not started |
| 5.8–5.9 | Cheques, tax | see §3.4, §3.6 |
| 5.10–5.17 | Payroll, assets, FX, projects, manufacturing, POS, budgets, consolidation | Not started (single currency) |
| 5.18 | Reports | **Partial**: trial balance, profit & loss, position, party balances, control reconciliation, CSV. Balance sheet, cash flow, aging buckets, drill-down chain not verified/started |
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

## Needs your decision before the next slice
1. **Cheque register in the ledger** (ADR-0005): approve the proposed design, then it is built test-first.
2. **Incremental validation** for save speed (ADR-0007).
3. **Statutory values**: someone qualified must review every `verify = true` value and the spec's §9
   "Open verify items" before any real invoice, payslip or filing relies on them.
4. **Licence** (ADR-0006) and whether "Daftar" or "Sarv" is the long-term name (ADR-0001).
