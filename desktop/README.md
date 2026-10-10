# Sarv Tauri desktop interface

Tauri 2 + React 19 + TypeScript + Vite 7 + Tailwind 4, Radix/shadcn-style shared components, Lucide and TanStack Table 8. Financial services come from the existing `sarv-quote` Rust crate. The preserved GTK app remains available in `crates/sarv-quote-gtk`. No schema migration or frontend accounting engine was added.

From this directory, with Node >=20.19 (or >=22.12), Rust and the platform's Tauri prerequisites installed:

```sh
npm ci
npm run tauri dev
npm run tauri build -- --no-bundle
```

Persian is supported throughout the UI with RTL layout and keyboard navigation. In Settings, interface language, display digits, and Gregorian/Jalali display are independent. Date-entry controls preserve Gregorian ISO values. A fresh installation follows the system language; the user's saved language takes precedence. The existing shadcn composition and official source primitives share the same minimal theme; no extra component libraries are required.

Native dev/build commands also compile `sarv-agent`. Enable local CLI/MCP access in **Settings → Agents**, choose read-only/full access, and copy the connection. Access starts off and can be revoked in Settings. See [agent commands and access rules](../docs/tauri/AGENT-ACCESS.md).

For Linux, Tauri needs the distribution's GTK3, WebKitGTK 4.1, libsoup 3 and their development packages. Install their runtime subprocess helpers too. A rootless extraction can compile but does not necessarily run WebKit, which may use compiled-in system paths. This cloud executor built and ran the native webview with a temporary read-only helper/library mapping; see the delivery report and `scripts/run-tauri-cloud.sh`. System files and WebKit sandbox settings are unchanged. macOS and Windows packaging/signing have not been validated.

Default Tauri storage is the application data directory under `ir.sarv.accounting/books`. Set `SARV_DATA_DIR` to an existing Sarv profile to open the same SQLite books; financial writes use the existing generation guards. Keep backups and avoid simultaneous editing across interfaces. The original GTK launch and its default profile remain unchanged.

```sh
npm run check
npm run lint
npm run build
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
VITE_QA_TOKEN=a-test-token-with-at-least-24-characters npm run dev
SARV_QA_TOKEN=a-test-token-with-at-least-24-characters npm test
```

The example binds only to loopback, restricts origins and requires an explicit token. It is not bundled in the product. Production code has no HTTP adapter. The browser test creates a separate synthetic company and checks purchasing, invoice posting, invalid journals, reporting, CSV export, preferences, keyboard commands, dark/RTL and four window sizes. Browser CSV download is separate from the verified native save dialog. QA file watching is disabled to keep posting reviews stable; restart Vite and the Rust adapter after source changes before testing.

Supported: core company/chart/opening, operational contacts/resources, durable goods sales drafts/orders, paired delivery/invoice, purchasing/receiving/billing, stock inspection, manual journals/reversals, period lock/reopen, P&L/trial/control reports and CSV. Partial receipt/bill/delivery selection, original-cost customer and supplier returns, and warehouse transfers are also connected. Original quotation/PDF workflows remain in GTK. Missing: service invoices, payments/refunds, bank statements/reconciliation, attachments, approvals/projects, authentication/roles and translation of arbitrary imported/backend messages. This is a verified implementation checkpoint, not a production release.
