# Sarv Tauri desktop interface

Tauri 2 + React 19 + TypeScript + Vite 7 + Tailwind 4, official shadcn Radix Luma components, Phosphor and TanStack Table 8. Financial services come from the existing `sarv-quote` Rust crate. This is the only frontend; `crates/sarv-quote` is the headless Rust accounting backend. No schema migration or frontend accounting engine was added.

From this directory, with Node >=20.19 (or >=22.12), Rust and the platform's Tauri prerequisites installed:

```sh
pnpm install
pnpm exec tauri dev
pnpm exec tauri build --no-bundle
```

Persian is supported throughout the UI with RTL layout and keyboard navigation. In Settings, interface language, display digits, and Gregorian/Jalali display are independent. Date-entry controls preserve Gregorian ISO values and LTR reading order. A fresh installation follows the system language; the user's saved language takes precedence. The official components use preset `b6TqdhPcaf` (Radix Luma, teal, IBM Plex Sans/Space Grotesk and Phosphor), applied with `pnpm dlx shadcn@latest apply --preset b6TqdhPcaf --yes`. Vazirmatn remains bundled for Persian. Buttons, inputs, native selects, tables, badges, dialogs, sidebar, tabs, switches, checkboxes, command search, fields, alerts, avatars, separators, skeletons and tooltips use the registry components. Application adapters preserve localized labels, financial review scrolling and editor autofocus without restyling the primitives.

First run presents a localized welcome and three-step company/fiscal-period/opening setup. Going back preserves entered values; opening balances still require explicit review and confirmation. Short CSS animations cover page entry, dialogs, setup stages and control feedback, with all motion disabled for reduced-motion preferences. Financial rows and amounts stay stationary.

Feature screens, editors and command search load on demand, with navigation preloading. Table search defers filtering while typing; search labels and international formatters are cached. The main production JavaScript chunk is 494.87 kB (155.04 kB gzip), down from 527.52 kB (156.64 kB gzip). This includes the complete official preset sidebar and overlays and measures the entry chunk, not total downloaded assets or runtime latency. The requested preset supplies `tw-animate-css`; no separate animation framework was added.

Native dev/build commands also compile `sarv-agent`. Enable local CLI/MCP access in **Settings → Agents**, choose read-only/full access, and copy the connection. Access starts off and can be revoked in Settings. See [agent commands and access rules](../docs/tauri/AGENT-ACCESS.md).

Normal Rust engine and CLI builds have no GUI dependencies. Existing PDF renderers are opt-in through the `pdf-export` Cargo feature.

For Linux, Tauri needs the distribution's GTK3, WebKitGTK 4.1, libsoup 3 and their development packages. Install their runtime subprocess helpers too. A rootless extraction can compile but does not necessarily run WebKit, which may use compiled-in system paths. This cloud executor built and ran the native webview with a temporary read-only helper/library mapping; see the delivery report and `scripts/run-tauri-cloud.sh`. System files and WebKit sandbox settings are unchanged. macOS and Windows packaging/signing have not been validated.

Default Tauri storage is the application data directory under `ir.sarv.accounting/books`. Set `SARV_DATA_DIR` to an existing Sarv profile to open the same SQLite books; financial writes use the existing generation guards. Keep backups and avoid simultaneous editing across interfaces. Stored-data compatibility is retained; the old GTK application has been removed.

```sh
pnpm check
pnpm lint
pnpm build
```

A normal browser preview does not contain a mock financial backend. It displays a desktop-required error. For **isolated browser QA only**, use the opt-in Rust example from the repository root:

```sh
SARV_QA_DATA_DIR=/absolute/temporary/profile \
SARV_QA_TOKEN=a-test-token-with-at-least-24-characters \
SARV_QA_SEED=1 cargo run -p sarv-quote --example desktop_qa
```

Seed only an empty test profile; it imports the repository's clearly synthetic opening fixture. In another shell, start Vite from this directory with the matching QA token:

```sh
VITE_QA_URL=http://127.0.0.1:1421 \
VITE_QA_TOKEN=a-test-token-with-at-least-24-characters pnpm dev
SARV_QA_TOKEN=a-test-token-with-at-least-24-characters pnpm test
```

The example binds only to loopback, restricts origins and requires an explicit token. `SARV_QA_PORT` optionally selects another loopback port for isolated tests; it defaults to 1421. Compile the example before running Playwright: `cargo build -p sarv-quote --example desktop_qa`. The two first-run tests launch their own empty profiles on port 1422 and cover English/Persian, desktop/mobile, back navigation, reduced motion, explicit opening confirmation and restart persistence. It is not bundled in the product. Production code has no HTTP adapter. The browser test creates a separate synthetic company and checks purchasing, invoice posting, invalid journals, reporting, CSV export, preferences, keyboard commands, dark/RTL and four window sizes. Browser CSV download is separate from the verified native save dialog. QA file watching is disabled to keep posting reviews stable; restart Vite and the Rust adapter after source changes before testing.

Supported: core company/chart/opening, operational contacts/resources, durable goods sales drafts/orders, paired delivery/invoice, purchasing/receiving/billing, stock inspection, manual journals/reversals, period lock/reopen, P&L/trial/control reports and CSV. Partial receipt/bill/delivery selection, original-cost customer and supplier returns, and warehouse transfers are also connected. Customer receipts and supplier payments allocate exact amounts to open invoices and bills, with surplus advances, typed reversal, and full control reconciliation. Banking offers a statement-matching reconciliation dialog per cash/bank account. Quotations are issued from the sales editor with stable series numbering, printable A4 PDFs, and exact conversion into sales orders. Missing: service invoices, attachments, approvals/projects, and translation of arbitrary imported/backend messages. A shop password and roles exist inside one local profile. That is not multi-shop isolation, not a tafsili chart, and not the 81-box or 156-box checklist. This is not a production release.
