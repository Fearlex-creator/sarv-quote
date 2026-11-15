# Goal Tasks — All Plans in `./plans`

Ordered by Phase and dependency from `plans/README.md`.

---

## Phase 0 — Foundation

### Plan 001: CI runs fmt, clippy and both test suites
- [x] **P01-T01**: Pin Rust toolchain (`rust-toolchain.toml`, `rust-version = "1.85"` in `Cargo.toml` + crates), add `rustfmt.toml` (`max_width = 140`), run `cargo fmt --all`, set `[profile.dev] debug = "line-tables-only"`.
  - Files: `rust-toolchain.toml`, `rustfmt.toml`, `Cargo.toml`, `crates/sarv-quote/Cargo.toml`, `crates/sarv-desktop/Cargo.toml`, `crates/**/*.rs` (formatting only)
  - Done when: `cargo fmt --all -- --check && cargo test -p sarv-desktop --bin sarv-desktop`
- [x] **P01-T02**: Fix all `clippy` errors/warnings across workspace (`-D warnings`), create `.github/workflows/ci.yml`, document `poppler-utils` in `README.md`, update `plans/README.md`.
  - Files: `crates/sarv-quote/src/*.rs`, `crates/sarv-desktop/src/**/*.rs`, `.github/workflows/ci.yml`, `README.md`, `plans/README.md`
  - Done when: `cargo clippy --workspace --all-targets -- -D warnings && python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))"`

### Plan 002: Deterministic test clock
- [x] **P02-T01**: Make `data::today()` overridable via `SARV_TODAY`, pin `Engine::demo()` to `2026-10-08` via `std::sync::Once`, add `test_demo_clock_is_inside_the_fixture_year`, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/data.rs`, `crates/sarv-desktop/src/main.rs`, `plans/README.md`
  - Done when: `SARV_TODAY=2026-10-08 cargo test -p sarv-desktop --bin sarv-desktop test_demo_clock`

### Plan 003: Long-lived Store in desktop
- [x] **P03-T01**: Hold open `Store` in `Engine` (`Mutex<Option<Store>>` or `RefCell`), route `Engine::call` through `refresh_if_stale` + `dispatch_in`, add `with_state` for `ui/documents.rs`, single-query `refresh_if_stale` in `store.rs`, add 2 tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/data.rs`, `crates/sarv-desktop/src/ui/documents.rs`, `crates/sarv-quote/src/store.rs`, `crates/sarv-quote/src/desktop.rs`, `crates/sarv-desktop/src/main.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop && cargo test -p sarv-quote --test storage_hardening --test process_recovery --test agent_access`

### Plan 033: Desktop test coverage & suite organization
- [x] **P33-T01**: Move `mod tests` out of `main.rs` into `crates/sarv-desktop/src/tests/{mod.rs,harness.rs,shell.rs,sales.rs,purchases.rs,payments.rs,accounting.rs,inventory.rs,settings.rs,setup.rs,sheets.rs,pos.rs,walkthrough.rs}` with harness helpers (`type_into`, `danger_toasts`, `assert_no_danger`, `count`, `trial_balance_is_balanced`, `books_validate`).
  - Files: `crates/sarv-desktop/src/main.rs`, `crates/sarv-desktop/src/test_helper.rs`, `crates/sarv-desktop/src/tests/*.rs`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop && test $(wc -l < crates/sarv-desktop/src/main.rs) -lt 140`
- [x] **P33-T02**: Add 15 characterization tests in `crates/sarv-desktop/src/tests/` (`banking.rs` etc.), add engine tests `crates/sarv-quote/tests/{sheets_bridge,contact_profile}.rs`, check `cashbook`/`views` usage, document ignored migration tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/tests/*.rs`, `crates/sarv-quote/tests/sheets_bridge.rs`, `crates/sarv-quote/tests/contact_profile.rs`, `AGENTS.md`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop && cargo test -p sarv-quote --test sheets_bridge --test contact_profile`

---

## Phase 1 — Correctness and Data Safety

### Plan 004: Dialog open & request keys
- [x] **P04-T01**: Add `State::mint_key` in `app.rs`, atomic `uid()` in `resources.rs`, one `open_*` per dialog (`accounting::open_journal`, `payments::open_payment`, `sales::open_sale`, `purchases::open_purchase`, `contacts::open_new_contact`, `inventory::open_new_product`, `inventory::open_transfer`), transfer `request_key`+`posting` guard, clear keys on success, remove double toasts, add 5 tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/app.rs`, `crates/sarv-desktop/src/ui/resources.rs`, `crates/sarv-desktop/src/ui/ops.rs`, `crates/sarv-desktop/src/ui/shell.rs`, `crates/sarv-desktop/src/ui/views/{accounting,payments,sales,purchases,contacts,inventory,overview,banking,pos}.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/main.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 005: Strict money input & unit display
- [x] **P05-T01**: Add `parse_money` / `MoneyError` and `Fmt::money_input` in `i18n.rs`, replace lenient `parse_money_input` and truncating `rials / 10` conversions, add `Ui::money_field` in `glass.rs`, update views & `fa_messages.rs`, add unit + 3 flow tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/i18n.rs`, `crates/sarv-desktop/src/ui/resources.rs`, `crates/sarv-desktop/src/ui/ops.rs`, `crates/sarv-desktop/src/ui/glass.rs`, `crates/sarv-desktop/src/ui/views/*.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 006: Six small confirmed desktop bugs
- [x] **P06-T01**: Fix product profile `minimum_stock_thousandths` string parse, balance sheet "Out of balance" state, `Books.error` refresh/badge surfacing, walk-in customer cross-language lookup + law notice language, period lock/reopen label, restore state reset & safe indexing/slicing/aging keys, add tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/ui/views/{inventory,reports,settings,accounting,payments,sales}.rs`, `crates/sarv-desktop/src/ui/shell.rs`, `crates/sarv-desktop/src/ui/resources.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 007: Backup recovery when books fail to load
- [x] **P07-T01**: Add `Store::recover_from_backup` in `store.rs`, handle `backup_list`/`backup_restore_preview`/`recover_from_backup` when `Store::open` fails in `desktop_bridge.rs`, add `backup_file_kind` + safety backup retention, add `crates/sarv-quote/tests/recovery.rs`.
  - Files: `crates/sarv-quote/src/store.rs`, `crates/sarv-quote/src/desktop_bridge.rs`, `crates/sarv-quote/tests/recovery.rs`
  - Done when: `cargo test -p sarv-quote --test recovery && cargo test -p sarv-quote`
- [x] **P07-T02**: Add `Books::load_failed()` in `data.rs`, route to `views::recovery::view` in `ui/mod.rs`, create `src/ui/views/recovery.rs`, update `settings.rs` restore list + `fa_messages.rs`, add test, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/data.rs`, `crates/sarv-desktop/src/ui/mod.rs`, `crates/sarv-desktop/src/ui/views/{mod.rs,recovery.rs,settings.rs}`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 008: Data directory & file safety
- [x] **P08-T01**: Add `books_dir(os, get)` + legacy fallback + `open_error` in `data.rs`, show "Books folder" in `settings.rs`, private `0o700`/`0o600` Unix permissions & portable `write_new` + `sweep_stale_staging` in `store.rs`, add tests in `storage_hardening.rs` and `data.rs`, update `README.md` and `plans/README.md`.
  - Files: `crates/sarv-desktop/src/data.rs`, `crates/sarv-desktop/src/ui/views/settings.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-quote/src/store.rs`, `crates/sarv-quote/tests/storage_hardening.rs`, `README.md`, `plans/README.md`
  - Done when: `cargo test -p sarv-quote --test export_hardening --test storage_hardening --test process_recovery && cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 010: Engine idempotency and panics
- [x] **P10-T01**: Strict cheque retry matching (`replayed`) in `cheques.rs`, bridge `replay` covers payments & cheques, unique quotation numbering + `max(stored, incoming)` settings `next_number`, non-object payload guard & unwrap removal in `desktop_bridge.rs`, poisoned mutex recovery in `agent.rs`, add 8 engine tests, update `plans/README.md`.
  - Files: `crates/sarv-quote/src/cheques.rs`, `crates/sarv-quote/src/desktop_bridge.rs`, `crates/sarv-quote/src/agent.rs`, `crates/sarv-quote/tests/{cheques,payments_trust,sales_drafts,settings,desktop_bridge}.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-quote && cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 009: Agent access hardening
- [x] **P09-T01**: Decide agent access from in-memory `state.policy.access` + tamper check in `agent.rs`, prevent `local-agent` from reopening locked periods in `desktop_bridge.rs`, require `expected_generation` on `payment_post`/`draft_post`/`purchase_post`/`period_lock`, add 5s total read deadline in `read_message`, add tests in `agent_access.rs`, update `plans/README.md`.
  - Files: `crates/sarv-quote/src/agent.rs`, `crates/sarv-quote/src/desktop_bridge.rs`, `crates/sarv-quote/tests/agent_access.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-quote --test agent_access && cargo test -p sarv-quote && cargo test -p sarv-desktop --bin sarv-desktop`

---

## Phase 2 — UI System

### Plan 011: Sarv identity and design system
- [x] **P11-T01**: Restore teal `sarv_accent` in `kit.rs` (+ unit test), tone-aware toast icons in `shell.rs`/`icons.rs`, tone-aware `badge()` and `DIALOG_SM`/`DIALOG_MD`/`DIALOG_LG`/`SHEET_RADIUS` tokens in `glass.rs` & views, archive old `docs/DESIGN.md` to `docs/milestones/design-gtk-v1.md`, write new `docs/DESIGN.md` + update `docs/FRONTEND-UI-GUIDE.md`, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/ui/{kit.rs,glass.rs,shell.rs,icons.rs}`, `crates/sarv-desktop/src/ui/views/*.rs`, `docs/DESIGN.md`, `docs/milestones/design-gtk-v1.md`, `docs/FRONTEND-UI-GUIDE.md`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop && test $(grep -c "libadwaita\|GTK" docs/DESIGN.md) -eq 0`

### Plan 013: Confirmations and unsaved guards
- [x] **P13-T01**: Create `ui/confirm.rs` (`Confirm` dialog) mounted in `ui/mod.rs`, split `ops::review_operation`/`post_reviewed`, wire confirmations for sales/purchase return, journal reverse, period lock/reopen, and cheques (with fund account choice + `mint_key` + `_generation`).
  - Files: `crates/sarv-desktop/src/app.rs`, `crates/sarv-desktop/src/ui/{mod.rs,confirm.rs,ops.rs}`, `crates/sarv-desktop/src/ui/views/{sales,purchases,accounting,payments}.rs`, `crates/sarv-desktop/src/fa_messages.rs`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`
- [x] **P13-T02**: Add `Ui::form_dialog` in `glass.rs` (no outside-click dismiss), dirty checks (`Forms::any_with_prefix`, `close_or_confirm`, profile snapshots) across all form dialogs, Sheets dirty guard on `nav`/`select_company`, add 5 tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/app.rs`, `crates/sarv-desktop/src/main.rs`, `crates/sarv-desktop/src/ui/glass.rs`, `crates/sarv-desktop/src/ui/views/*.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 014: Jalali-aware date field in every posting dialog
- [x] **P14-T01**: Add `Fmt::parse_date` / `Fmt::date_input` in `i18n.rs` (+ unit tests), update `setup.rs::iso_or_parsed`, add `Ui::date_field` in `glass.rs` + `State::form_date` in `app.rs`, wire into journal/banking/payment+cheque/transfer/sale/purchase dialogs and `ops::post_operation`, add 4 tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/i18n.rs`, `crates/sarv-desktop/src/app.rs`, `crates/sarv-desktop/src/ui/{glass.rs,ops.rs}`, `crates/sarv-desktop/src/ui/views/{setup,sales,purchases,payments,accounting,banking,inventory}.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 016: Copy, validation, and settings persistence
- [x] **P16-T01**: Replace label-as-error toasts with real validation sentences, add `Ui::btn_enabled` and disable header actions without selection, localize default strings written to books, add `every_ui_key_has_a_persian_entry` test in `i18n.rs`, implement immediate `State::save_preferences` without overwriting digit preference on `set_lang`, add 2 tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/i18n.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/app.rs`, `crates/sarv-desktop/src/main.rs`, `crates/sarv-desktop/src/ui/{glass.rs,shell.rs,resources.rs}`, `crates/sarv-desktop/src/ui/views/*.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 012: Table sorting and row models
- [x] **P12-T01**: Create `crates/sarv-desktop/src/ui/rows.rs` (`Row`, `SortKey`, `sort_rows` + unit tests), add `State::stamp` + `views::ensure_rows`, convert all tables across all 8 views (`sales`, `purchases`, `payments`, `banking`, `contacts`, `inventory`, `accounting`, `reports`) to `Row` + `.on_sort()`, add ≥3 view sort tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/ui/{mod.rs,rows.rs}`, `crates/sarv-desktop/src/app.rs`, `crates/sarv-desktop/src/ui/views/{mod.rs,sales.rs,purchases.rs,payments.rs,banking.rs,contacts.rs,inventory.rs,accounting.rs,reports.rs}`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 015: Shortcuts and command palette
- [x] **P15-T01**: Create `src/shortcuts.rs`, wire `mod+alt+<key>` / `F1` / `F9` / `Esc` in `main.rs` `handle_key` (skipping when focused/dialog open), generate Settings shortcuts list, add keyboard navigation (`Up`/`Down`/`Enter`) and deep links to contacts/products in `shell.rs` palette, add `i18n::fold` for Persian search, update `README.md`, add 6 tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/{main.rs,app.rs,shortcuts.rs,i18n.rs,fa_messages.rs}`, `crates/sarv-desktop/src/ui/shell.rs`, `crates/sarv-desktop/src/ui/views/{settings,contacts,inventory,purchases}.rs`, `crates/sarv-desktop/src/tests/*.rs`, `README.md`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

---

## Phase 3 — Full-Year Features

### Plan 017: Fiscal years
- [x] **P17-T01**: Verify multi-period books in `crates/sarv-quote/tests/fiscal_years.rs`, add `"period_add"` action in `desktop_bridge.rs` (gapless check, `expected_generation`), test happy/overlap/gap/stale paths.
  - Files: `crates/sarv-quote/src/desktop_bridge.rs`, `crates/sarv-quote/tests/fiscal_years.rs`
  - Done when: `cargo test -p sarv-quote --test fiscal_years && cargo test -p sarv-quote`
- [x] **P17-T02**: Add fiscal-year switcher + past-year badge in `shell.rs`, update `data.rs::pick_period`, add "New fiscal year" dialog in `accounting.rs` + proactive notice in `overview.rs`, add 2 desktop tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/data.rs`, `crates/sarv-desktop/src/ui/shell.rs`, `crates/sarv-desktop/src/ui/views/{accounting,overview,setup}.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 018: Chart of accounts
- [x] **P18-T01**: Add `"account_add"` and `"account_archive"` in `desktop_bridge.rs` + `crates/sarv-quote/tests/chart_accounts.rs`, add "New account" and "Archive account" dialogs + "Show archived" toggle in `accounting.rs`, filter archived/non-postable in `pickers.rs`, add 3 desktop tests, update `plans/README.md`.
  - Files: `crates/sarv-quote/src/desktop_bridge.rs`, `crates/sarv-quote/tests/chart_accounts.rs`, `crates/sarv-desktop/src/ui/views/accounting.rs`, `crates/sarv-desktop/src/ui/pickers.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-quote --test chart_accounts && cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 019: History paging and record detail
- [x] **P19-T01**: Add `"records"` paged/filtered action and `"ledger"` running balance (`opening_balance`, `balance`) in `desktop_bridge.rs` + tests in `crates/sarv-quote/tests/desktop_bridge.rs`.
  - Files: `crates/sarv-quote/src/desktop_bridge.rs`, `crates/sarv-quote/src/agent.rs`, `crates/sarv-quote/tests/desktop_bridge.rs`
  - Done when: `cargo test -p sarv-quote`
- [x] **P19-T02**: Add `Books::load_more` + "Load older" footer on lists, create `ui/detail.rs` mounted in `ui/mod.rs` and wired across views, upgrade `mount_ledger` to full `Table` with running balance & pager + drill-down from `reports.rs`, add 4 desktop tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/data.rs`, `crates/sarv-desktop/src/app.rs`, `crates/sarv-desktop/src/ui/{mod.rs,detail.rs}`, `crates/sarv-desktop/src/ui/views/{sales,purchases,payments,accounting,reports,banking,overview}.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 020: Document lifecycle
- [x] **P20-T01**: Create `ui/operation_dialog.rs` (editable quantities in display units, thousandths integer parser + unit test, preview + `Confirm`), wire "Receive goods"/"Enter bill"/partial "Return" in `purchases.rs`, partial "Deliver & invoice"/"Return" in `sales.rs`, and Edit/Convert/Delete quotations in `sales.rs` Drafts tab, add 7 tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/app.rs`, `crates/sarv-desktop/src/ui/{mod.rs,ops.rs,operation_dialog.rs}`, `crates/sarv-desktop/src/ui/views/{sales,purchases}.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop && cargo test -p sarv-quote`

### Plan 021: Payments and cheques
- [x] **P21-T01**: Add "Reverse payment" flow + "Reversed" badge, "Endorse to supplier" (`cheque_spend`) flow, full "Cheques" tab register sorted by due date with `date_field`/branch/drawer/Sayad hint, "Cheques due this week" card on `overview.rs`, check opening cheques finding, add 4 tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/ui/views/{payments.rs,cheques.rs,overview.rs,mod.rs}`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop && cargo test -p sarv-quote`

### Plan 022: Add company and switcher
- [x] **P22-T01**: Add `adding_company` mode to `setup.rs` with Cancel, "Add company" in `settings.rs` (always show company list), sidebar `org-selector` company-switch dialog with dirty guard, add 3 tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/ui/views/{setup,settings}.rs`, `crates/sarv-desktop/src/ui/{shell.rs,mod.rs}`, `crates/sarv-desktop/src/app.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 024: Multi-line manual journal
- [ ] **P24-T01**: Replace `debit_account`/`credit_account` with `lines: Vec<JournalLine>` in `accounting.rs`, build multi-line grid with add/remove (`remove_line` shifts fields), totals/difference row, 2-line amount mirroring, validation, update 2 + add 4 tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/ui/views/{accounting,overview}.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 025: Exports and printing
- [ ] **P25-T01**: Add `balance_sheet_csv`, `aging_csv`, `ledger_csv` in `desktop_bridge.rs` + tests in `crates/sarv-quote/tests/desktop_bridge.rs`, add `Toast.open_path` + `toast_file` + `reveal()` + unique timestamped export names in `documents.rs`/`app.rs`/`shell.rs`, add purchase/trial-balance PDFs & per-tab report/ledger exports, add 4 desktop tests, update `plans/README.md`.
  - Files: `crates/sarv-quote/src/desktop_bridge.rs`, `crates/sarv-quote/tests/desktop_bridge.rs`, `crates/sarv-desktop/src/ui/{documents.rs,shell.rs}`, `crates/sarv-desktop/src/app.rs`, `crates/sarv-desktop/src/ui/views/{reports,purchases,sales,accounting,settings}.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-quote --features pdf-export && cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 023: Persian affordances
- [ ] **P23-T01**: Add `crates/sarv-quote/src/iran/words.rs` (`persian_words`, `amount_in_words` + ≥20 unit tests), show amount in words in Persian on sale/POS/payment (and sales PDF if ≤20 lines), add `Ui::hinted` + advisory ID hints on contact/business forms, replace law notice with Law-data table dialog, add/update 3 tests, update `plans/README.md`.
  - Files: `crates/sarv-quote/src/iran/{mod.rs,words.rs}`, `crates/sarv-quote/src/sales_print.rs`, `crates/sarv-desktop/src/ui/glass.rs`, `crates/sarv-desktop/src/ui/views/{contacts,settings,sales,pos,payments}.rs`, `crates/sarv-desktop/src/fa_messages.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-quote --features pdf-export && cargo test -p sarv-desktop --bin sarv-desktop`

---

## Phase 4 — Release

### Plan 030: Crash log and version
- [ ] **P30-T01**: Create `crates/sarv-desktop/src/logfile.rs` (+ unit tests), install panic hook + `--version` flag in `main.rs`, log `State::run` & `Books::refresh` failures, add `build.rs` (`SARV_GIT_SHA`), set `version = "0.2.0"` & `strip = "none"`/`debug = "line-tables-only"` (or fallback), add About section in `settings.rs`, create `CHANGELOG.md`, add 2 tests, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/{logfile.rs,main.rs,app.rs,data.rs,fa_messages.rs}`, `crates/sarv-desktop/src/ui/views/settings.rs`, `crates/sarv-desktop/build.rs`, `crates/sarv-desktop/Cargo.toml`, `Cargo.toml`, `CHANGELOG.md`, `plans/README.md`
  - Done when: `cargo test -p sarv-desktop --bin sarv-desktop && cargo run -p sarv-desktop -- --version`

### Plan 031: Release packaging
- [ ] **P31-T01**: Add `windows_subsystem` attribute in `main.rs`, generate `assets/brand/sarv.ico`, add Windows `winresource` in `build.rs`/`Cargo.toml`, remove `crates/sarv-quote/assets` symlink & update `pdf_fonts.rs`, replace `release-windows.yml` with `release.yml`, create `scripts/package-linux.sh` & update `scripts/install-desktop.sh` (`--uninstall`), create `docs/RELEASING.md` & update `README.md`, update `plans/README.md`.
  - Files: `crates/sarv-desktop/src/main.rs`, `crates/sarv-desktop/build.rs`, `crates/sarv-desktop/Cargo.toml`, `assets/brand/sarv.ico`, `crates/sarv-quote/src/pdf_fonts.rs`, `crates/sarv-quote/assets` (remove), `.github/workflows/release.yml`, `scripts/{package-linux.sh,install-desktop.sh}`, `docs/RELEASING.md`, `README.md`, `plans/README.md`
  - Done when: `test ! -e crates/sarv-quote/assets && cargo test -p sarv-quote --features pdf-export && python3 -c "import yaml,sys; [yaml.safe_load(open(f)) for f in sys.argv[1:]]" .github/workflows/*.yml`

### Plan 032: Docs and dead code
- [ ] **P32-T01**: Truth-pass `README.md`, move historical docs to `docs/history/`, update `docs/{ROADMAP.md,ACCOUNTING-SPEC.md,SPEC-COVERAGE.md,adr/README.md}`, `decisions.md`, `CONTEXT.md`, `migrations.md`, `AGENTS.md`, create `docs/adr/0012-rune-desktop.md`, remove dead code (`phase_views.rs`, `money.rs`, `service.rs`, `rust_decimal`, move `scale_probe.rs` & `AGENT_PLAN.md`), copy demo fixture to `crates/sarv-quote/fixtures/demo-books.json`, update `plans/README.md`.
  - Files: `README.md`, `AGENTS.md`, `CONTEXT.md`, `decisions.md`, `migrations.md`, `Cargo.toml`, `crates/sarv-quote/Cargo.toml`, `crates/sarv-quote/src/{lib.rs,desktop_bridge.rs,phase_views.rs,money.rs,service.rs}`, `crates/sarv-quote/fixtures/*`, `docs/**`, `examples/scale_probe.rs`, `AGENT_PLAN.md`, `plans/README.md`
  - Done when: `cargo build --workspace --all-targets && cargo test -p sarv-quote && cargo test -p sarv-quote --features pdf-export && cargo test -p sarv-desktop --bin sarv-desktop`

---

## Phase 5 — Design Spikes

### Plan 026: Engine-owned numbering and quick sale (ADR-0008 + Part B)
- [ ] **P26-T01**: Write `docs/adr/0008-engine-owned-numbering.md` (status proposed, 8 topics + open questions) and index in `docs/adr/README.md`. Implement Part B: `fulfill_post`/`receive_post`/`bill_post` return created ids in `desktop_bridge.rs` (+ engine test), `sales.rs`/`pos.rs` use returned `invoice_id` and resume half-posted sales without duplicating (+ 2 desktop tests), update `plans/README.md`.
  - Files: `docs/adr/0008-engine-owned-numbering.md`, `docs/adr/README.md`, `crates/sarv-quote/src/desktop_bridge.rs`, `crates/sarv-quote/tests/desktop_bridge.rs`, `crates/sarv-desktop/src/ui/views/{sales,pos}.rs`, `crates/sarv-desktop/src/tests/*.rs`, `plans/README.md`
  - Done when: `cargo test -p sarv-quote && cargo test -p sarv-desktop --bin sarv-desktop`

### Plan 027: Year-end closing design (ADR-0009 + prototype)
- [ ] **P27-T01**: Create `crates/sarv-quote/tests/year_end_prototype.rs`, write `docs/adr/0009-year-end-closing.md` (status proposed, 7 questions + open questions), index in `docs/adr/README.md`, update `plans/README.md`.
  - Files: `crates/sarv-quote/tests/year_end_prototype.rs`, `docs/adr/0009-year-end-closing.md`, `docs/adr/README.md`, `plans/README.md`
  - Done when: `cargo test -p sarv-quote --test year_end_prototype`

### Plan 028: VAT design (ADR-0010 + goldens)
- [ ] **P28-T01**: Create `crates/sarv-quote/tests/vat_prototype.rs` (≥12 golden cases), write `docs/adr/0010-vat-on-documents.md` (status proposed, posting matrix, user-entered rate, open questions), index in `docs/adr/README.md`, update `plans/README.md`.
  - Files: `crates/sarv-quote/tests/vat_prototype.rs`, `docs/adr/0010-vat-on-documents.md`, `docs/adr/README.md`, `plans/README.md`
  - Done when: `cargo test -p sarv-quote --test vat_prototype`

### Plan 029: Bank reconciliation design (ADR-0011 + prototype)
- [ ] **P29-T01**: Create `crates/sarv-quote/tests/reconciliation_prototype.rs` (3 scenarios), write `docs/adr/0011-bank-reconciliation.md` (status proposed, 10 questions, open questions), index in `docs/adr/README.md`, update `plans/README.md`.
  - Files: `crates/sarv-quote/tests/reconciliation_prototype.rs`, `docs/adr/0011-bank-reconciliation.md`, `docs/adr/README.md`, `plans/README.md`
  - Done when: `cargo test -p sarv-quote --test reconciliation_prototype`
