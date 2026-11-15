# Plan 019: Users can reach records older than the latest 100 and open any record to see its lines and journal

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-quote/src/desktop_bridge.rs crates/sarv-desktop/src/data.rs crates/sarv-desktop/src/ui/views`

## Status

- **Priority**: P1
- **Effort**: M–L
- **Risk**: LOW (read-only features)
- **Depends on**: 012 (row models) strongly recommended; 003 (performance)
- **Category**: direction (feature)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Two read-path gaps make the books hard to trust and, past a certain size, partly invisible:

1. **The 100-row window.** The `workspace` action returns only the newest 100 journals, sales, purchases, payments and drafts. The engine accepts an `offset`, but the desktop never sends one. A shop writing 10 invoices a day loses sight of anything older than two weeks. The account ledger dialog is worse: it shows 12 plain-text rows of an at-most-100-row response.
2. **No record detail.** Clicking a row only selects it. The engine has a `detail` action that returns the full journal, sale, purchase or payment by id; nothing calls it. After posting, a user cannot see an invoice's lines or the journal a payment produced, and reports have no drill-down.

## Current state

- Engine `workspace` (`crates/sarv-quote/src/desktop_bridge.rs:1008`, one very long line): lists are built as `….rev().skip(offset).take(100)`; `offset` comes from `optional(&p, "offset", "0").parse::<usize>()?`; the response includes `journal_count`, `sales_count`, `purchase_count`, `payment_count`. One `offset` applies to all lists at once.
- Engine `detail` (`desktop_bridge.rs:1768-1808`): payload `{company_id, id}`; returns the raw `Journal`, sales record, procurement record or payment record JSON, or the error "Record not found in this company".
- Engine `ledger` (`:1809-1821`): payload `{account_id, from, to, offset}`; returns `{"count": n, "rows": [{journal_id, date, reference, description, debit, credit}]}` — 100 rows per call, **no running balance**.
- Desktop: `crates/sarv-desktop/src/data.rs:69-99` `Books::refresh` calls `workspace` with only `company_id`. `Books::list(key)` returns the array. No file in `crates/sarv-desktop/src` sends `offset`.
- Desktop ledger dialog: `crates/sarv-desktop/src/ui/views/accounting.rs:379-407` — `rows.iter().take(12)`, each rendered as `"{date}  {debit}  {credit}"`.
- Row selection handlers only store an id, e.g. `ui/views/sales.rs:422`: `.on_select(|st, id, _| st.sales.selected = id.strip_prefix("sale-").unwrap_or(&id).to_string())`. Check whether the kit `Table` has a separate activate/double-click callback: `grep -n "pub fn on_" ~/.cargo/git/checkouts/rune-*/b556a38/crates/rune-kit/src/table.rs`.
- Display helpers: `st.fmt.money(rials_str)`, `st.fmt.qty(thousandths_str)`, `st.fmt.date(iso)`, `st.books.party_name(id)`. Engine JSON numbers arrive as strings — read with `crate::data::s(&v)`.
- UI helpers (`crates/sarv-desktop/src/ui/glass.rs`): `ui.dialog_described(...)`, `ui.panel_rows(key, rows)`, `ui.section(key, title, body)`, `ui.cell`, `ui.muted`, kit `Table`.
- Record JSON shapes are not documented; discover them by printing: in a scratch test call `dispatch(dir, "detail", json!({"company_id": c, "id": some_id}))` for one record of each kind from the demo fixture and read the keys. Sales records wrap their data in a `command`/`data` enum keyed by variant (see `is_invoice` at `ui/views/sales.rs:801-803`: `record["command"].get("Fulfill")`).

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Engine tests | `cargo test -p sarv-quote` | all pass |
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-quote/src/desktop_bridge.rs` (new read-only `records` action; `ledger` running balance), `crates/sarv-quote/tests/desktop_bridge.rs`, `crates/sarv-desktop/src/data.rs`, new `src/ui/detail.rs`, `src/ui/views/{sales,purchases,payments,accounting,reports,banking,overview}.rs`, `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: changing the `workspace` response (the first page stays as is, other code depends on it); full-text search in the engine; editing posted records (they are immutable by design — corrections are reversals and returns).

## Git workflow

- Branch `advisor/019-history-paging-and-record-detail`; commits `feat(engine): paged records action`, `feat(desktop): load older records`, `feat(desktop): record detail sheet`, `feat(desktop): full ledger view`.

## Steps

### Step 1: Engine — a paged, filterable list per kind

Add a read-only action `"records"`: payload `{company_id, kind: "journals"|"sales"|"purchases"|"payments"|"drafts", offset, limit (≤200, default 100), party_id?, from?, to?, query?}`. Return `{"count": total_after_filter, "rows": [...]}` with rows in the same shape and order (`newest first`) that `workspace` uses for that kind — reuse the same iterator expressions so the desktop's existing accessors keep working. `query` matches the record's reference/source id (case-insensitive substring); `party_id` and date filters apply where the record has those fields.

Add `"records"` to the agent's `READ` list in `crates/sarv-quote/src/agent.rs` only if `"workspace"` is there (keep the two consistent) — otherwise leave the agent alone.

Tests (`tests/desktop_bridge.rs` helpers): post 120 journals in a loop (pattern: any `journal_post` usage in that file), then `records` with `offset 0` → 100 rows and `count ≥ 120`; `offset 100` → the remainder; the first row of page 1 equals `workspace.journals[0]`; date and party filters reduce `count`.

### Step 2: Engine — ledger running balance

In the `ledger` arm add to the response `"opening_balance"` (signed sum of debit−credit for lines of that account dated before `from`, plus those before `offset` inside the range) and to each row `"balance"` (running). Compute with checked `i128`/the engine's `Money` arithmetic. Test: ledger of the cash account in the fixture — last row's balance equals `opening + Σdebit − Σcredit`; page 2's first balance continues from page 1's last.

**Verify**: `cargo test -p sarv-quote` → all pass.

### Step 3: Desktop — "Load older" on every list

Add to `Books`: `pub fn load_more(&mut self, engine: &Engine, key: &str /* "journals" etc. */) -> Result<usize, String>` that calls `records` with `offset = current list length` and appends the rows to `self.workspace[key]`. `refresh()` naturally resets to the first page.

Under each list (Sales invoices, Sales drafts, Purchases, Payments, Accounting journals) show a footer row when `list.len() < count`: muted text `"{shown} of {total}"` (use `st.fmt.number`) and a `Btn::Ghost` "Load older" button. If plan 012 landed, call the view's row rebuild after appending.

Persian: `("Load older", "نمایش قدیمی‌ترها")`, `("{shown} of {total}", "{shown} از {total}")` via `ui.f.tf(..)`.

### Step 4: Desktop — record detail sheet

Create `ui/detail.rs`:

- State: `pub struct DetailState { pub open: bool, pub record: serde_json::Value, pub kind: DetailKind }` on `State`.
- `pub fn open(st: &mut State, id: &str)` → `st.query("detail", json!({"id": id}))` (company is filled by `fill_books_context`); classify the kind from the JSON shape; on error toast it.
- `pub fn mount(s, ui, layer)`: a dialog (width 760) with: header facts (reference, date via `fmt.date`, party via `books.party_name`, total via `fmt.money`); for sales/purchases a lines table (item name, quantity via `fmt.qty`, unit price, line total); for payments the allocations; and for every kind the **journal lines** (account code + name, debit, credit) — if the record JSON does not embed its journal, find the journal by the record's journal id/source id among `books.list("journals")` or via a second `detail` call. Footer: Close; plus contextual actions that already exist elsewhere (Print PDF for invoices; "Open journal" link from a document to its journal).

Wire it: double-click/activate on a row if the kit supports it, else add a `Btn::Glass` "Open" header action enabled when a row is selected (and `Enter` on the selected row if the kit exposes it). Wire from Sales, Purchases, Payments, Accounting journals, Banking movements (each has a `journal_id`), and Overview's recent-entries panel.

Mount it from `ui/mod.rs` `root_view` next to the palette so it works from any page.

### Step 5: Desktop — full ledger

Replace `mount_ledger`'s 12 text lines with a kit `Table` (columns: date, reference, description, debit, credit, balance) in a 900-wide dialog, with the opening balance as a first muted row and a "Load older"-style pager using `offset`. Selecting a row and pressing Open shows the journal via `detail::open`.

**Drill-down from reports**: in Reports → Trial balance and Balance sheet, activating an account row opens this ledger for that account and the current range (reuse `open_ledger`; make it `pub(crate)` and parameterised by account id).

### Step 6: Tests (`main.rs`)

1. `test_load_older_appends_records` — seed >100 journals through `rt.state.run("journal_post", …)` in a loop (keep it to ~105 to stay fast), navigate to Accounting, assert the footer exists, click `btn-Load older`, assert the list length grew.
2. `test_invoice_detail_shows_lines_and_journal` — after the demo sale flow (pattern: `test_owner_walks_profiles_documents_and_books`), open detail for the invoice; assert `rt.find("detail-dialog")`, a lines element and a journal element exist.
3. `test_ledger_shows_running_balance`.
4. `test_trial_balance_row_opens_the_ledger`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `grep -rn '"offset"' crates/sarv-desktop/src | wc -l` ≥ 2
- [ ] `grep -rn '"detail"' crates/sarv-desktop/src/ui/detail.rs` matches
- [ ] `grep -n "take(12)" crates/sarv-desktop/src/ui/views/accounting.rs` returns nothing
- [ ] Engine and desktop suites pass with the new tests
- [ ] `plans/README.md` updated

## STOP conditions

- Record JSON does not contain enough to render lines (e.g. item names are ids only and the item was deleted): show the id and continue; but if totals cannot be derived without re-implementing engine math in the UI, stop — the engine should return a display-ready detail instead (propose the shape).
- Posting 100+ journals in a test takes more than ~20 s (each save is O(total data), ADR-0007): reduce by seeding through one `Store::change` in the engine test and skip the desktop paging test, noting it.

## Maintenance notes

- The UI must not compute accounting figures from record JSON beyond summing displayed lines; anything else belongs in the engine response.
- Saved filters and search across all history are natural follow-ups on the `records` action.
