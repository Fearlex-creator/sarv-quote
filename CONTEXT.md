# Sarv Domain Language

Sarv is a local-first Persian accounting and quotation system. The canonical ledger and quotation rules live in `crates/sarv-quote`.

## Accounting & operations

**Books**: A SQLite-backed profile directory (`SARV_DATA_DIR`) holding the company's validated state snapshot, audit history, and rolling recovery backups.
_Avoid_: Database cluster, shared file

**Company**: A business entity inside the books with its own chart of accounts, policy snapshot, fiscal years, parties, items, and documents.

**Fiscal period**: A bounded date range (fiscal year) with an optional lock state (`locked`) that prevents ordinary postings into closed periods.

**Journal**: An immutable, balanced double-entry ledger entry (sum of debits equals sum of credits in integer rials) linked to an audited event.
_Avoid_: Mutable transaction

**Posting**: Committing a validated business event or manual journal to the ledger and audit log. Drafts and quotations never post.

**Reversal**: A new compensating journal or payment reversal that offsets a prior posting without editing or deleting posted history.
_Avoid_: Delete, edit posted entry

**Draft**: An editable, unposted sales or quotation proposal. Raw drafts may hold incomplete text until review/posting and never affect the ledger or stock.

**Quotation**: An approved, immutable price offer with frozen product and price snapshots, numbered from the company's invoice reference prefix.
_Avoid_: Invoice, ledger entry

**Sales order / Invoice**: A confirmed goods sale (`Sales order`) fulfilled by a paired delivery and `Invoice` that posts revenue, receivables, and moving-average cost of goods sold.

**Purchase order / Receipt / Bill**: A supplier order (`Purchase order`, no ledger/stock effect), a goods `Receipt` (debits Inventory, credits GRNI), and a matched supplier `Bill` (debits GRNI, credits Accounts Payable).

**Open item**: An unpaid or partially paid invoice or supplier bill with a remaining balance eligible for payment allocation.

**Allocation**: Applying a payment amount across one or more open items, updating their remaining balances.

**Advance**: Surplus payment amount not allocated to an open item, tracked as a customer or supplier advance balance.

**Cheque**: A received or issued negotiable instrument tracked in an append-only register (`cheques.rs`) with states `InHand`, `InCollection`, `Cleared`, `Bounced`, `Spent`, `Returned`, `Issued`, and `Voided`.

**Generation**: The store's optimistic-concurrency counter (`expected_generation`) checked on every `Store::change` commit to prevent stale overwrites.

**Request key**: An idempotency key (`request_key`) passed on mutations so retries resolve to the original committed result without duplicate posting.

**Rial / Toman**: Canonical money is exact integer rials (`1 toman = 10 rials`). Toman display divides by ten without losing fractional tomans.

## Quotation preparation

**Catalog**: A named, approved snapshot of supplied product prices, units, aliases and source timestamps.
_Avoid_: Inventory, live price feed

**Approved alias**: A supplier-authorized alternative item identifier included in an approved catalog. An alias may refer to several products and then requires human review.
_Avoid_: Suggested substitute

**Customer order list**: Requested item identifiers and quantities supplied by a customer, awaiting explicit resolution where needed.
_Avoid_: Posted sale

**Quote revision**: An immutable, approved offer containing exact product and price snapshots. Revisions in one series preserve prior offers.
_Avoid_: Invoice, ledger entry
