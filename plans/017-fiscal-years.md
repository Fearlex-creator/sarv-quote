# Plan 017: Users can open the next fiscal year and switch between years

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-quote/src/desktop_bridge.rs crates/sarv-quote/src/accounting.rs crates/sarv-quote/src/agent.rs crates/sarv-desktop/src/data.rs crates/sarv-desktop/src/ui/shell.rs crates/sarv-desktop/src/ui/views/accounting.rs`

## Status

- **Priority**: P1 — without this the product stops working after its first fiscal year
- **Effort**: M
- **Risk**: MED
- **Depends on**: 002 (test clock) and 014 (date field) recommended
- **Category**: direction (feature)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

A fiscal period is created exactly once, by first-run setup. The engine rejects any post dated outside its period. There is no action to create another period and no way to choose which period the app is looking at. On the first day of the next fiscal year every sale, purchase, payment and journal fails with "date outside the selected fiscal year", and the user has no remedy. The engine function that creates a period already exists and is replay-safe; it is simply not reachable.

This plan adds **opening** a new year and **switching** years. It does not add year-end closing entries (moving profit to retained earnings) — that is designed in plan 027.

## Current state

- `crates/sarv-quote/src/accounting.rs:911-947`:
  ```rust
  pub fn add_period(&mut self, company_id: &str, name: &str, start: &str, end: &str, actor: &str) -> Result<String> {
      …
      ensure!(start_date <= end_date, "پایان سال مالی پیش از شروع آن است");
      ensure!(!self.periods.iter().any(|p| p.company_id == company_id && start <= p.end_date.as_str() && end >= p.start_date.as_str()),
          "سال مالی با دورهٔ موجود هم‌پوشانی دارد");
      let period_id = id();
      self.periods.push(FiscalPeriod { id, company_id, name, start_date, end_date, locked: false });
      self.activity(company_id, &period_id, AuditOperation::PeriodCreated, actor, "ایجاد سال مالی");
      Ok(period_id)
  }
  ```
  Its only caller outside replay is setup (`crates/sarv-quote/src/core.rs:587`).
- `crates/sarv-quote/src/desktop_bridge.rs` — `dispatch_in` is one big `match action { … }` starting ~line 1153. Model the new arm on `"period_lock"` (~lines 1676-1706): it reads `company(&store.state, &p)`, uses `actor(&p)`, `string(&p, "…")?`, and wraps the mutation in `store.change(|s| …)`. Helper `civil_date(text)?` validates an ISO date (used in the `"ledger"` arm).
- The `workspace` response already includes `"periods"` for the company (`desktop_bridge.rs:1008`), each with `id, name, start_date, end_date, locked`.
- Whether the rest of the engine assumes one period per company is the key unknown. Check before building: `grep -n "periods\[0\]\|periods.first()\|\.periods\.iter()\.find" crates/sarv-quote/src/*.rs` and read each hit. Sales/procurement/payment commands carry a `period_id` in their context (`context(&store.state, &p)`), which the desktop fills from `books.period_id` (`crates/sarv-desktop/src/app.rs:321-323`).
- Reports that are cumulative (balance sheet, aging, party balances) must keep including earlier years; `report` takes `from`/`to` dates (`crates/sarv-desktop/src/data.rs:83-90` passes the selected period's range). Read `fn report` in `desktop_bridge.rs` (~line 860) to see which parts use `from` (P&L, trial movement) and which use only `to` (position, balance sheet).
- Desktop period handling — `crates/sarv-desktop/src/data.rs:101-113` `pick_period`: keeps `period_id` if still valid, else the period containing today, else the first. `range()` returns the selected period's dates. `State::select_company` clears `period_id`.
- Toolbar badge: `crates/sarv-desktop/src/ui/shell.rs:171-175` shows `Fiscal year {name}` as a static `Badge`.
- Accounting page has a periods tab and the "Lock fiscal period" button (`ui/views/accounting.rs:194-196`).
- Jalali helpers: `sarv_quote::calendar::{Calendar, from_gregorian, format_date, parse_date}`; setup computes a Jalali fiscal year's ISO bounds in `crates/sarv-desktop/src/ui/views/setup.rs` (top of file, the function returning `(name, start_iso, end_iso)`); reuse it.
- Agent allow-lists: `crates/sarv-quote/src/agent.rs:43-69` (`WRITE`). Do **not** add the new action there.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Engine tests | `cargo test -p sarv-quote` | all pass |
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-quote/src/desktop_bridge.rs` (new `period_add` arm), `crates/sarv-quote/tests/desktop_bridge.rs` (or new `tests/fiscal_years.rs`), `crates/sarv-desktop/src/data.rs`, `src/app.rs`, `src/ui/shell.rs`, `src/ui/views/accounting.rs`, `src/ui/views/setup.rs` (make the fiscal-year helper `pub(crate)`), `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: closing entries / retained earnings (plan 027); changing `add_period`'s validation; exposing `period_add` to the agent.

## Git workflow

- Branch `advisor/017-fiscal-years`; commits `feat(engine): period_add action`, `feat(desktop): open and switch fiscal years`.

## Steps

### Step 1: Prove multi-period books work in the engine (test first)

Write `crates/sarv-quote/tests/fiscal_years.rs` using the helpers from `tests/desktop_bridge.rs:1-19`. Because the action does not exist yet, the first version calls the engine directly:

```rust
let mut store = Store::open(d.path()).unwrap();
let second = store.change(|s| s.accounting.add_period(&c, "1406", "2027-03-21", "2028-03-20", "test")).unwrap();
```

then, through `dispatch`, post a journal dated `2027-04-01` with `period_id = second` (payload shape: the `journal_preview`/`journal_post` usage in `tests/desktop_bridge.rs`), and assert: (a) it succeeds; (b) `report` for the second year's range shows it; (c) `report` for the first year's range does not show it in P&L; (d) the balance-sheet / position figures as of the second year's end include first-year balances; (e) reopening the store (`Store::open`) still validates.

**Verify**: `cargo test -p sarv-quote --test fiscal_years` → passes. If it fails, that is a STOP condition (the engine has a single-period assumption) — report the failing assertion and the error text.

### Step 2: `period_add` action

Add the arm to `dispatch_in`:

- Inputs: `company_id`, `name`, `start_date`, `end_date` (ISO), `expected_generation` (required: `ensure!` it is present and equals `store.local_generation()` — copy the comparison at `desktop_bridge.rs:1143-1151`).
- Additional rule the engine function does not enforce: the new period must start the day after the company's latest period ends (no gaps), unless the company has no periods. Error text: "The new fiscal year must start the day after the previous one ends."
- Returns `{"period_id": id}`.

Extend the test file: `period_add` happy path; overlap rejected; gap rejected; stale generation rejected.

**Verify**: `cargo test -p sarv-quote` → all pass.

### Step 3: Desktop — choose the year

- `shell.rs`: replace the static fiscal-year `Badge` with a control that lists the company's periods (newest first) and sets `st.books.period_id` then calls `st.refresh()`. Use the kit `Select` if the toolbar can host it (see how `selects: HashMap<String, bool>` in `State` and `pickers.rs` `select_field` are used); otherwise make the badge a button that opens a small dialog listing periods as rows. Element key `fiscal-year-switch`.
- `data.rs` `pick_period`: unchanged logic, but when today is **after** the last period's end, select the last period (not the first).
- When the selected period does not contain today, posting dialogs would send today's date and fail. With plan 014 landed the date field shows "Outside the selected fiscal year"; additionally show a persistent toolbar hint badge `ui.t("Viewing a past fiscal year")` in that case.

### Step 4: Desktop — open the next year

On the Accounting page add a `Btn::Glass` action "New fiscal year" (`btn-New fiscal year`). It opens a dialog (use `DIALOG_SM`/480 width) pre-filled with: name = next year label, start = day after the latest period's end, end = one calendar year later minus a day **in the active calendar** (for Jalali, the end of the Jalali year — reuse the setup helper so leap years are right). Show both dates with `s.fmt.date(..)`; allow editing only the name. Save: take `_generation` from `st.books.workspace` (confirm the workspace response carries it; otherwise call `st.query("integrity", …)` or any read that returns `_generation`), call `st.run("period_add", payload, Some("Fiscal year opened"))`, then set `st.books.period_id` to the new id and refresh.

Also prompt proactively: on Overview, when today is within 30 days of the selected (latest) period's end or past it, show an info card "Your fiscal year ends on {date}. Open the next year to keep posting." with the same button. Find where Overview builds its notice cards (`grep -n "IntegrityNotice\|integrity" crates/sarv-desktop/src/ui/views/overview.rs`) and add it there.

Add Persian strings: `("New fiscal year", "سال مالی جدید")`, `("Fiscal year opened", "سال مالی باز شد")`, `("Viewing a past fiscal year", "در حال مشاهدهٔ سال مالی گذشته")`, and the Overview sentence.

### Step 5: Desktop tests (`main.rs`)

1. `test_open_next_fiscal_year_and_post_in_it`: click `btn-New fiscal year`, save; assert two periods; with `SARV_TODAY` semantics from plan 002 unavailable per-test, post a journal with an explicit date in the new year (plan 014's `journal-date`; if 014 has not landed, call `rt.state.run("journal_post", …)` with an explicit `date` and `period_id`).
2. `test_switching_fiscal_year_changes_the_report_range`: switch to the new year, assert `rt.state.books.range()` equals its bounds.
3. Existing `test_fiscal_period_follows_the_jalali_year` and `test_period_label_is_locked_or_open` still pass.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `grep -n '"period_add"' crates/sarv-quote/src/desktop_bridge.rs` matches; `grep -n "period_add" crates/sarv-quote/src/agent.rs` does not
- [ ] `cargo test -p sarv-quote --test fiscal_years` passes (≥5 tests)
- [ ] Desktop suite passes with 2 new tests
- [ ] `plans/README.md` updated

## STOP conditions

- Step 1 fails: some engine path (sales, procurement, payments, validation, reports) assumes a single period or mishandles a second one. Report precisely; do not patch the kernel under this plan.
- Opening balances are refused for the second period in a way that makes cumulative balances wrong (`accounting.rs:1380-1385` refuses a second opening — expected and fine as long as balances carry by date; if they do not, stop).
- P&L for the new year includes the previous year's income (it should not): reports need a closing concept first → stop and point at plan 027.

## Maintenance notes

- Until plan 027 lands, the balance sheet's equity shows cumulative unclosed earnings; `statements.rs:28` already documents that ("not yet closed into equity by a year-end entry"). Mention it in release notes.
- If the agent should ever open fiscal years, add `period_add` to `agent.rs` `WRITE` **and** to the preview-first list deliberately.
