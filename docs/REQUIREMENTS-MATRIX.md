# Requirements matrix

Status is what this branch can show with a test or a named blocker. A screen that only looks finished is not marked implemented.

| Need | Status | Acceptance that was actually checked | Where |
| --- | --- | --- | --- |
| Keep Sarv's ledger, SQLite book, and schema 7 | Implemented | Full `cargo test -p sarv-quote` including migration tests. Empty `retail` is omitted from JSON. | `domain.rs`, `store.rs` |
| Shop password, roles, no default credential | Implemented | `shop_lock_hides_secrets_and_cashier_cannot_post_a_journal` checks the database bytes and a cashier journal denial. Recovery replaces the password. The code is not written into logs by the test. | `access.rs`, Settings → Shop lock |
| Session for every desktop command once the lock exists | Implemented | `desktop::dispatch` and `AgentService::ui_dispatch` call `access::enforce`. PDF/CSV save in the Tauri shell checks the same lock. | `desktop_bridge.rs`, `agent.rs`, `src-tauri/src/main.rs` |
| Agent token cannot skip the lock | Blocked as a documented limit | The agent has its own token and still uses that path. Closing it would change the existing agent contract. | `agent.rs` |
| Email password reset | Not implemented | There is no mail transport. Recovery is a local code shown once. | `access.rs` |
| Catalog SKU, barcode, price, cost | Implemented, already in the engine; Excel now loads them | Product round-trip keeps SKU `00123` as text and Persian digits `۲۵۰۰۰۰` as 250000 rials. | `excel.rs`, `product_profile.rs` |
| Quote to invoice to stock to payment | Implemented on the existing kernel | POS checkout calls draft, fulfill, and payment. The browser test sells one unit and the report stays balanced. | `pos.tsx`, `sales.rs` |
| Negative stock policy | Implemented | Default remains reject. The POS screen says so. Concurrent last-unit test leaves quantity 0. | `concurrent_last_unit_sells_once_and_return_restores_stock` |
| Idempotent retry | Implemented | Same `request_key` returns the original invoice or return. A second different sale of the last unit fails. | same test; existing payment tests |
| Partial return | Implemented on the kernel | The new test returns the sold unit and stock returns to 1000 thousandths. Older partial-sale tests remain. | `sales.rs`, `desktop_bridge` tests |
| Till expected vs counted cash | Implemented as a record, not a journal | Close with 150000 expected and counted posts no journal. | `till_close_records_expected_cash_without_a_journal` |
| Live card terminal or bank collection | Not implemented | The screen says the amount is a manual record. No PAN/CVV fields exist. | `pos.tsx` |
| Offline POS queue | Not implemented | Checkout is disabled while `navigator.onLine` is false. No silent sale. | `pos.tsx` |
| Double-entry, trial balance, P&L, balance sheet | Implemented before this slice and still tested | Opening stock: inventory 1301 debit equals GRNI 2201 credit. Trial debit equals credit. | `statements.rs`, Excel opening test |
| COGS | Implemented as weighted average already in the engine | Unchanged. Opening stock uses the typed unit cost. | procurement receive |
| Closed periods | Implemented before this slice | Existing period-lock tests still pass. | `period_lock` |
| Tafsili, statutory e-invoice, payroll | Not implemented | Law data stays unreviewed and must not feed a filing. | `docs/SPEC-COVERAGE.md` |
| Real `.xlsx` import and export | Implemented | Calamine reads values. rust_xlsxwriter writes the export. Caps: 1 MB compressed, 8 MB uncompressed, 64 entries, 5000 rows. | `excel.rs` |
| CSV as the only import | Not a substitute | CSV opening import remains for the older path. Excel is a separate command. | `opening_csv` |
| Formula and macro execution | Refused | `=1+1` and `xl/vbaProject.bin` fail the tests. | `excel_rejects_formulas_macros_numeric_skus_and_oversized_files` |
| Microsoft 365 live sync | Blocked | Status is `not_configured`. `excel_graph_sync` returns an error. No OAuth client is included. | `excel.rs` |
| Multi-tenant isolation | Not claimed | One local profile. Roles apply inside that profile. | `access.rs` |
| Chrome desktop and mobile RTL | Implemented for the new flow | `retail-excel-pos.spec.ts` on the system Chrome. | `desktop/tests` |
| Firefox and WebKit | Blocked in this environment | Those browsers were not installed. Not marked passed. | — |
| Windows release | Not run | `.github/workflows/release-windows.yml` publishes on `v*` or a manual dispatch. Neither was used. | `docs/RELEASE-GATES.md` |
