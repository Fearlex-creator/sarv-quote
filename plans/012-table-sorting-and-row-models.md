# Plan 012: Table columns that show a sort arrow actually sort, and rows are computed once per refresh

> **Executor instructions**: Follow this plan step by step. Convert one view at a time and run the tests after each. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/ui/views crates/sarv-desktop/src/data.rs`

## Status

- **Priority**: P2
- **Effort**: M–L (eight views, mechanical after the first)
- **Risk**: LOW–MED
- **Depends on**: 004 and 006 (same files; land those first to avoid conflicts)
- **Category**: bug / perf
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

25 table columns across the app are declared `.sortable()`, which makes the kit draw a clickable sort arrow — but no table registers `on_sort`, so clicking does nothing. The kit's contract (rune-kit `table.rs`): "A header click sets the sort and calls `Table::on_sort`, where the app reorders its rows."

Separately, every table's `cell` callback re-derives the whole list for every visible cell on every frame: `Books::list()` clones the JSON array, and some views re-run the search filter too. With a few thousand records this turns scrolling into visible lag and defeats the table's virtualisation.

## Current state

- Kit API (pinned rune-kit, `crates/rune-kit/src/table.rs`):
  ```rust
  pub enum SortDir { Ascending, Descending }
  impl TableState { pub fn sort(&self) -> Option<(&str, SortDir)>; pub fn set_sort(&mut self, column: impl Into<String>, dir: SortDir); }
  impl Table { pub fn on_sort(mut self, f: impl Fn(&mut S, &str, SortDir, &mut Tx) + 'static) -> Self }
  ```
- A typical table today — `crates/sarv-desktop/src/ui/views/sales.rs:400-425`:
  ```rust
  Table::new(
      |s: &State| &s.sales.table,
      |s: &mut State| &mut s.sales.table,
      |s: &State| sales_records(s).len(),
      |s: &State, i| Cow::Owned(format!("sale-{}", crate::data::s(&sales_records(s)[i]["id"]))),
  )
  .column(TableColumn::new("customer", ui.t("Customer")).flex(2.0).min(140.0).sortable())
  .column(TableColumn::new("amount", ui.t("Amount")).width(160.0).min(100.0).align(Align::End).sortable())
  .cell(|s: &State, i, col| {
      let record = &sales_records(s)[i];
      match col { "ref" => Cell::text(sales_ref(record)), … }
  })
  ```
  `sales_records(s)` rebuilds a `Vec<Value>` each call.
- `crates/sarv-desktop/src/data.rs:157-159`: `pub fn list(&self, key: &str) -> Vec<Value> { self.workspace[key].as_array().cloned().unwrap_or_default() }`.
- Where `.sortable()` appears: `grep -rn "sortable()" crates/sarv-desktop/src/ui/views` (sales, purchases, contacts, inventory, reports, banking, accounting, payments).
- `State::refresh()` (`app.rs:303-305`) is the single place the books reload; `State::run` calls it after every successful post.
- Search fields that filter lists: Contacts (`filtered_parties`), Inventory, Purchases (`filtered`).

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: a new `crates/sarv-desktop/src/ui/rows.rs` (+ `pub mod rows;` in `ui/mod.rs`), the eight view files' table code, `src/app.rs` (`refresh` hook), `src/main.rs` (tests).

**Out of scope**: pagination beyond the engine's 100-row window (plan 019); changing column sets; the Sheets grid.

## Git workflow

- Branch `advisor/012-table-sorting-and-row-models`; one commit per view: `feat(desktop): sortable sales table`, etc.

## Steps

### Step 1: Shared row model

Create `ui/rows.rs`:

```rust
/// One table row prepared for display and sorting. Built once per refresh or filter change.
#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    /// Display text per column, already formatted (digits, money, date).
    pub cells: Vec<String>,
    /// Sort key per column: numbers sort numerically, text by normalised string.
    pub keys: Vec<SortKey>,
}
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum SortKey { Num(i128), Text(String) }

pub fn sort_rows(rows: &mut [Row], column: usize, dir: rune_kit::SortDir)
```

`sort_rows` must be a **stable** sort (`sort_by`) so equal keys keep engine order; `Descending` reverses the comparison, not the slice. Text keys: lowercase and fold Arabic `ي`→`ی`, `ك`→`ک` so Persian names sort consistently. Unit-test `sort_rows` (numbers, text, stability, descending).

Check the exact import path of `SortDir` (`grep -n "pub use.*SortDir\|pub mod table" ~/.cargo/git/checkouts/rune-*/b556a38/crates/rune-kit/src/lib.rs`).

### Step 2: Convert the Sales invoices table (the pattern)

1. Add `rows: Vec<Row>` and `rows_stamp: u64` to `SalesState`.
2. Write `fn build_rows(s: &State) -> Vec<Row>` from the existing `sales_records`, `sales_ref`, `sales_party`, `sales_date`, `sales_amount` helpers: `cells` holds what the cell callback shows today; `keys` holds `Num(amount)` for the amount column, `Text(iso_date)` for the date, `Text(name)` otherwise.
3. Rebuild when data or formatting changes. Add `pub stamp: u64` to `State`, incremented in `State::refresh()`, `set_lang`, and wherever `fmt` or `dark` changes are applied (`apply_settings`, the Settings switches). Provide `pub fn ensure(st: &mut State)` in the view that rebuilds when `st.sales.rows_stamp != st.stamp`, then applies the current `table.sort()`. Call `ensure` from `State::refresh` for every converted view (a single `views::ensure_rows(self)` function that calls each view's `ensure`), and from `set_lang`.
4. Table closures read `s.sales.rows`: count = `rows.len()`, id = `rows[i].id`, cell = `rows.get(i).map(|r| Cell::text(r.cells[c].clone()))` with the existing `.muted()` / `.mono()` modifiers per column.
5. `.on_sort(|st, col, dir, _tx| { st.sales.table.set_sort(col, dir); let c = column_index(col); sort_rows(&mut st.sales.rows, c, dir); })`.
6. Mark every text/number/date column `.sortable()` — not only the two that have it today.

Test `test_sales_table_sorts_by_amount`: make runtime, navigate to Sales, call the sort handler path by clicking the header (find its key by inspecting `rt.find` on `"sales-table"` children; if header cells have no stable key, call `sort_rows` through a small `pub fn apply_sort(st, "amount", SortDir::Descending)` in the view and test that) and assert `rows` amounts are non-increasing.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

### Step 3: Convert the remaining tables

Same pattern, one commit each: Sales drafts tab, Purchases, Payments, Banking movements, Contacts, Inventory, Accounting (chart, journals, periods), Reports (trial balance, P&L, parties, balance sheet, aging).

For views with a search box (Contacts, Inventory, Purchases): the filter runs inside `build_rows`; rebuild when the query text changes — store `rows_query: String` and compare in `ensure`, and call `ensure` at the top of the view function through a cheap check... the view function only has `&State`, so instead rebuild in the field's edit path: wrap the search field's `on_edit` so that after the edit the view's `rows_stamp` is reset to 0 and `ensure` runs on the next `refresh`-independent hook. Simplest correct approach: keep an unfiltered `rows` in state and filter by index in a cached `visible: Vec<usize>` that is recomputed when `rows_query != forms.get("…-search")`; perform that comparison in a `pub fn sync(st: &mut State)` called from the key handler of the search field or, if no hook exists, from `State::nav` and from the search field's `on_edit` closure after it returns the `TextEdit` (see how `Ui::field` wires `on_edit` in `ui/glass.rs:151-160`). If neither hook can run code after an edit, see STOP conditions.

`test_contacts_search_filters_the_table` must keep passing unchanged.

### Step 4: Remove per-cell list building

After all conversions: `grep -rn "\.cell(" -A6 crates/sarv-desktop/src/ui/views | grep -n "books.list\|filtered\|_records(s)"` must be empty.

## Done criteria

- [ ] `grep -rn "on_sort" crates/sarv-desktop/src/ui/views | wc -l` ≥ the number of tables (about 14)
- [ ] The Step 4 grep is empty
- [ ] `sort_rows` unit tests and at least 3 view sort tests pass; full desktop suite passes
- [ ] `plans/README.md` updated

## STOP conditions

- There is no way to run state-mutating code after a search field edit (Step 3). Report it; an acceptable fallback is to keep search-filtered views computing `visible` in the view function **once per frame** (not per cell) by building it before `Table::new` and moving it into the closures with `Rc`.
- Row selection breaks (the selected id no longer matches after sorting): selection must be by `Row.id`, never by index — if a view stores an index, report it.
- Existing tests that read cells by row index now fail because order changed: default order (no sort chosen) must equal today's order.

## Maintenance notes

- New tables must use `Row`; reviewers should reject list-building inside `.cell`.
- When plan 019 adds paging, sorting applies to the loaded window only; say so in the UI ("sorted within the rows shown") or move sorting into the engine query.
