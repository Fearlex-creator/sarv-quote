# Sarv / سرو

Local-first Persian accounting for individuals and businesses. The desktop interface uses Tauri 2, React 19, TypeScript, official shadcn components, Tailwind, and npm. The original Rust accounting, quotation, procurement, inventory, sales, and returns engine remains the source of business rules and persisted books.

The primary desktop lives in `desktop/`: a minimal native-style shell, dense financial tables, first-class Persian RTL, independent language/digit/calendar settings, and opt-in local CLI/MCP agents. See [desktop development](desktop/README.md), [agent access](docs/tauri/AGENT-ACCESS.md), and [UI verification and limitations](docs/tauri/NATIVE-UI-REDESIGN.md). This remains an implementation checkpoint, not a production release. The earlier web/Tauri interface and its [cloud handoff](docs/CLOUD-HANDOFF.md) are preserved, alongside [legacy GTK documentation](docs/legacy-quotation-readme.md).

## Local development

Install Node >=20.19 (or >=22.12), npm, Rust, and the Tauri 2 prerequisites for your platform. Linux needs WebKitGTK 4.1, GTK 3, Pango/Cairo and development packages; the preserved native UI additionally needs GTK 4/libadwaita.

```bash
npm ci --prefix desktop
npm run dev
```

The primary desktop uses `ir.sarv.accounting/books` in the application data directory. Set `SARV_DATA_DIR` explicitly to open an existing Sarv profile. It uses the original validated SQLite store; keep backups and avoid simultaneous editing across interfaces.

For the earlier preserved web interface’s browser preview (install its pnpm dependencies first):

```bash
pnpm preview:dev
```

Optional environment variables: `SARV_PREVIEW_DATA_DIR`, `SARV_PREVIEW_WEB_PORT` (default 5178), and `SARV_PREVIEW_LEDGER_PORT` (default 4783). The preview is localhost-only and development-token protected; it is not the future team server.

## Build

```bash
npm run lint
npm run build
npm run desktop:build -- --no-bundle
```

The preserved GTK app can be run with `cargo run -p sarv-quote-gtk --release`. Rust checks live in `crates/sarv-quote/tests`; the desktop bridge and local agent tests are `desktop_bridge.rs` and `agent_access.rs`. Use `cargo test -p sarv-quote` for the engine and `npm run test:desktop` with the isolated QA adapter for the desktop workflows.

## One ledger

Canonical money is exact integer Rials; Toman display divides by ten without losing fractional Tomans. React receives money as strings. The existing Rust core validates balanced double-entry journals, fiscal-period locks, immutable postings, duplicate request keys, stock availability, weighted-average costs, and original-cost returns. Financial mutations use `Store::change` with validation and SQLite generation checks. Corrections use reversal journals or the typed goods-return workflows.

The primary Tauri interface invokes `sarv-quote::desktop::dispatch` and `AgentService` over the original store. The earlier `DesktopHost`, `web/`, root `src-tauri/`, and experimental accounting crate remain available; `npm run legacy:tauri -- dev` launches the earlier shell. Both adapters use the same Rust accounting kernel.

The original store persists a validated versioned snapshot in SQLite, with a 64 MiB backup limit. This release has not established suitability for large or concurrent enterprise books. A shared SQLite file is not the team architecture.

## Confirmed roadmap

1. Restore the original workflows in a Persian RTL/Jalali shadcn desktop, with individual/service/trading onboarding and a resumable setup checklist.
2. Add the remaining workflow depth, imports, exports, performance validation and production hardening on the same ledger.
3. Add an optional team server, authorization and enterprise workflows behind the existing transport boundary.
4. Add multi-currency accounting with explicit exchange-rate postings. Rial/Toman comes first.

Later phases add screens and services on top of the same ledger; they must preserve earlier books and history. Payroll, live bank feeds and payment processing remain outside the current scope.
