# Plan 006: Fix six small confirmed desktop bugs

> **Executor instructions**: Follow this plan step by step. Each step is independent — commit each separately. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src`

## Status

- **Priority**: P1
- **Effort**: M (six S-sized fixes)
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Each item below is a small defect a real user will hit: a low-stock threshold that silently resets, an out-of-balance balance sheet labelled as "no data", a green "refreshed" toast when the books failed to load, a duplicate walk-in customer after switching language, an English legal notice in a Persian app, a period button with the wrong label, and a few panics waiting on unusual data.

## Repo conventions you need

- UI text is an English key translated through `ui.t("…")` / `st.fmt.t("…")`; Persian lives in `crates/sarv-desktop/src/fa_messages.rs` as `("English key", "فارسی"),`. Every new key needs an entry.
- Values from the engine arrive as JSON where **every number is a string** (`crates/sarv-quote/src/desktop_bridge.rs:41-47`, `wire()`). Read them with `crate::data::s(&value)` then parse; `.as_i64()` on engine JSON is always `None`.
- `State::run(action, payload, success_key)` refreshes the books and toasts errors itself. `State::query` does neither.
- Tests live in `crates/sarv-desktop/src/main.rs` `mod tests`; use `make_test_runtime()` and `click(&mut rt, "btn-<Label key>")`.
- Commit style: `fix(desktop): <what>`.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-desktop/src/ui/views/{inventory,reports,settings,accounting,payments,sales}.rs`, `src/ui/shell.rs`, `src/ui/resources.rs`, `src/data.rs`, `src/app.rs`, `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: engine crate; dialog open/reset logic (plan 004); money parsing (plan 005).

## Steps

### Step 1: Product profile save no longer resets minimum stock

`ui/views/inventory.rs:522`:
```rust
let min_th = profile["minimum_stock_thousandths"].as_i64().unwrap_or(0);
```
Always 0 because the value is a string. Replace with `crate::data::s(&profile["minimum_stock_thousandths"])` passed straight to `qty_from_thousandths` (which already parses a string). First confirm the field name: `grep -n "minimum_stock" crates/sarv-quote/src/product_profile.rs crates/sarv-quote/src/desktop_products.rs`.

Test `test_product_profile_keeps_minimum_stock`: via `rt.state.run("product_profile", json!({"item_id":"ITEM-A","profile":{…,"minimum_stock":"5"}}), None)` (payload shape: `main.rs:68-80`), open the profile through the UI function `open_product`, assert `forms.get("prod-min-stock") == "5"`.

**Verify**: new test passes.

### Step 2: Balance sheet says "Out of balance" when it is

`ui/views/reports.rs:249-256` shows `"No balances through this date"` when `balanced` is false. Change to three states: no rows → the existing empty text; balanced → `"Balanced"`; otherwise a danger-toned line `ui.t("Out of balance")` using `ui.badge(Tone::Danger, …)` or text coloured `ui.tone_fg(Tone::Danger)`. Add `("Out of balance", "تراز نیست"),`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → pass.

### Step 3: Show refresh failures

`data.rs` stores engine failures in `Books.error` (lines 87, 96) and nothing reads it. `ui/shell.rs:158-166` — the Refresh button always toasts success.

- In `shell.rs` Refresh handler: after `st.refresh()`, if `!st.books.error.is_empty()` toast `Tone::Danger` with the error; else the success toast.
- In `ui/mod.rs` `root_view`… do NOT edit `mod.rs`; instead in `shell::build_toolbar` add, when `s.books.error` is non-empty, a trailing `Badge` with `ui.t("Books could not be loaded")` in place of the "Saved on this device" badge. Add the FA entry `("Books could not be loaded", "دفاتر بارگذاری نشد"),`.

Test `test_refresh_reports_engine_failure`: build a runtime, point the engine at an unreadable location by setting `rt.state.books.company_id = "no-such-company".into()` then `rt.state.refresh()`; assert `!rt.state.books.error.is_empty()`. If the engine does not error on an unknown company, instead write garbage to `<dir>/sarv.sqlite3` after dropping the runtime's store — or skip this test and note it in the report.

### Step 4: Walk-in customer is found by a stable id, law notice follows the language

- `ui/resources.rs:159-172` finds the walk-in customer by its **translated** name, so switching language creates a second one. Change the lookup to match either label: build both `tr(Lang::Fa, "Walk-in customer")` and `tr(Lang::En, "Walk-in customer")` (`crate::i18n::tr`) and match a customer whose name equals either.
- `ui/views/settings.rs:209-216` prefers `notice_en`. Choose by language: `if st.lang() == Lang::Fa { notice_fa, fallback notice_en } else { notice_en, fallback notice_fa }`.

Test `test_walk_in_customer_is_reused_across_languages`: call `ensure_walk_in_customer()` in Fa, `set_lang(Lang::En)`, call again, assert same id.

### Step 5: Period button label and confirmation text

`ui/views/accounting.rs:196` always says "Lock fiscal period" but `toggle_period` (`:352-377`) also reopens. Compute the selected period's `locked` flag in the view (same lookup `toggle_period` does) and label the button `"Reopen fiscal period"` when locked, `"Lock fiscal period"` otherwise. Check both keys exist in `fa_messages.rs` (`grep -n "fiscal period" crates/sarv-desktop/src/fa_messages.rs`); add what is missing. (A confirmation dialog comes in plan 013.)

### Step 6: Restore resets view state; remove panics-in-waiting

- `ui/views/settings.rs` restore handler (near line 373): after a successful `backup_restore`, do what `State::select_company` does (`app.rs:307-313`): `st.sheets.loaded = false; st.pos = Default::default();` and clear `selected` in sales/purchases/payments/accounting; set `st.books.period_id.clear()` then `st.refresh()`.
- Replace unchecked indexing inside table cell closures with `.get(i)` and an empty cell fallback: `ui/views/accounting.rs` (near 77, 101), `ui/views/reports.rs` (near 68, 228, 295), `ui/views/sales.rs:403,413` (`sales_records(s)[i]`). Pattern to copy: any view that already uses `.get(i)` (`grep -n "\.get(i)" crates/sarv-desktop/src/ui/views/*.rs`).
- Replace byte slicing of ids with char-safe truncation: `&id[..id.len().min(6)]` at `sales.rs:831`, and the `[..8]`-style slices found by `grep -rn '\[\.\.' crates/sarv-desktop/src/ui/views`. Use `id.chars().take(N).collect::<String>()`.
- `ui/views/payments.rs:344`: `if … { "payment_preview" } else { "payment_preview" }` → just `"payment_preview"`.
- `ui/views/reports.rs` aging rows keyed `age-{party_id}` collide when a party is both receivable and payable: include the side in the key.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `grep -n 'minimum_stock_thousandths"\].as_i64' crates/sarv-desktop/src` returns nothing
- [ ] `grep -n "No balances through this date" crates/sarv-desktop/src/ui/views/reports.rs` appears only on the empty branch
- [ ] `grep -rn "books.error" crates/sarv-desktop/src/ui` returns at least one reader
- [ ] Desktop tests pass with the new tests
- [ ] `plans/README.md` updated

## STOP conditions

- The profile field in Step 1 is not `minimum_stock_thousandths` in the engine response.
- Fixing indexing requires restructuring a table's row source (leave it and report; plan 012 rebuilds row models).

## Maintenance notes

- Anyone reading engine JSON should use `data::s`; consider a clippy `disallowed_methods` entry for `Value::as_i64` in the desktop crate later.
