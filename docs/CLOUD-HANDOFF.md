# Sarv current handoff — 2026-10-10

The application has one frontend (`desktop/src`, React/TypeScript/shadcn) and one Rust accounting backend (`crates/sarv-quote`). The only native shell is `desktop/src-tauri` (Tauri 2). Root npm scripts launch, build and check this application.

The GTK4/libadwaita app, duplicate `web/` and root `src-tauri/` shells, experimental `free-accounting-core`, retired preview host, old launch commands and GTK QA scripts have been removed. Their history remains available in Git. Accounting migrations, fixtures, stored preference DTOs, exact money rules, CSV exports and financial history compatibility are retained.

## Build and tests

Install the frontend with `npm ci --prefix desktop`. Run `npm run dev`, `npm run lint`, `npm run typecheck`, `npm run build`, or `npm run desktop:build -- --no-bundle` from the root.

The Rust engine and `sarv-agent` CLI have no GUI dependencies in their default build. Existing PDF rendering is optional: `cargo test -p sarv-quote --features pdf-export`. Linux Tauri still uses GTK3/WebKitGTK for its native webview; this is a platform dependency, not a second application. Windows uses WebView2 and macOS uses WKWebView.

Use `npm run test:core` for Rust and [desktop QA instructions](../desktop/README.md) for the nine browser workflows using the real isolated Rust adapter. See [agent access](tauri/AGENT-ACCESS.md) and [UI verification](tauri/NATIVE-UI-REDESIGN.md).

## Shadcn preset, React performance and first run

The user-selected `b6TqdhPcaf` preset is applied through the official CLI with component reinstall. Configuration is Radix Luma, teal, IBM Plex Sans/Space Grotesk, Phosphor, inverted menu color and subtle menu accent. The old primitive styling is removed; official Sidebar/Tabs/Switch/Checkbox/Field/Command/Alert/Empty/Avatar/Separator compositions now join the reinstalled Button/Input/NativeSelect/Dialog/Table/Badge/Skeleton/Textarea. Radix DirectionProvider and bundled Vazirmatn keep Persian first-class. Existing financial/layout adapters remain app-specific.

Feature screens/editors and command search are lazy-loaded with navigation preloading. Stable context/callbacks reduce unrelated page rendering. Grid search defers cached-label filtering, and international amount/date formatters are reused. The main entry chunk is 494.87 kB / 155.04 kB gzip versus the previous 527.52 kB / 156.64 kB gzip; this is not a startup latency measurement.

The first-run welcome and company/fiscal/opening steps are localized for English and Persian. CSS handles brief page/dialog/setup entry and control feedback; reduced-motion disables animation. Financial data remains stationary. Existing financial commands, preview/post safeguards and schema are unchanged. Dependencies are the official preset's requirements; the older Lucide/Inter and direct Radix dependencies were removed.

## Known inherited issues

The three legacy-compat tests that failed in upstream commit `caefee7` now pass: optional `Customer.is_customer`, `Settings.app_mode`/`fiscal_year_start_month`/`text_direction` serialize conditionally again, so schema-4 states round-trip byte-exactly.

Completed since the previous handoff: typed payments (customer receipts and supplier payments with exact open-item allocation, surplus advances, typed reversal, control reconciliation), a banking statement-matching reconciliation dialog, and quotations issued from the sales editor (stable numbering, printable A4 PDF, exact conversion to orders). A custom chart editor, attachments, team permissions and platform release qualification remain unfinished.
