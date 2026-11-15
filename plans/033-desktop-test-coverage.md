# Plan 033: The desktop's money-moving flows are covered by headless tests, and tests live outside `main.rs`

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/main.rs crates/sarv-desktop/src/test_helper.rs crates/sarv-desktop/tests`

## Status

- **Priority**: P2 — but do Steps 1-2 early if many other plans are about to add tests
- **Effort**: M
- **Risk**: LOW (tests only; one mechanical move)
- **Depends on**: 002 (test clock). Steps 3-4 are **characterization tests**: write them against today's behaviour before plans 004, 005, 013, 020 change these flows, or adapt them if those plans already landed.
- **Category**: tests
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

The desktop's 34 headless tests live in a 900-line `mod tests` inside `main.rs` (the file is 1,039 lines; the application itself is the first 119). They mostly set state fields directly and assert that elements exist. Several flows that move money or stock are never exercised end to end through the UI: cash/bank movements, stock transfers, cheque lifecycle actions, returns, period locking, backup-then-restore, and opening a dialog twice. The audit that produced these plans found seven real bugs in exactly those untested paths (stale request keys, a double-posting transfer, a reset stock threshold, leaked form values, truncated prices, an invisible refresh error, a mislabelled balance sheet). On the engine side, three modules have no tests at all.

## Current state

- `crates/sarv-desktop/src/main.rs:121-146` — the harness:
  ```rust
  fn make_test_runtime() -> Runtime<State> {
      let env = Env { system_dark: true, scale: 2.0 };
      let mut rt = Runtime::new(State::with_engine(data::Engine::demo(), true), ui::root_view, (1180.0, 780.0), env);
      rt.frame(0.0);
      rt
  }
  fn click(rt: &mut Runtime<State>, key: &str) {
      let target = key.to_string();
      rt.update(|_, tx| tx.scroll_into_view(target));
      rt.frame(1.0 / 60.0);
      let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing element `{key}`"));
      rt.pointer_down(x, y);
      rt.pointer_up(x, y);
      rt.frame(1.0 / 60.0);
  }
  ```
  Useful facts: `rt.state` is the `State`; `rt.find(key) -> Option<(f32, f32)>`; mutate state with `rt.update(|s, _| …)` so the view is marked dirty (mutating `rt.state` directly does not re-render — recorded in the project's notes); element keys follow `btn-{English label}`, `field-{form key}`, `nav-{page}`; form text is set with `rt.state.forms.set("key", "value")` (or via `rt.update`).
- `crates/sarv-desktop/src/test_helper.rs` — 5 lines; see what it holds.
- Existing tests (names at `main.rs:148-981`): rendering/navigation (≈10), dialogs exist (≈4), `test_journal_post_uses_preview_generation`, `test_rial_sale_is_not_scaled`, `test_payment_allocates_across_open_items`, `test_export_writes_csv_and_language_persists`, `test_backup_and_law_notice`, `test_owner_walks_profiles_documents_and_books` (a 200-line walk-through), `test_first_run_is_a_welcome_then_the_books`, `test_sheet_formulas_save_beside_the_books`, `test_pos_charges_the_catalog_and_keeps_the_books`.
- The crate is binary-only: `[[bin]] name = "sarv-desktop" path = "src/main.rs"`, with `pub mod app; pub mod data; …` declared in `main.rs`. Integration tests under `tests/` cannot import a binary's modules; `tests/png_scenes.rs` runs the built binary and needs a GPU.
- `Engine::demo()` (`crates/sarv-desktop/src/data.rs:29-39`) seeds a temp directory per call: items such as `ITEM-A`, one company, one fiscal year 2026-03-21…2027-03-20. Inspect what else it contains (warehouses, parties, bank/cash accounts) by printing `rt.state.books.workspace["resources"]` once.
- Engine modules without tests: `crates/sarv-quote/src/cashbook.rs` (395 lines), `contact_profile.rs`, `views.rs` (744 lines); `sheets.rs` has one inline test, and `sheet_save`/`sheet_load` through `dispatch` (`crates/sarv-quote/src/desktop_bridge.rs:1223-1230`) are only covered from the desktop.
- Three engine migration tests are permanently ignored: `crates/sarv-quote/tests/purchase_return_migration.rs:175`, `procurement_migration.rs:101`, `sales_migration.rs:95` — `#[ignore = "requires retained immutable M{3,4,5} executable in SARV_M*_COMPAT_BINARY"]`.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass (34 at planning time) |
| One test | `cargo test -p sarv-desktop --bin sarv-desktop <name> -- --nocapture` | passes |
| Engine tests | `cargo test -p sarv-quote` | all pass |

## Scope

**In scope**: `crates/sarv-desktop/src/main.rs` (remove the test module), new `crates/sarv-desktop/src/tests/` directory (`mod.rs`, `harness.rs`, one file per area), `crates/sarv-desktop/src/test_helper.rs`, new engine tests `crates/sarv-quote/tests/{sheets_bridge,contact_profile}.rs`, `docs/VERIFICATION.md` or `docs/history/…` only to add a note about the ignored migration tests (or `AGENTS.md`, one line).

**Out of scope**: changing application behaviour — if a characterization test reveals a bug, record it (see Step 4) instead of fixing it here; splitting the crate into lib + bin; GPU snapshot comparisons.

## Git workflow

- Branch `advisor/033-desktop-test-coverage`; commits `test(desktop): move the headless suite out of main.rs`, `test(desktop): harness helpers`, then one commit per flow file.

## Steps

### Step 1: Move the suite (pure move, no edits to test bodies)

Create `crates/sarv-desktop/src/tests/mod.rs` and move the body of `mod tests` there, split by area into files: `shell.rs` (rendering, navigation, sidebar, theme, language, palette, keyboard), `sales.rs`, `purchases.rs`, `payments.rs`, `accounting.rs`, `inventory.rs`, `settings.rs`, `setup.rs`, `sheets.rs`, `pos.rs`, `walkthrough.rs` (the long owner test). In `main.rs` leave only `#[cfg(test)] mod tests;`. Each file starts with `use super::harness::*;` plus what it needs from `crate::…`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop 2>&1 | tail -3` → the same number of tests as before the move, all passing; `wc -l crates/sarv-desktop/src/main.rs` < 140.

### Step 2: Harness helpers (`src/tests/harness.rs`)

Move `make_test_runtime` and `click` there and add:

```rust
pub fn type_into(rt: &mut Runtime<State>, form_key: &str, text: &str)      // rt.update(|s,_| s.forms.set(..)); rt.frame(..)
pub fn danger_toasts(rt: &Runtime<State>) -> Vec<String>
pub fn assert_no_danger(rt: &Runtime<State>)                                // panics listing the messages
pub fn count(rt: &Runtime<State>, key: &str) -> usize                       // books.count("journal_count") etc.
pub fn trial_balance_is_balanced(rt: &Runtime<State>) -> bool               // from books.report["trial"] totals — print the JSON once to find the fields
pub fn books_validate(rt: &Runtime<State>)                                  // sarv_quote::store::Store::open(&rt.state.engine.dir).expect("books reopen and validate")
```

`books_validate` is the important one: after every flow test, reopening the store runs the engine's full validator — the strongest cheap assertion available.

### Step 3: Characterization tests for uncovered flows

For each, drive the UI with `click`/`type_into` (set picker selections through state when a combobox cannot be driven headlessly — note it in a comment), then assert the ledger effect and call `assert_no_danger` + `books_validate`.

| File | Test | Asserts |
|---|---|---|
| `banking.rs` | `money_in_posts_one_balanced_journal` | journal count +1; the bank account's closing balance in `books.banking` rose by the amount; trial balance balanced |
| `banking.rs` | `transfer_between_cash_and_bank_moves_the_amount` | cash −X, bank +X; position cash+bank unchanged |
| `inventory.rs` | `stock_transfer_moves_quantity_between_warehouses` | needs two warehouses — create the second via the UI's "New warehouse" flow or `rt.state.run("resource", …)` (payload shape: `crates/sarv-desktop/src/ui/resources.rs`); source −q, target +q, total unchanged |
| `inventory.rs` | `product_profile_save_round_trips_every_field` | open profile, save unchanged, reopen: every `prod-*` field identical (this is the test that exposes the minimum-stock reset if plan 006 is not done) |
| `payments.rs` | `received_cheque_goes_through_deposit_and_clear` | statuses change in order; the bank balance rises only at clear |
| `payments.rs` | `bounced_cheque_reopens_the_receivable` | after bounce the customer's open amount is back |
| `sales.rs` | `sale_return_restocks_and_credits_the_customer` | stock back; receivable reduced |
| `sales.rs` | `opening_the_sale_dialog_twice_starts_clean` | no leftover line fields, a new `post_key` |
| `purchases.rs` | `purchase_receive_and_bill_updates_stock_and_payables` | stock +q; payables +amount |
| `purchases.rs` | `purchase_return_reduces_stock_and_payables` | |
| `accounting.rs` | `locked_period_refuses_a_journal` | lock; a journal post shows exactly one danger toast; count unchanged; reopen; post succeeds |
| `accounting.rs` | `reverse_creates_a_mirror_journal` | count +1; trial balance totals unchanged for the affected accounts |
| `settings.rs` | `backup_then_restore_returns_the_books_to_the_snapshot` | backup; post a journal; restore the backup; journal count equals the pre-journal count; `books_validate` |
| `shell.rs` | `refresh_after_external_write_shows_the_new_data` | write through a second `sarv_quote::desktop::dispatch` on the same dir; refresh; UI state reflects it |
| `walkthrough.rs` | `double_click_on_save_posts_once` (for journal, payment, sale: invoke the Save button twice without a frame in between if the harness allows, else twice in a row) | each posts exactly once |

### Step 4: When a characterization test exposes a bug

Do not fix application code in this plan. If the bug matches one already described in `plans/` (004: stale keys, transfer double post, leaked fields; 005: truncation/parse; 006: minimum stock, balance-sheet label, refresh error), write the test asserting the **correct** behaviour and mark it `#[ignore = "fixed by plans/00N-…"]`, and list it in your report. The plan that fixes it removes the `ignore`. If it is a new bug, do the same with `#[ignore = "new finding: <one line>"]` and describe it in your report with `file:line`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all non-ignored tests pass; `cargo test -p sarv-desktop --bin sarv-desktop -- --ignored 2>&1 | tail -15` → shows which ignored tests fail (expected for known bugs).

### Step 5: Engine gaps

- `crates/sarv-quote/tests/sheets_bridge.rs`: `sheet_save` then `sheet_load` through `dispatch` round-trips a workbook with a formula; a second company's sheet is isolated; an oversized payload is rejected if the engine has a limit (read `sheets.rs` for one). Helpers as in `tests/desktop_bridge.rs:1-19`.
- `crates/sarv-quote/tests/contact_profile.rs`: `party_profile` save + `contact_detail` read round-trip of every profile field; editing a profile does not change a posted document's snapshot (README: "editing them preserves posted document snapshots") — post a sale, edit the customer's name/address, read the sale via `detail`, assert the snapshot fields are unchanged.
- `cashbook.rs` and `views.rs`: first check whether anything reachable uses them (`grep -rn "cashbook\|views::" crates/sarv-quote/src --include=*.rs | grep -v "^crates/sarv-quote/src/\(cashbook\|views\).rs" | head`). If `cashbook` is only held in state and never driven by an action, do **not** write tests for dead paths — record it as an owner decision ("personal mode: build or remove") in `plans/README.md`.

### Step 6: Document the ignored migration tests

Add to `AGENTS.md` (one line) or `docs/VERIFICATION.md`/its successor: the three `#[ignore]`d compatibility tests need the retained M3/M4/M5 binaries via `SARV_M3_COMPAT_BINARY` etc. (take exact variable names from the `#[ignore]` strings and the test bodies), where those binaries are expected to come from (if the repo does not say, write "not recorded — owner to supply"), and the command to run them (`cargo test -p sarv-quote -- --ignored`).

## Done criteria

- [ ] `wc -l < crates/sarv-desktop/src/main.rs` is under 140
- [ ] `ls crates/sarv-desktop/src/tests/` shows `harness.rs` and at least 10 area files
- [ ] `cargo test -p sarv-desktop --bin sarv-desktop` passes with ≥ 34 + 15 tests (ignored ones listed in the report with their reason)
- [ ] `cargo test -p sarv-quote --test sheets_bridge --test contact_profile` passes
- [ ] No file under `crates/sarv-desktop/src/ui` or `src/app.rs`, `src/data.rs` changed (`git diff --stat`)
- [ ] `plans/README.md` updated

## STOP conditions

- After the pure move in Step 1 the test count differs or a test fails — the move was not pure; fix the move, do not edit test logic.
- A flow cannot be driven headlessly at all (e.g. a picker with no state hook and no element key). Skip that test, list it, and name the missing key so a later plan can add it.
- The harness's `Engine::demo()` data lacks something every banking/cheque test needs (e.g. no bank account) and creating it requires a multi-step setup — build one shared `fn books_with_bank()` helper rather than repeating it; if even that is impossible through actions, report.

## Maintenance notes

- New feature plans should add their tests to the matching area file and use `books_validate` at the end.
- If the crate is ever split into `lib` + thin `bin`, these tests can move to `crates/sarv-desktop/tests/` unchanged.
