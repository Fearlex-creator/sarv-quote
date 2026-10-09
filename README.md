# Sarv / سرو

Local-first Persian accounting for individuals and businesses. The desktop interface uses Tauri 2, React 19, TypeScript, official shadcn components, Tailwind, and pnpm. The original Rust accounting, quotation, procurement, inventory, sales, and returns engine remains the source of business rules and persisted books.

This is a **work-in-progress cloud handoff**, not a completed production release. See [the handoff](docs/CLOUD-HANDOFF.md) for the confirmed scope, current implementation, and outstanding work. The original native application documentation is preserved in [the legacy README](docs/legacy-quotation-readme.md).

## Local development

Install Node >=20.9, pnpm, Rust, and the Tauri 2 prerequisites for your platform. Linux needs WebKitGTK 4.1, GTK 3, Pango/Cairo and development packages; the preserved native UI additionally needs GTK 4/libadwaita.

```bash
pnpm install
pnpm --dir web install
pnpm tauri dev
```

The desktop discovers an existing project `data/sarv.sqlite3`; otherwise it uses a Sarv directory within its application data directory. Set `SARV_DATA_DIR` explicitly to select the original store. New workspaces live in independent `workspaces/<uuid>` directories. Existing stores are preserved.

For a browser preview using the real Rust adapter and an isolated temporary store:

```bash
pnpm preview:dev
```

Optional environment variables: `SARV_PREVIEW_DATA_DIR`, `SARV_PREVIEW_WEB_PORT` (default 5178), and `SARV_PREVIEW_LEDGER_PORT` (default 4783). The preview is localhost-only and development-token protected; it is not the future team server.

## Build

```bash
pnpm lint
pnpm build
pnpm tauri build
```

The preserved native app can be run with `pnpm gtk`. Rust checks live in `crates/sarv-quote/tests`; the desktop adapter tests are `desktop_workflows.rs`.

## One ledger

Canonical money is exact integer Rials; Toman display divides by ten without losing fractional Tomans. React receives money as strings. The existing Rust core validates balanced double-entry journals, fiscal-period locks, immutable postings, duplicate request keys, stock availability, weighted-average costs, and original-cost returns. Financial mutations use `Store::change` with validation and SQLite generation checks. Corrections use reversal journals or the typed goods-return workflows.

The active Tauri interface uses `desktop_view` / `desktop_mutate` over `sarv-quote::desktop::DesktopHost`, rather than the separate six-screen experimental ledger. Its coarse views are cached in React Query; lists use paged reads and virtualized rows. The experimental crate and frontend remain in the repository as inactive earlier work.

The original store persists a validated versioned snapshot in SQLite, with a 64 MiB backup limit. This release has not established suitability for large or concurrent enterprise books. A shared SQLite file is not the team architecture.

## Confirmed roadmap

1. Restore the original workflows in a Persian RTL/Jalali shadcn desktop, with individual/service/trading onboarding and a resumable setup checklist.
2. Add the remaining workflow depth, imports, exports, performance validation and production hardening on the same ledger.
3. Add an optional team server, authorization and enterprise workflows behind the existing transport boundary.
4. Add multi-currency accounting with explicit exchange-rate postings. Rial/Toman comes first.

Later phases add screens and services on top of the same ledger; they must preserve earlier books and history. Payroll, live bank feeds and payment processing remain outside the current scope.
