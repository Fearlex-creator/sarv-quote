# Sarv cloud handoff — 2026-10-10

The user requested committing and pushing all current work for cloud continuation, without further tests. This snapshot includes the preexisting crate reorganization and earlier Tauri implementation. Do not treat the rebuild as finished or undo the preserved Rust workflows.

## Confirmed product choices

- Sarv / سرو branding; official shadcn components throughout the frontend.
- Persian RTL and Jalali input/display by default; English/LTR available.
- Tailored individual/service/trading onboarding plus a resumable setup checklist.
- Existing / new / import paths; preview before imports; preserve original books.
- Restore products, quotations, procurement, stock, sales, returns and reports using the original Rust core.
- Exact Rial storage and Toman display first; multi-currency later.
- Local desktop first; optional team server and enterprise permissions later.
- pnpm throughout.

## Current code

- `crates/sarv-quote/src/desktop.rs`: original-store desktop adapter, independent workspace host, onboarding, exact string views, resource creation, combined purchase/receive/bill and sales/delivery/invoice commands, partial goods returns, stock transfers, wallets, manual reversals, period locks, import previews and open exports.
- `src-tauri/src/{lib,commands}.rs`: active commands now use the original desktop host. The separate accounting database was removed from the active command surface.
- `web/src/desktop/`: new official-shadcn shell, wizard, forms, virtual lists, charts, accounts and settings. Main/App now use this frontend.
- Official shadcn RTL migration was run. The calendar adds the official DayPicker Persian package; core Rust validates Jalali civil dates.
- Original GTK and Rust workflows remain in `crates/sarv-quote-gtk` and `crates/sarv-quote`; PDF code now imports Pango/GLib directly to keep the core independent of GTK.
- Earlier six-screen experimental implementation remains inactive in `crates/free-accounting-core` and `web/src/accounting`.

## Verified before the handoff request

An earlier revision of the adapter passed three integration tests: exact purchase → sale → partial sales return with balanced ledger and durable restart; Persian-digit wallet income, reversal and fiscal lock; and independent workspace preservation. Frontend lint and production build passed before the last workspace-host and import-validation follow-ups. Browser checks showed Persian RTL onboarding, a Jalali date picker, and a reviewed expense posting into the original ledger with updated dashboard balances.

Full restored tests initially failed to compile because the preexisting crate move left relative fixture paths pointing at the former root. The docs/demo paths have been adjusted. No final all-suite, native launch, mobile or production verification is claimed. The most recent edits were not reverified after the user requested an immediate commit/push.

## Continue here

- Re-run compilation/lint/build and the restored Rust suite when authorized in the cloud. Validate the latest workspace selector, backup checksum handling, and purchase-return source mapping.
- Update the debug-only native IPC smoke script: it still refers to the retired `accounting_view` / `accounting_mutate` commands.
- Complete CSV catalog/contact imports, PDF exports in the new shell, quotation revision editing, partial order/receipt workflows, accounting setup/opening depth, and detailed contact history.
- Finish personal-mode terminology, localized civil-date defaults, 320 px / keyboard / reduced-motion checks, error handling and performance measurement.
- The combined buy/sell forms currently post complete goods chains; the original Rust partial operations remain available but need deeper UI wiring.
- Existing quotation-only books without an accounting company need a smooth bridge in onboarding; preserve their catalogs and approved quotes.
- The original snapshot store has a 64 MiB backup limit. Do not claim large-enterprise scale or simultaneous team access without a measured storage/migration/server design.

## Local QA isolation

Browser preview during the handoff used `/tmp/sarv-restored-qa-20261010`, web port 5182 and ledger port 4787. Do not confuse its synthetic records with real user books. Standard preview uses its own temporary directory. Product data and dependency/build caches are ignored by Git.
