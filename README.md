# Sarv / سرو

Local-first Persian accounting for individuals and businesses. The desktop interface uses Tauri 2, React 19, TypeScript, official shadcn components, Tailwind, and pnpm. The original Rust accounting, quotation, procurement, inventory, sales, and returns engine remains the source of business rules and persisted books.

The primary desktop lives in `desktop/`: a minimal native-style shell, dense financial tables, first-class Persian RTL, independent language/digit/calendar settings, and opt-in local CLI/MCP agents. See [desktop development](desktop/README.md), [agent access](docs/tauri/AGENT-ACCESS.md), and [UI verification and limitations](docs/tauri/NATIVE-UI-REDESIGN.md). This remains an implementation checkpoint, not a production release. There is one React frontend and one Rust accounting backend. See [current handoff](docs/CLOUD-HANDOFF.md) for remaining work.

Shop lock, Excel workbooks, and the till are documented in [progress](docs/PROGRESS.md), the [requirements matrix](docs/REQUIREMENTS-MATRIX.md), [Excel](docs/EXCEL.md), [access](docs/ACCESS.md), [point of sale](docs/POS.md), and [release gates](docs/RELEASE-GATES.md). Local `.xlsx` import and export do not need Microsoft 365. Live workbook sync is not configured.

## Local development

Install Node >=20.19 (or >=22.12), pnpm 12.5.1, Rust, and the Tauri 2 prerequisites for your platform. Linux needs WebKitGTK 4.1 and GTK3 development/runtime packages for Tauri’s native webview. There is no GTK4/libadwaita application. Windows uses WebView2; macOS uses WKWebView. The Rust engine and agent CLI build without any GUI toolkit.

```bash
pnpm --dir desktop install
pnpm dev
```

The primary desktop uses `ir.sarv.accounting/books` in the application data directory. Set `SARV_DATA_DIR` explicitly to open an existing Sarv profile. It uses the original validated SQLite store; keep backups and avoid simultaneous editing across interfaces.

For a browser preview of the same React frontend, run `pnpm preview:dev`. Browser financial workflows require the isolated Rust QA adapter described in [desktop development](desktop/README.md); the product uses native Tauri IPC.

## Build

To install Sarv on Linux as a desktop application:

```bash
pnpm desktop:build --no-bundle
pnpm desktop:install
```

Open **Sarv** from your application launcher. Installation copies the desktop binary, agent and icon into your user application directory. Run `pnpm desktop:install` again after rebuilding to update the installed app. `pnpm start` opens the release binary from this checkout.

Linux startup defaults to WebKit's compatible rendering path to avoid DMA-BUF/GBM allocation and related Wayland protocol failures. Set `WEBKIT_DISABLE_DMABUF_RENDERER=0` explicitly to use the default DMA-BUF renderer instead.

```bash
pnpm lint
pnpm build
pnpm desktop:build --no-bundle
```

Rust checks live in `crates/sarv-quote/tests`. Use `pnpm test:core` for the engine and `pnpm test:desktop` with the isolated QA adapter for the desktop workflows. PDF rendering and its tests use `cargo test -p sarv-quote --features pdf-export`. The desktop enables this feature for printable sales documents; normal engine and agent CLI builds exclude PDF renderer dependencies.

Create additional businesses in **Settings → Add company**. First-run setup collects business contact details, registration and tax IDs, invoice reference prefix, terms and footer; these details are saved separately for each business. The selected calendar applies throughout date entry, reports and sales PDFs. Open a sales record and choose **Save printable PDF** to export an A4 document with embedded Persian fonts, totals and signature lines.

Quotations are issued from the sales editor with **Save as quotation**: the number comes from your invoice reference prefix settings and stays stable across edits, the document prints as an A4 PDF with validity and terms, and converting to an order keeps the exact line prices. Add customers from **Customers → New customer**, and products directly from **Inventory → New product**. Sales and purchase forms also offer manual creation without leaving the editor. A blank product code generates a unique code; a first product can create the Piece unit automatically. Unfinished businesses are guided through explicit opening-balance confirmation before creation resumes.

Customer records include first/last name, organization, phone/mobile, email, address, city, postal code, national/tax IDs, website and notes. Click a customer name to edit their profile, browse linked orders/invoices/returns, and review outstanding amounts, monthly net sales, invoiced products and gross profit through the selected date. Profile details are scoped to the business and included in backups; editing them preserves posted document snapshots.

Products have editable purchase/selling price defaults in rials per base unit, barcode, category, brand, preferred supplier, minimum stock and notes. Click a product in Inventory or a posted document to review stock by warehouse, carried cost, recent received/invoiced prices, monthly sales/cost/gross profit, customers/suppliers and linked documents. Pricing estimates show profit per unit, gross margin and markup; actual gross profit uses posted invoice revenue and frozen stock costs with returns. Changing a price default never revalues inventory. New sales/purchase lines use the saved price when you select a product.

**Banking** shows the existing cash/bank ledger accounts, opening/closing balances and dated movement history. Money in, money out and cash/bank transfers require a balanced posting review. These movements support funding and operating income/expenses; they do not allocate customer or supplier invoice payments or connect to external banks. Use the linked journal to review or reverse a posted banking movement. Each account also offers a statement reconciliation dialog: enter the closing statement balance, see the exact difference, and mark the entries you found on the statement.

**Payments** settles customer invoices and supplier bills with exact integer amounts. Choose cash or bank, allocate across open documents (partial payments accumulate), and any surplus becomes an explicit customer or supplier advance. Every payment posts a balanced journal through the same immutable kernel, keeps control accounts reconciled, supports idempotent retries and a typed mirrored reversal. Payment history links to its journal for review.

**Settings → Keyboard shortcuts** lists all supported shortcuts. Use **Ctrl + Alt** (or **Command + Option** on macOS) with **N** for a new sales invoice, **O** for an order, **C** for a customer, **P** for a product, **V** for a vendor, **B** for a purchase order, **W** for a warehouse, **U** for a unit, **J** for a journal, **T** for a stock transfer, and **S** for Settings. Creation shortcuts use physical keys in either keyboard layout and pause in text fields and dialogs. **Ctrl/Command + K** opens the command menu, **F9** toggles the sidebar, **F1** opens shortcut help, and **Esc** closes a dialog. Invoice creation leads through sale entry and explicit delivery/invoice review; no shortcut posts a financial entry automatically.

## One ledger

Canonical money is exact integer Rials; Toman display divides by ten without losing fractional Tomans. React receives money as strings. The existing Rust core validates balanced double-entry journals, fiscal-period locks, immutable postings, duplicate request keys, stock availability, weighted-average costs, and original-cost returns. Financial mutations use `Store::change` with validation and SQLite generation checks. Corrections use reversal journals or the typed goods-return workflows.

The Tauri interface invokes `sarv-quote::desktop::dispatch` and `AgentService` over the original store. The former GTK app, duplicate web/Tauri shells, experimental second ledger, and their build/launch scripts have been removed. Existing books, accounting migrations, financial tests, CSV functions and optional PDF rendering remain in the Rust backend.

The original store persists a validated versioned snapshot in SQLite, with a 64 MiB backup limit. This release has not established suitability for large or concurrent enterprise books. A shared SQLite file is not the team architecture.

## Confirmed roadmap

1. Restore the original workflows in a Persian RTL/Jalali shadcn desktop, with individual/service/trading onboarding and a resumable setup checklist.
2. Add the remaining workflow depth, imports, exports, performance validation and production hardening on the same ledger.
3. Add an optional team server, authorization and enterprise workflows behind the existing transport boundary.
4. Add multi-currency accounting with explicit exchange-rate postings. Rial/Toman comes first.

Later phases add screens and services on top of the same ledger; they must preserve earlier books and history. Payroll, live bank feeds and payment processing remain outside the current scope.

## Licence

The core (the `sarv-quote` engine, the desktop app and the agent binary) is licensed under the
**GNU Affero General Public License v3.0 only** (`AGPL-3.0-only`); see [LICENSE](LICENSE).
Any future SDK or plugin API will be published separately under Apache-2.0. The bundled Vazirmatn font
keeps its own SIL OFL 1.1 licence. Earlier snapshots of this repository were published under MIT.
