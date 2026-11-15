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

## 2026-11-15 — P07-T02 (Plan 007 Steps 4–5: Desktop recovery screen)
- **Changes**: Added `Books::load_failed` in `crates/sarv-desktop/src/data.rs`; routed failed loads to new `views::recovery::view` (no sidebar) in `ui/mod.rs`; created `crates/sarv-desktop/src/ui/views/recovery.rs` and shared `mount_backup_list` / `backup_kind_label` with `settings.rs` (scrollable list + kind badges); added FA translations in `fa_messages.rs`; added `test-support` feature + `Store::corrupt_for_test` in `sarv-quote` and `test_recovery_screen_restores_a_backup` in `crates/sarv-desktop/src/tests/settings.rs`; marked Plan 007 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop && cargo test -p sarv-quote`
- **Result**: PASS (`sarv-desktop`: 63 passed, 2 ignored; `sarv-quote` all non-ignored tests pass).

## 2026-11-15 — P08-T01 (Plan 008: Data directory & file safety)
- **Changes**: Extracted pure `books_dir(os, get)` with per-OS defaults (`SARV_DATA_DIR`, Windows `%APPDATA%`, macOS `Library/Application Support`, Linux `XDG_DATA_HOME`/`HOME`), legacy location fallback, and `open_error` surfacing in `crates/sarv-desktop/src/data.rs` (+ 6 unit tests); added `"Books folder"` row in `settings.rs` and `fa_messages.rs`; enforced `0o700` dir / `0o600` SQLite & backup permissions on Unix, portable `write_new_inner` fallback when `hard_link` is unsupported, and `sweep_stale_staging` on open in `crates/sarv-quote/src/store.rs`; added 2 tests in `crates/sarv-quote/tests/storage_hardening.rs`; updated `README.md`; marked Plan 008 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-quote --test export_hardening --test storage_hardening --test process_recovery && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 69 passed, 2 ignored; `storage_hardening`: 18 passed; `process_recovery`: 10 passed).

## 2026-11-15 — P11-T01 (Plan 011: Sarv identity and design system)
- **Changes**: Restored teal `sarv_accent(dark)` in `crates/sarv-desktop/src/ui/kit.rs` with unit test `sarv_kit_uses_sarv_teal_accent`; made `mount_toasts` in `shell.rs` and `Ui::badge` in `glass.rs` tone-aware; added `DIALOG_SM`, `DIALOG_MD`, `DIALOG_LG`, and `SHEET_RADIUS` tokens in `glass.rs` and replaced literal radii/widths across views; archived old `docs/DESIGN.md` to `docs/milestones/design-gtk-v1.md` and wrote the 9-section Rune `docs/DESIGN.md`; updated `docs/FRONTEND-UI-GUIDE.md`; marked Plan 011 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop && test $(grep -c "libadwaita\|GTK" docs/DESIGN.md) -eq 0`
- **Result**: PASS (`sarv-desktop`: 70 passed, 2 ignored; `DESIGN.md` GTK/libadwaita count = 0).

## 2026-11-15 — P13-T01 (Plan 013 Steps 1–3: Reusable Confirm dialog & confirmations)
- **Changes**: Created `crates/sarv-desktop/src/ui/confirm.rs` (`Confirm`, `ConfirmChoice`, `Confirm::one`, `mount`) mounted in `ui/mod.rs` and held in `app::State.confirm`; split `ops::post_operation` into `review_operation` and `post_reviewed`; wired `Confirm` dialogs for sales/purchase returns (`btn-Post return`), journal reverse (`btn-Reverse entry`), fiscal period lock/reopen (`btn-Lock period`/`btn-Reopen period`), and cheque actions (`mint_key`, `_generation`, per-account buttons on `cheque_clear`); added FA translations and updated tests.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 70 passed, 2 ignored).

## 2026-11-15 — P13-T02 (Plan 013 Steps 4–7: form_dialog, dirty guards, and 5 tests)
- **Changes**: Added `Ui::form_dialog` in `crates/sarv-desktop/src/ui/glass.rs` using `rune`'s `El::modal(false, ...)` to disable outside-click dismissal while keeping focus trapping; added `State::request_close_top_dialog` for `Key::Escape` in `main.rs`; added `Forms::any_with_prefix` / `snapshot_prefix` in `app.rs` and `confirm::discard_form` + `close_or_confirm` across all 11 form dialogs; added `" · unsaved changes"` and `nav`/`select_company` dirty guard for Sheets; added 5 tests (`test_return_requires_confirmation`, `test_reverse_requires_confirmation`, `test_cheque_clear_offers_fund_accounts`, `test_dirty_sale_survives_cancel`, `test_sheet_dirty_blocks_navigation`); marked Plan 013 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 75 passed, 2 ignored).

## 2026-11-15 — P14-T01 (Plan 014: Jalali-aware date field)
- **Changes**: Added `Fmt::parse_date` and `Fmt::date_input` (+ unit test) in `crates/sarv-desktop/src/i18n.rs`; updated `setup.rs::iso_or_parsed` to use `parse_date`; added `Ui::date_field` in `glass.rs` (with live formatted echo & fiscal-year bounds warning) and `State::form_date` in `app.rs`; wired date fields into journal, banking, payment/cheque (`pay-due`), transfer, sale (`sale-date`), and purchase (`buy-date`) dialogs and passed `date` through `ops::post_operation` / `review_operation`; added 4 tests (`test_journal_posts_on_the_typed_jalali_date`, `test_invalid_date_blocks_the_post`, `test_cheque_due_date_accepts_jalali`, `test_backdated_sale_chain_uses_one_date`); marked Plan 014 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 80 passed, 2 ignored).

## 2026-11-15 — P16-T01 (Plan 016: Copy, validation, and settings persistence)
- **Changes**: Replaced label-as-error toasts with real validation sentences (`parse_positive_money`, Sayad 16-digit check, empty-selection & warehouse guards); added `Ui::btn_enabled` in `glass.rs` for selection-dependent header actions; localized default book strings (`Manual journal`, `Payment`, `Goods sales order`, `Recorded in Sarv`, `Piece`, `Main Cash`, `Main Bank`, etc.) and phone placeholders; added `every_ui_key_has_a_persian_entry` in `i18n.rs` and missing FA translations; added immediate `State::save_preferences` across Settings/sidebar/shortcuts and preserved `persian_digits` on `set_lang`; added 2 tests (`test_theme_toggle_persists_across_restart`, `test_language_switch_keeps_digit_choice`); marked Plan 016 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 83 passed, 2 ignored).

## 2026-11-15 — P12-T01 (Plan 012: Table sorting and row models)
- **Changes**: Created `crates/sarv-desktop/src/ui/rows.rs` (`Row`, `SortKey`, `fold_text`, stable `sort_rows`, `reapply_sort`, `FilterCache`, 3 unit tests); added `State::stamp` and `views::ensure_rows`; converted all tables across all 8 views (`sales`, `purchases`, `payments`, `banking`, `contacts`, `inventory`, `accounting`, `reports`) to cached `Row` models with `.sortable()` and `.on_sort()`; removed per-cell list building; added 3 view sort tests (`test_sales_table_sorts_by_amount`, `test_accounting_coa_sorts_by_code`, `test_contacts_table_sorts_by_name_and_keeps_search`); marked Plan 012 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 89 passed, 2 ignored; `on_sort` count = 15; per-cell list-building grep = empty).

## 2026-11-15 — P15-T01 (Plan 015: Shortcuts and command palette)
- **Changes**: Created `crates/sarv-desktop/src/shortcuts.rs` (`CREATE` table); wired `mod+alt+<key>`, `F1`, `F9`, and palette `Up`/`Down`/`Enter` in `main.rs`; added `Forms::any_focused`, `State::any_dialog_open`, `palette_cursor`, and `palette_last_query` in `app.rs`; updated `shell.rs` palette with cursor highlight, deep links to contact/product profiles, and all `CREATE` actions; generated Settings shortcuts list from the table; added `i18n::fold` (+ unit test) and wired Persian search folding across palette/Contacts/Inventory/rows; updated `README.md`; added 6 tests in `crates/sarv-desktop/src/tests/shell.rs`; marked Plan 015 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 96 passed, 2 ignored).

## 2026-11-15 — P17-T01 (Plan 017 Steps 1–2: Multi-period books & `period_add`)
- **Changes**: Added `"period_add"` action in `crates/sarv-quote/src/desktop_bridge.rs` (requires `expected_generation`, enforces contiguous start date after previous period's end, not exposed in `agent.rs`); created `crates/sarv-quote/tests/fiscal_years.rs` verifying multi-period P&L isolation, cumulative balance-sheet/position carry-over, and `period_add` happy/overlap/gap/stale-generation paths.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet -p sarv-quote --all-targets -- -D warnings && cargo test -p sarv-quote --test fiscal_years && cargo test -p sarv-quote`
- **Result**: PASS (`fiscal_years`: 2 tests covering all 5 scenarios; all `sarv-quote` non-ignored tests pass).

## 2026-11-15 — P17-T02 (Plan 017 Steps 3–5: Desktop fiscal-year switcher & "New fiscal year")
- **Changes**: Updated `Books::pick_period` in `crates/sarv-desktop/src/data.rs` to select latest period when `today()` is past the last period; replaced static fiscal-year badge in `shell.rs` with clickable `fiscal-year-switch` dialog + `"Viewing a past fiscal year"` warning badge; added `next_fiscal_period` in `setup.rs`; added `"New fiscal year"` dialog in `accounting.rs` and 30-day/past expiry notice in `overview.rs`; added FA translations and 2 desktop tests (`test_open_next_fiscal_year_and_post_in_it`, `test_switching_fiscal_year_changes_the_report_range`); marked Plan 017 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 98 passed, 2 ignored).

## 2026-11-15 — P19-T01 (Plan 019 Steps 1–2: Paged `records` & `ledger` running balance)
- **Changes**: Added read-only `"records"` action (paged, filtered by `party_id`/`from`/`to`/`query`) and `"opening_balance"` + per-row running `"balance"` on `"ledger"` in `crates/sarv-quote/src/desktop_bridge.rs`; exposed `sarv_records` in `crates/sarv-quote/src/agent.rs` `READ`; added `records_pages_and_filters_match_workspace` and `ledger_computes_opening_balance_and_running_balance_across_pages` in `crates/sarv-quote/tests/desktop_bridge.rs` (and updated tool count in `agent_access.rs`).
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-quote --test desktop_bridge --test agent_access`
- **Result**: PASS (`desktop_bridge`: 24 passed; `agent_access`: 9 passed, 1 ignored).

## 2026-11-15 — P18-T01 (Plan 018: Chart of accounts)
- **Changes**: Added `"account_add"` and `"account_archive"` in `crates/sarv-quote/src/desktop_bridge.rs` (generation-checked, not exposed to `agent.rs`) with 5 tests in `crates/sarv-quote/tests/chart_accounts.rs`; filtered archived/non-postable accounts in `crates/sarv-desktop/src/ui/pickers.rs`; added `"New account"`, `"Archive account"`, and `"Show archived"` toggle in `crates/sarv-desktop/src/ui/views/accounting.rs` (+ `ui/mod.rs`, `app.rs`, `fa_messages.rs`); added 3 desktop tests in `crates/sarv-desktop/src/tests/accounting.rs`; marked Plan 018 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-quote --test chart_accounts && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`chart_accounts`: 5 passed; `sarv-desktop`: 101 passed, 2 ignored).

## 2026-11-15 — P19-T02 (Plan 019 Steps 3–6: Load older, record detail sheet, & full ledger)
- **Changes**: Added `Books::load_more` in `crates/sarv-desktop/src/data.rs` and `Ui::load_more_footer` in `glass.rs` across Sales/Drafts/Purchases/Payments/Accounting; created `crates/sarv-desktop/src/ui/detail.rs` (`DetailState`, `open`, `mount`) mounted in `ui/mod.rs` and wired to `btn-Open` / `Table::on_open` across views; upgraded `mount_ledger` in `accounting.rs` to full `DIALOG_LG` `Table` with `opening_balance`, running `balance`, `"Load older"` pager, and journal drill-down; wired Trial balance & Balance sheet drill-down in `reports.rs`; added 4 tests in `tests/accounting.rs`; marked Plan 019 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 105 passed, 2 ignored; `take(12)` removed; `"offset"` count = 3).

## 2026-11-15 — P20-T01 (Plan 020: Document lifecycle)
- **Changes**: Created `crates/sarv-desktop/src/ui/operation_dialog.rs` (`OperationState`, `open`, `mount`, integer-only `parse_thousandths` + unit test, preview with `expected_generation` -> `Confirm`); wired `"Receive goods"`, `"Enter bill"`, and partial `"Return"` in `purchases.rs`; wired partial `"Deliver & invoice"` and `"Return"` in `sales.rs`; added Drafts tab `"Edit"` (preserving `quote_issue` and `expected_revision`), `"Convert to sale"`, and `"Delete"` (`draft_delete` via `Confirm`); added FA translations and 7 tests in `tests/{purchases,sales}.rs`; marked Plan 020 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 113 passed, 2 ignored).

## 2026-11-15 — P21-T01 (Plan 021: Payments and cheques)
- **Changes**: Added `"Reverse payment"` flow (`payment_reverse_preview` -> `Confirm` -> `payment_post` reversal) and `"Reversed"` status in `crates/sarv-desktop/src/ui/views/payments.rs`; mapped `"spend"` to `"Endorse to supplier"` (`cheque-spend-dialog` with supplier picker, `build_allocations`, `cheque_spend`); replaced 8-row strip with `"Cheques"` tab `Table` sorted by open-first + due date ascending, action details panel, branch/drawer national ID fields, and live Sayad 16-digit hint; added `"Cheques due this week"` metric card in `overview.rs`; un-ignored the 2 cheque tests and added 4 new tests (`test_payment_reversal_restores_the_open_invoice`, `test_received_cheque_can_be_endorsed_to_a_supplier`, `test_cheque_register_sorts_by_due_date`, `test_overview_counts_cheques_due_within_a_week`); recorded Step 5 finding (Rune setup does not emit `cheques_receivable`/`cheques_payable` opening lines) and marked Plan 021 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 119 passed, 0 failed, 0 ignored).

## 2026-11-15 — P22-T01 (Plan 022: Add company and switcher)
- **Changes**: Added `adding_company` mode (`start_add_company`, `cancel_adding`) in `crates/sarv-desktop/src/ui/views/setup.rs` and `ui/mod.rs`; added `"Add company"` button and always-visible company list in `settings.rs`; replaced `org-selector` action with `mount_company_switcher` (`DIALOG_SM`, dirty-guarded `select_company` + `"Add company"`) in `shell.rs` and `app.rs`; added FA translations and 3 tests in `tests/settings.rs`; marked Plan 022 `DONE` in `plans/README.md`.
- **Checks**: `cargo fmt --all -- --check && cargo clippy --quiet --workspace --all-targets -- -D warnings && cargo test -p sarv-desktop --bin sarv-desktop`
- **Result**: PASS (`sarv-desktop`: 122 passed, 0 failed, 0 ignored).

## P24-T01: Multi-line manual journal (Plan 024)
- **Changed**:
  - `crates/sarv-desktop/src/ui/views/accounting.rs`: Replaced fixed `debit_account`/`credit_account` with dynamic `lines: Vec<JournalLine>` (`JournalLine { account_id, combo }`), added `ensure_lines`/`set_line_account`/`remove_line` (shifting `journal-debit-*`, `journal-credit-*`, `journal-desc-*` down and clearing trailing index), dynamic journal grid with `Add line` & per-row `Remove`, totals/difference row, 2-line debit-to-credit mirroring, and multi-line validation + preview/post flow.
  - `crates/sarv-desktop/src/ui/pickers.rs`: Added `account_combo_options`.
  - `crates/sarv-desktop/src/ui/glass.rs`: Exposed `Ui::button_enabled`.
  - `crates/sarv-desktop/src/fa_messages.rs`: Added Persian entries for the 4 new multi-line validation messages.
  - `crates/sarv-desktop/src/tests/{accounting,purchases,walkthrough,settings}.rs`: Updated existing journal tests and added `test_three_line_journal_posts_balanced`, `test_unbalanced_journal_is_blocked_with_one_toast`, `test_removing_a_journal_line_shifts_the_fields`, and `test_two_line_journal_mirrors_the_amount`.
  - `plans/README.md`: Marked Plan `024` as `DONE`.
- **Checks**:
  - `cargo clippy --quiet --workspace --all-targets -- -D warnings` -> exit 0
  - `cargo test -p sarv-desktop --bin sarv-desktop` -> exit 0 (`126 passed; 0 failed; 0 ignored`)
