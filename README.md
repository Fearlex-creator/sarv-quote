# Sarv / سرو

Local-first Persian accounting for individuals and businesses. The books and the ledger rules live in `crates/sarv-quote`. The desktop is `crates/sarv-desktop`, a Rune and rune-kit application that calls the engine in-process. There is no React, Tauri, or GTK application.

## Desktop

```bash
cargo run -p sarv-desktop
cargo test -p sarv-desktop --bin sarv-desktop
SARV_DEMO=1 cargo run -p sarv-desktop -- --scene overview
```

`SARV_DATA_DIR` chooses the books directory. Without it, the app uses `ir.sarv.accounting/books` under the XDG data directory. `SARV_DEMO=1` opens a throwaway fixture. Snapshot scenes (`--scene`, `--snap`) need a GPU: `cargo test -p sarv-desktop --test png_scenes`.

A new profile opens on a welcome: language and theme, then the company and fiscal year, then opening cash and bank. The sidebar appears after the books are open.

The UI must not invent ledger rules. It sends the engine's existing actions (`draft_post`, `fulfill_post`, `payment_post`, `banking_post`, and the rest) and shows the engine's error when a post is refused. Payroll, live bank feeds, payment processors, and a team server are out of scope.

See [AGENTS.md](AGENTS.md). Screen notes are in [the desktop guide](docs/FRONTEND-UI-GUIDE.md).

## Install

```bash
cargo build --release -p sarv-desktop -p sarv-quote --bin sarv-agent
bash scripts/install-desktop.sh
```

Open **Sarv** from your application launcher. The script copies the desktop binary, the agent, and the icon into your user application directory. Run it again after rebuilding to update the installed app.

```bash
cargo test -p sarv-quote
cargo test -p sarv-desktop --bin sarv-desktop
cargo test -p sarv-quote --features pdf-export
```

The desktop enables PDF export for printable sales documents. Normal engine and agent CLI builds exclude the PDF renderer. Linux PDF rendering needs cairo and pango.

Create additional businesses in **Settings → Add company**. First-run setup collects business contact details, registration and tax IDs, invoice reference prefix, terms and footer; these details are saved separately for each business. The selected calendar applies throughout date entry, reports and sales PDFs. Open a sales record and choose **Save printable PDF** to export an A4 document with embedded Persian fonts, totals and signature lines.

Quotations are issued from the sales editor with **Save as quotation**: the number comes from your invoice reference prefix settings and stays stable across edits, the document prints as an A4 PDF with validity and terms, and converting to an order keeps the exact line prices. Add customers from **Customers → New customer**, and products directly from **Inventory → New product**. Sales and purchase forms also offer manual creation without leaving the editor. A blank product code generates a unique code; a first product can create the Piece unit automatically. Unfinished businesses are guided through explicit opening-balance confirmation before creation resumes.

Customer records include first/last name, organization, phone/mobile, email, address, city, postal code, national/tax IDs, website and notes. Click a customer name to edit their profile, browse linked orders/invoices/returns, and review outstanding amounts, monthly net sales, invoiced products and gross profit through the selected date. Profile details are scoped to the business and included in backups; editing them preserves posted document snapshots.

Products have editable purchase/selling price defaults in rials per base unit, barcode, category, brand, preferred supplier, minimum stock and notes. Click a product in Inventory or a posted document to review stock by warehouse, carried cost, recent received/invoiced prices, monthly sales/cost/gross profit, customers/suppliers and linked documents. Pricing estimates show profit per unit, gross margin and markup; actual gross profit uses posted invoice revenue and frozen stock costs with returns. Changing a price default never revalues inventory. New sales/purchase lines use the saved price when you select a product.

**Banking** shows the existing cash/bank ledger accounts, opening/closing balances and dated movement history. Money in, money out and cash/bank transfers require a balanced posting review. These movements support funding and operating income/expenses; they do not allocate customer or supplier invoice payments or connect to external banks. Use the linked journal to review or reverse a posted banking movement. Each account also offers a statement reconciliation dialog: enter the closing statement balance, see the exact difference, and mark the entries you found on the statement.

**Payments** settles customer invoices and supplier bills with exact integer amounts. Choose cash or bank, allocate across open documents (partial payments accumulate), and any surplus becomes an explicit customer or supplier advance. Every payment posts a balanced journal through the same immutable kernel, keeps control accounts reconciled, supports idempotent retries and a typed mirrored reversal. Payment history links to its journal for review.

**Settings → Keyboard shortcuts** lists all supported shortcuts. Use **Ctrl + Alt** (or **Command + Option** on macOS) with **N** for a new sales invoice, **O** for an order, **C** for a customer, **P** for a product, **V** for a vendor, **B** for a purchase order, **W** for a warehouse, **U** for a unit, **J** for a journal, **T** for a stock transfer, and **S** for Settings. Creation shortcuts use physical keys in either keyboard layout and pause in text fields and dialogs. **Ctrl/Command + K** opens the command menu, **F9** toggles the sidebar, **F1** opens shortcut help, and **Esc** closes a dialog. Invoice creation leads through sale entry and explicit delivery/invoice review; no shortcut posts a financial entry automatically.

## One ledger

Canonical money is exact integer Rials; Toman display divides by ten without losing fractional Tomans. The desktop receives money as strings. The Rust core validates balanced double-entry journals, fiscal-period locks, immutable postings, duplicate request keys, stock availability, weighted-average costs, and original-cost returns. Financial mutations use `Store::change` with validation and SQLite generation checks. Corrections use reversal journals or the typed goods-return workflows.

The desktop calls `sarv-quote::desktop::dispatch` in-process. The former GTK app, the React and Tauri shells, and their launch scripts have been removed. Existing books, accounting migrations, financial tests, CSV functions and optional PDF rendering remain in the Rust backend.

The original store persists a validated versioned snapshot in SQLite, with a 64 MiB backup limit. This release has not established suitability for large or concurrent enterprise books. A shared SQLite file is not the team architecture.

## Confirmed roadmap

1. The Persian RTL/Jalali desktop is the Rune app, with a first-run welcome, company, fiscal year, and opening balances.
2. Add the remaining workflow depth, imports, exports, performance validation and production hardening on the same ledger.
3. Add an optional team server, authorization and enterprise workflows behind the existing transport boundary.
4. Add multi-currency accounting with explicit exchange-rate postings. Rial/Toman comes first.

Later phases add screens and services on top of the same ledger; they must preserve earlier books and history. Payroll, live bank feeds and payment processing remain outside the current scope.

## Licence

The core (the `sarv-quote` engine, the desktop app and the agent binary) is licensed under the
**GNU Affero General Public License v3.0 only** (`AGPL-3.0-only`); see [LICENSE](LICENSE).
Any future SDK or plugin API will be published separately under Apache-2.0. The bundled Vazirmatn font
keeps its own SIL OFL 1.1 licence. Earlier snapshots of this repository were published under MIT.
