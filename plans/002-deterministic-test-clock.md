# Plan 002: Desktop tests stop depending on today's date

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/data.rs crates/sarv-desktop/src/app.rs crates/sarv-quote/src/desktop_bridge.rs`

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: tests
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

The demo fixture every desktop test uses has exactly one fiscal year, 2026-03-21 to 2027-03-20. The desktop stamps every post with the wall-clock date. The engine rejects a post dated outside its fiscal period. On 2027-03-21 the tests that post sales, purchases, payments and journals will start failing for a reason unrelated to any code change — and the `SARV_DEMO=1` app will be unusable the same day.

## Current state

- `crates/sarv-quote/src/desktop_bridge.rs:1885-1900` — `seed_fixture` loads `docs/assets/accounting-m3/restore/core-v4.json` via `include_str!`; that file's only period is `"start_date": "2026-03-21"`, `"end_date": "2027-03-20"` (lines 32-33 of the JSON).
- `crates/sarv-desktop/src/data.rs:63-65`:
  ```rust
  pub fn today() -> String {
      chrono::Local::now().format("%Y-%m-%d").to_string()
  }
  ```
  Called from `app.rs:325` (`fill_books_context`), `ui/ops.rs:59`, `ui/views/sales.rs`, `ui/views/purchases.rs`, `ui/views/payments.rs`, `ui/views/inventory.rs`, `data.rs` (`pick_period`, `range`).
- Engine check: `crates/sarv-quote/src/accounting.rs:1348-1351` ensures `command.date` is inside the period.
- Test harness: `crates/sarv-desktop/src/main.rs:126-134` — `make_test_runtime()` builds `State::with_engine(data::Engine::demo(), true)`.

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Future-date run | `SARV_TODAY=2027-06-01 cargo test -p sarv-desktop --bin sarv-desktop` | before the fix: failures; after Step 2: all pass |

## Scope

**In scope**: `crates/sarv-desktop/src/data.rs`, `crates/sarv-desktop/src/main.rs` (test module only).

**Out of scope**: the fixture JSON and `seed_fixture` (other engine tests assert on its exact contents); any engine file.

## Git workflow

- Branch `advisor/002-deterministic-test-clock`; commit `test(desktop): pin the clock to the demo fiscal year`.

## Steps

### Step 1: Make `today()` overridable

In `data.rs`, replace `today()` with:

```rust
/// Today's date as ISO `YYYY-MM-DD`. `SARV_TODAY` overrides the clock (tests, demos).
pub fn today() -> String {
    if let Ok(v) = std::env::var("SARV_TODAY") {
        if chrono::NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d").is_ok() {
            return v.trim().to_string();
        }
    }
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
```

**Verify**: `SARV_TODAY=2027-06-01 cargo test -p sarv-desktop --bin sarv-desktop 2>&1 | tail -5` → shows failures (this proves the hazard is real). If everything passes, see STOP conditions.

### Step 2: Pin the demo clock

In `Engine::demo()` (`data.rs`), when `SARV_TODAY` is not already set, set it to a date inside the fixture's year: `2026-10-08` (the date the engine's own bridge tests use). Use `unsafe { std::env::set_var("SARV_TODAY", "2026-10-08") }` (required form in edition 2024) guarded by `std::sync::Once`, with a comment that this runs before any other thread reads the variable in the demo/test path.

**Verify**:
- `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.
- `faketime` is not required: instead run `SARV_TODAY=2026-10-08 cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

### Step 3: Add a guard test

In the `tests` module of `main.rs`, add `test_demo_clock_is_inside_the_fixture_year`: build `make_test_runtime()`, read `rt.state.books.range()` and assert `from <= data::today() && data::today() <= to`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop test_demo_clock` → 1 passed.

## Test plan

- New: `test_demo_clock_is_inside_the_fixture_year` (pattern: `test_fiscal_period_follows_the_jalali_year` in the same module).
- Existing 34 tests must stay green.

## Done criteria

- [ ] `cargo test -p sarv-desktop --bin sarv-desktop` passes with 35+ tests
- [ ] `grep -n "SARV_TODAY" crates/sarv-desktop/src/data.rs` shows the override
- [ ] Only the two in-scope files changed (`git status`)
- [ ] `plans/README.md` updated

## STOP conditions

- Step 1's future-date run passes with no failures: the assumption "posts outside the period are rejected in these tests" is false; report what you observed instead of continuing.
- Existing tests fail under `SARV_TODAY=2026-10-08` (some test may assert on the real calendar year — report which).

## Maintenance notes

- Plan 017 (fiscal years) removes the underlying product problem by letting users open the next year; this plan only makes tests deterministic.
- Document `SARV_TODAY` in `AGENTS.md` when plan 032 rewrites the docs.
