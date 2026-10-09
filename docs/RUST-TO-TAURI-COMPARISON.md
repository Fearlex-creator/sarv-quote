# Original Rust application versus the current Tauri application

Comparison made on 2026-10-10 from repository history, current source, and stored native screenshots. This is a feature and architecture comparison, not a fresh acceptance test of the old application.

The current Tauri application is a separate six-screen money-management application. It does not preserve the existing Sarv application's business workflows or read its accounting database. It should not be described as a completed migration or an equivalent replacement.

## Which Rust version?

- First committed version: `c21c4ed`, “Publish private Sarv Quote v1 snapshot with tested scale and recovery fixes.” It already had product/price-list management, customers, quotation preparation, approved quotation history, PDF/CSV output, and backup/restore. It explicitly did not post invoices or inventory movements.
- Latest committed Rust baseline: `7742737`. Subsequent development added accounting, guided detailed openings, purchases, inventory, goods sales, and original-cost returns. This is the baseline to preserve when changing the frontend, rather than only the very first quotation release.

## Feature comparison

| Workflow | First Rust snapshot | Latest committed Rust baseline | Current Tauri UI |
| --- | --- | --- | --- |
| Products and approved supplier price lists | Product codes, names, units, exact prices, aliases, source dates, validated imports | Also catalog composition and expanded management | Absent |
| Quotations | Item selection, quantities, source prices, approval, revisions, archive | Expanded customer details, keyboard editing, native document preview | Absent |
| Goods purchases | Absent | Purchase orders, partial receipts, matched supplier bills | Absent; money expenses do not receive goods |
| Inventory | Absent as an operational stock ledger | Warehouses, quantities, carried values, unbilled receipts, transfers, reconciliation | Absent; low-stock dashboard placeholder only |
| Goods sales | Absent; quotations are offers | Sales drafts/orders, quotation conversion, linked deliveries/invoices, receivables and cost of goods sold | Absent; money income does not deliver goods |
| Returns | Absent | Original-cost sales returns and linked purchase returns/supplier credits | Absent |
| Accounting | Exact quotation calculations; no posted books | Companies, fiscal periods, chart, detailed openings, journal, reversals, period locks and trial balance | New wallet/category journal, corrections and dashboard; no migration of those books |
| Contacts | Saved customers | Customers plus linked customer/supplier workflow balances | Customer/supplier records with balances in the new ledger only |
| Documents and recovery | Quotation PDF/CSV, validated backup/restore | Additional linked business-document exports and recovery validation | Ledger/contact PDF/CSV, normalized CSV import and JSON export; no equivalent full restore workflow |
| UI organization | Persian native quotation/product workspace | Persian native navigation grouped around sales, purchases, inventory, accounting, quotations and products | Dashboard, transactions, Quick Add, wallets, contacts and settings |

The latest Rust baseline also has documented unfinished work, including treasury/payment allocation, cheques and broader release/performance acceptance. Preserving it does not mean calling those unfinished features complete.

## The architecture gap

The preserved native frontend is in `crates/sarv-quote-gtk`; its existing domain logic is in `crates/sarv-quote`. The native sidebar actually exposes sales, purchases, inventory, accounting, quotations, customers, products, backup and settings. Its accounting/procurement/sales modules remain available.

The current Tauri entry point instead uses `crates/free-accounting-core` and opens a separate `accounting.sqlite3`. It does not read the original `sarv.sqlite3` state, interpret its schema-7 document relationships, or migrate those records. Reusing Rust as the implementation language does not connect these two sets of books.

All 69 tracked files in the old root `src/` and `tests/` have counterparts in the reorganized crates: 61 match `HEAD` byte for byte and 8 differ. That confirms the original source is still available; it does not prove every original runtime behavior is unchanged. No existing user database was migrated as part of the new six-screen implementation.

## Why this happened

The supplied build prompt explicitly requested only six Phase 1 screens and deferred invoices, bills and inventory. Applying that scope as a new application excluded features already implemented in Sarv. This conflict should have been identified before presenting the new shell as the next version of the existing product.

## Recommended correction

Use the latest Rust baseline as the product and data contract. If Tauri/React remains the desired frontend, adapt its coarse commands to the existing domain operations and restore the existing business navigation and workflows. Product/quotation, purchases/inventory, and sales/returns parity should be verified against the old documents, stock quantities/values, journal balances and restart behavior before switching the default application. Changes to storage need an explicit validated migration that preserves document IDs, posted history, cost provenance and recovery.

Keep the new money-management implementation isolated until that decision; do not combine or relabel its entries as migrated Sarv history. A cosmetic Inventory menu or products placeholder would not resolve the missing workflows.

To run the preserved Rust frontend against the original project-local data location:

```sh
SARV_DATA_DIR="$PWD/data" pnpm gtk
```

The explicit data directory matters because moving the GTK crate changes the manifest-relative default location. This command requires the native GTK/libadwaita prerequisites. It was inspected, not launched during this comparison.

## Source references

- Earliest contract: `git show c21c4ed:README.md`.
- Latest native contract: `git show 7742737:README.md` and [existing roadmap](../requirements/roadmap.md).
- [Existing native navigation](../crates/sarv-quote-gtk/src/ui.rs), [accounting](../crates/sarv-quote/src/accounting.rs), [purchases/inventory](../crates/sarv-quote/src/procurement.rs), [sales](../crates/sarv-quote/src/sales.rs).
- [Current navigation](../web/src/accounting/AppShell.tsx) and [Tauri database entry point](../src-tauri/src/lib.rs).
- [Earlier product UI capture](assets/delivery-complete-native-products.png) and [native inventory/navigation capture](assets/ui-copy/native/sidebar/copy-inventory-light.png).
