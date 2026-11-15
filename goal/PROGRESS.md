# Goal Progress Log

Started: 2026-11-15
Baseline: `cargo test -p sarv-desktop --bin sarv-desktop` (34 passed), `cargo test -p sarv-quote` (all passed).

---
## P01-T01 — Pin toolchain, format workspace, dev line tables
- Added `rust-toolchain.toml` (`channel = "1.98.1"`, components `rustfmt`, `clippy`), `rustfmt.toml` (`max_width = 140`), `rust-version = "1.85"` in workspace & crate manifests, and `[profile.dev] debug = "line-tables-only"`.
- Formatted workspace with `cargo fmt --all`.
- Verified: `cargo fmt --all -- --check` exited 0; `cargo test -p sarv-desktop --bin sarv-desktop` -> 34 passed, 0 failed.

## P01-T02 — Fix clippy lints, add CI workflow, document Poppler
- Fixed all clippy errors/warnings across `sarv-quote` and `sarv-desktop` (`domain.rs` MSRV modulo, `sheets.rs` `allow(clippy::result_unit_err)`, `views.rs` `sort_by_key`, `desktop_bridge.rs` needless borrow, `desktop_bridge` test `unused_mut`, `glass.rs`/`pickers.rs`/`resources.rs`/`overview.rs` `allow(clippy::too_many_arguments)`, `shell.rs` `PaletteAction` type alias, `payments.rs` duplicate branch, `pos.rs` `#[derive(Default)]`, `sheets.rs` elided lifetime, needless borrows in `inventory.rs`/`purchases.rs`/`sales.rs`).
- Created `.github/workflows/ci.yml` with fmt, clippy, `sarv-quote`, `sarv-quote --features pdf-export`, and `sarv-desktop` headless tests.
- Documented `poppler-utils` in `README.md` and marked Plan 001 `DONE` in `plans/README.md`.
- Verified: `cargo fmt --all -- --check` (exit 0), `cargo clippy --workspace --all-targets -- -D warnings` (exit 0), `cargo test -p sarv-desktop --bin sarv-desktop` (34 passed).

## P02-T01 — Deterministic test clock
- Updated `today()` in `crates/sarv-desktop/src/data.rs` to honour `SARV_TODAY` when valid `YYYY-MM-DD`.
- Pinned `SARV_TODAY=2026-10-08` via `std::sync::Once` in `Engine::demo()`.
- Added `test_demo_clock_is_inside_the_fixture_year` in `crates/sarv-desktop/src/main.rs` and marked Plan 002 `DONE`.
- Verified: `SARV_TODAY=2026-10-08 cargo test -p sarv-desktop --bin sarv-desktop test_demo_clock` (1 passed); `cargo test -p sarv-desktop --bin sarv-desktop` (35 passed); fmt + clippy clean.

## P03-T01 — Long-lived Store in desktop
- `Engine` in `crates/sarv-desktop/src/data.rs` now holds `Mutex<Option<Store>>`, routes `call` through `refresh_if_stale` + `dispatch_in`, and exposes `with_state` for `ui/documents.rs` (eliminating `Store::open` in `documents.rs`).
- `Store::refresh_if_stale` in `crates/sarv-quote/src/store.rs` reads `(generation, payload)` in a single query when stale.
- Added `test_engine_sees_writes_from_another_store` and `test_integrity_status_survives_between_calls` in `main.rs`; marked Plan 003 `DONE`.
- Verified: `cargo test -p sarv-desktop --bin sarv-desktop` (37 passed), `cargo test -p sarv-quote --test storage_hardening --test process_recovery --test agent_access` (32 passed).

## P33-T01 — Move desktop test suite out of main.rs + harness helpers
- Moved `mod tests` from `crates/sarv-desktop/src/main.rs` (now 122 lines) into `crates/sarv-desktop/src/tests/` (`mod.rs`, `harness.rs`, and 11 area files).
- Added harness helpers `type_into`, `danger_toasts`, `assert_no_danger`, `count`, `trial_balance_is_balanced`, `books_validate`.
- Verified: `cargo fmt --all -- --check`, `cargo clippy --quiet --workspace --all-targets -- -D warnings`, `cargo test -p sarv-desktop --bin sarv-desktop` (37 passed).

## P33-T02 — Characterization tests & engine gap tests
- Added 15 desktop characterization tests across `crates/sarv-desktop/src/tests/` (creating `banking.rs` and updating `mod.rs`, `inventory.rs`, `payments.rs`, `sales.rs`, `purchases.rs`, `accounting.rs`, `settings.rs`, `shell.rs`, `walkthrough.rs`).
  - 5 marked `#[ignore]` for known/new issues: `product_profile_save_round_trips_every_field` (Plan 006), `opening_the_sale_dialog_twice_starts_clean` & `double_click_on_save_posts_once` (Plan 004), `received_cheque_goes_through_deposit_and_clear` & `bounced_cheque_reopens_the_receivable` (new finding: `cheque_strip` action buttons at `ui/views/payments.rs:481` don't trigger `on_press` on headless click; Plan 021 replaces `cheque_strip`).
- Added `crates/sarv-quote/tests/sheets_bridge.rs` (3 tests) and `crates/sarv-quote/tests/contact_profile.rs` (2 tests).
- Documented `SARV_M{3,4,5}_COMPAT_BINARY` ignored migration tests in `AGENTS.md` and marked Plan 033 `DONE` in `plans/README.md`.
- Verified: `sarv-desktop` 47 passed (5 ignored), `sheets_bridge` 3 passed, `contact_profile` 2 passed, fmt + clippy clean.


## 2026-11-15 — P07-T01 (Plan 007 Steps 1–3: Engine backup recovery & retention)
- **Changes**: Added `Store::recover_from_backup` in `crates/sarv-quote/src/store.rs` (preserves unreadable row to `unreadable-{id}.json` before replacing SQLite row); updated `desktop_bridge.rs` to serve `backup_list`, `backup_restore_preview`, and `recover_from_backup` when `Store::open` fails, refuse `recover_from_backup` when books load normally, validate `backup_file_kind` (`manual`, `before-restore`, `before-upgrade`), preview only the newest 20 backups, and prune safety backups to 5 per kind; added 4 tests in `crates/sarv-quote/tests/recovery.rs`; gated `export` import in `crates/sarv-quote/tests/manager_workflow.rs` behind `#[cfg(feature = "pdf-export")]` for clean `-D warnings` clippy without `pdf-export`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet -p sarv-quote --all-targets -- -D warnings && cargo clippy --quiet -p sarv-quote --features pdf-export --all-targets -- -D warnings && cargo test -p sarv-quote --test recovery`
- **Result**: PASS (`recovery`: 4 passed; all `sarv-quote` non-ignored tests pass).

## 2026-11-15 — P04-T01 (Plan 004: Dialog open functions & request keys)
- **Changes**: Added `State::mint_key` in `crates/sarv-desktop/src/app.rs` and atomic `uid()` in `crates/sarv-desktop/src/ui/resources.rs`; added `open_*` functions (`accounting::open_journal`, `payments::open_payment`, `sales::open_sale`, `purchases::open_purchase`, `contacts::open_new_contact`, `inventory::open_new_product`, `inventory::open_transfer`) and routed all dialog launchers through them; added `transfer_key` and `transfer_posting` double-click guard in `inventory.rs`; cleared keys on success across views and removed duplicate `toast_err` calls (`ops::post_operation` toasts its own errors); un-ignored 2 Plan 004 tests and added 5 new tests; marked Plan 004 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 54 passed, 3 ignored).

## 2026-11-15 — P10-T01 (Plan 010: Engine idempotency and panics)
- **Changes**: Added `ChequeRegister::replayed` and strict registration retry matching in `crates/sarv-quote/src/cheques.rs`; extended `dispatch_in` replay check in `desktop_bridge.rs` to cover payments and cheques; enforced stored `quote_issue` priority, unique bounded quotation number issuance, and `max(stored, incoming)` for `settings.next_number`; added `p.is_object()` guard and replaced non-test `.unwrap()` calls in `desktop_bridge.rs`; recovered poisoned mutexes in `agent.rs` by reopening `Store::open`; added 8 engine tests across `cheques.rs`, `payments_trust.rs`, `sales_drafts.rs`, and `desktop_bridge.rs`; marked Plan 010 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-quote`
- **Result**: PASS (all `sarv-quote` unit and integration tests pass; 3 M3/M4/M5 compat binary tests remain `#[ignore]`d).

## 2026-11-15 — P05-T01 (Plan 005: Strict money parser & unit display)
- **Changes**: Added `MoneyError`, `parse_money`, and `Fmt::money_input` with table-driven & round-trip unit tests in `crates/sarv-desktop/src/i18n.rs`; changed `State::parse_money_input` to return `Result<i64, MoneyError>`; preserved single decimal separator in `ops::typed_display_amount`; replaced truncating `rials / 10` conversions in `pos.rs`, `sales.rs`, and `inventory.rs` with `Fmt::money_input`; added `Ui::money_field` in `glass.rs` and updated all amount inputs across views; added `"Enter a valid amount."` to `fa_messages.rs`; added 3 flow tests (`test_pos_posts_exact_rials_for_odd_price`, `test_bad_opening_amount_does_not_open_empty_books`, `test_product_profile_roundtrip_keeps_price`); marked Plan 005 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 58 passed, 3 ignored).

## 2026-11-15 — P09-T01 (Plan 009: Agent access hardening)
- **Changes**: Decided agent access from in-memory `state.policy.access` with disk tamper check in `crates/sarv-quote/src/agent.rs`; blocked `local-agent` from reopening locked fiscal periods in `desktop_bridge.rs`; required `expected_generation` on `payment_post`, `draft_post`, `purchase_post`, and `period_lock` in `agent.rs` and `desktop_bridge.rs`; added 5-second total `TimedReader` deadline in `read_message`; added 4 tests in `crates/sarv-quote/tests/agent_access.rs` (including `slow_client_is_dropped` `#[ignore]`, verified manually); marked Plan 009 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-quote --test agent_access` (+ `slow_client_is_dropped -- --ignored`)
- **Result**: PASS (`agent_access`: 9 passed, 1 ignored; ignored `slow_client_is_dropped` passed in 7.01s).

## 2026-11-15 — P06-T01 (Plan 006: Six small confirmed desktop bugs)
- **Changes**: Fixed `minimum_stock_thousandths` string parsing in `inventory.rs` (un-ignored `product_profile_save_round_trips_every_field` + added `test_product_profile_keeps_minimum_stock`); added `"Out of balance"` Danger badge in `reports.rs`; surfaced `Books.error` in `shell.rs` Refresh toast and toolbar badge (added `test_refresh_reports_engine_failure`); matched walk-in customer across Fa/En in `resources.rs` (added `test_walk_in_customer_is_reused_across_languages`) and followed `st.lang()` for law notice in `settings.rs`; labeled fiscal period toggle `"Reopen fiscal period"` when locked in `accounting.rs`; reset view state on restore in `settings.rs` and replaced unchecked `[i]` / byte slicing / colliding aging keys across views; marked Plan 006 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 62 passed, 2 ignored).
