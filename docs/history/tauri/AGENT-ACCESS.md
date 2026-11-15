# Local agent access

Open **Settings → Agents**, choose **Read only** or **Full access**, and turn **Agent access** on. **Copy CLI** copies the local command; **Copy MCP** copies a standard `mcpServers.sarv` configuration. **Revoke** turns access off. A new profile starts off. The permission preference persists; the desktop app must be running. This integration needs no model credentials and makes no outbound model calls.

Native dev/build commands compile `sarv-agent` too. For a standalone distribution, put `sarv-agent` (`sarv-agent.exe` on Windows) next to the desktop executable or set `SARV_AGENT_BIN` to its absolute path. Source builds find the root `target/debug` or `target/release` binary. Packaging/signing remain separate release work.

```sh
cargo build -p sarv-quote --release --bin sarv-agent
sarv-agent --connection /absolute/profile/.agent-connection.json status
sarv-agent --connection /absolute/profile/.agent-connection.json tools
sarv-agent --connection /absolute/profile/.agent-connection.json schema journal_post
sarv-agent --connection /absolute/profile/.agent-connection.json call workspace --json '{}'
sarv-agent --connection /absolute/profile/.agent-connection.json call journal_preview --stdin < journal.json
```

Append the desired operation to the CLI command copied from Settings. Alternatively set `SARV_AGENT_CONNECTION`. `call` accepts a JSON object using `--json` or `--stdin`. Successful replies are JSON values; errors contain `{ "error": { "code", "message" } }` and exit nonzero. Exports return filename/CSV content for agents to save using their own filesystem access.

## Discover and use the books

First call `workspace` without a company ID. Select a discovered company and call it again with `company_id` to obtain periods, accounts, setup/resource IDs, preferences, drafts and source records. Source/ledger lists page at 100 records using a string `offset`; counts describe complete source lists. Reports include trial balance, P&L, position, party balances, stock and control reconciliation.

Amounts, quantities, revisions, offsets and generations are **strings**. Money is integer rials unless a raw sales draft explicitly chooses `Toman`. Input quantities are decimal base units with up to three fractional digits. Output quantities ending in `_thousandths` are integer strings in thousandths. Dates are ISO `YYYY-MM-DD`. Discover company/period/account/resource IDs rather than guessing them.

For a financial operation, create a unique `source_id`/`request_key`, call its `_preview`, inspect totals/costs/journal lines, and copy `_generation` into `expected_generation` on the corresponding `_post`. The payload and key must otherwise stay identical. Opening CSV, receipts, bills, delivery/invoices, returns, transfers and manual journals require this generation. Reversal uses `journal_reverse_preview`, then `journal_reverse` with the same generation. Changed books require a fresh preview. Retain the request key only for an identical retry.

For partial processing, call `operation_options` with the source `record_id` and operation. Send selected `{ "id": "eligible-line-id", "quantity": "1" }` rows as `lines`; `"0"` leaves a line untouched. Returns use `record_id`, receipts use `order_id`, bills use `receipt_id`, and delivery/invoice uses `order_id`. Bills and delivery require `due_date`. The Rust kernel validates company membership, remaining quantities, original allocations and stock availability.

For sales, create a raw draft with its discovered company/period/setup/customer IDs, dates, reference, `date_display` (`Iso`/`Jalali`), `display_unit` (`Rial`/`Toman`), discounts, supplied total, reason and lines. Use `draft_preview` to calculate `supplied_total`, then `draft_save`, then `draft_post` with the returned revision. Delivery/invoicing is a separate financial operation. `initialize_zero` requires explicit `confirmed: true` for a fresh company with no opening balances or stock; existing businesses use opening CSV. `settings` takes the complete current `workspace.settings` object and cannot alter agent permissions.

## MCP tools

`sarv-agent --connection PATH mcp` is a newline-delimited JSON-RPC MCP stdio server supporting initialization, initialized notifications, ping, tool discovery and calls. It negotiates supported protocol `2024-11-05`. MCP clients use the copied configuration; CLI-capable agents use the same operations directly. Stdout contains protocol messages only.

There are **36 tools**, named `sarv_<action>`, with schemas and read/write annotations:

| Workflow | Actions |
|---|---|
| Discovery and records | `workspace`, `detail`, `ledger`, `operation_options` |
| Reports and exports | `report`, `report_csv`, `profit_csv` |
| Company and opening | `setup_company`, `initialize_zero`, `opening_csv_preview`, `opening_csv_post` |
| Contacts/resources | `resource` with customer/vendor/unit/warehouse/item kind |
| Sales drafts/orders | `draft_preview`, `draft_save`, `draft_delete`, `draft_post` |
| Purchasing | `purchase_preview`, `purchase_post` |
| Receiving and bills | `receive_preview`, `receive_post`, `bill_preview`, `bill_post` |
| Delivery/invoice | `fulfill_preview`, `fulfill_post` |
| Original-cost returns | `sale_return_preview`, `sale_return_post`, `purchase_return_preview`, `purchase_return_post` |
| Warehouse transfer | `transfer_preview`, `transfer_post` |
| Journals/reversal | `journal_preview`, `journal_post`, `journal_reverse_preview`, `journal_reverse` |
| Period/preferences | `period_lock`, `settings` |

These cover implemented Tauri workflows. Original GTK quotation/PDF tools and missing accounting services are not exposed as fake agent tools. Agent grant controls are available only in Settings.

## Access boundary and verification

The service binds an ephemeral **127.0.0.1** TCP port, authenticating every request with a random private token. The connection file is created with mode `0600` on Unix. Copied CLI/MCP commands contain its path, never its token. Keep the file private to the authorized OS account. Profile access by the OS account remains the trust boundary; this is not multi-user accounting authorization or provider-specific identity management.

The gateway fixes the profile path and action allowlist and uses the same Rust Store/kernel as the UI. Requests cannot choose an arbitrary database/output path or shell command. Message sizes, active clients and timeouts are bounded. Read-only rejects persistent operations before dispatch. Revocation and accepted calls are serialized, so queued calls cannot write after disable returns. Permission changes/restarts issue fresh credentials. Persisted grant epochs invalidate stale instances too.

Exact money, period locks, generation checks, company isolation, idempotency and immutable history remain enforced. Agent writes use audit actor `local-agent`. Accepted writes emit a native refresh event so the interface reloads books/reports; failed calls and reads stay quiet.

Six Rust integration tests exercise the real CLI goods cycle, exact returns/transfers and reconciliation, large money values, preview freshness, locks/retries/reversal, default-off/read-only/forbidden grant controls, secrecy/file mode, revocation/restart/stale instances, forged tokens, remote endpoint rejection, corrupt policy and MCP. The browser agent test exercises actual Settings, keyboard navigation, copied commands, CLI writes, MCP reads and revocation. Native WebKit/X11 verification additionally checks grants, clipboard, posting/reporting, automatic refresh, downgrade and revoke. Evidence: `artifacts/tauri-agents-native-evidence.json`.
