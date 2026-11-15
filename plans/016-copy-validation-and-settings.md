# Plan 016: Clear validation messages, Persian text in Persian books, and settings that persist

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src`

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW
- **Depends on**: 004, 005 (same handlers; land them first)
- **Category**: bug / UX
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Three kinds of rough edges make the app feel unfinished:

1. **Validation toasts reuse unrelated labels.** A missing amount toasts the single word "Amount"; an incomplete cheque toasts "Cheque"; printing with nothing selected says "Select contact"; choosing the same warehouse twice says "Select warehouse". Users are not told what is wrong.
2. **English text is written into Persian books.** Default reasons such as "Manual journal", "Payment", "Goods sales order", "Transfer", "Reverse entry", "Desktop", the default unit name "Piece", and the opening accounts "Main Cash"/"Main Bank" are hard-coded English strings sent to the engine and shown back in journals and lists.
3. **Preferences do not stick.** Theme, language, digits, currency unit and calendar change live but are saved only by "Save preferences"; the sidebar and ⌘L/⌘D toggles never save. Switching language silently overwrites the digit choice.

## Current state

- i18n: `crate::i18n::tr(lang, key)` — English key is the fallback; Persian in `crates/sarv-desktop/src/fa_messages.rs` (`pub const FA: &[(&str, &str)]`). `ui.t(..)` / `st.fmt.t(..)` call it. An existing unit test `translations_resolve_and_fall_back` is in `i18n.rs`.
- Label-as-error sites (verify each with grep before editing):
  - `ui/views/accounting.rs:255` `st.fmt.t("Select account")` (fine), `:261` `st.fmt.t("Amount")`
  - `ui/views/payments.rs` ~257 `t("Amount")`, ~324 `t("Cheque")`
  - `ui/views/setup.rs` ~78 `t("Name")`
  - `ui/views/sales.rs:807` `t("Select contact")` when nothing is selected for printing
  - `ui/views/purchases.rs` ~521 `t("Goods that have arrived.")`
  - `ui/views/inventory.rs:480` `t("Select product")` (also used when a warehouse is missing), `:484` `t("Select warehouse")` when source == target
  - `fa_messages.rs` already has `"A Sayad ID has exactly 16 digits."`, unused.
- English written to the books:
  - `ui/views/accounting.rs:267` `"Manual journal"`, `:340` `"Reverse entry"`, `:372` `"Reopen fiscal period"` / `"Lock fiscal period"`
  - `ui/views/payments.rs:296` `"Payment"`
  - `ui/views/sales.rs:192` `"Goods sales order"`, `:238` `"Deliver goods and issue invoice"`, `:833` `"Restock customer return and issue credit"`
  - `ui/views/banking.rs` ~179 `"Transfer"`
  - `app.rs:328` `payload["reason"] = "Desktop"`
  - `ui/resources.rs` ~112 `"default_unit_name": "Piece"`; `ui/views/inventory.rs` ~57 `"Unit"`
  - `ui/views/setup.rs:336,349` `"Main Cash"`, `"Main Bank"`, `:307` `"Opening balances"`
- Hard-coded Persian placeholders shown even in English mode: `grep -rn '"[۰-۹٬٫ -]\+"' crates/sarv-desktop/src/ui/views` (e.g. phone `"۰۲۱-۸۸۸۸۸۸۸۸"` in contacts).
- Missing FA entries: the label `"Number"` (`ui/views/payments.rs:226`) has none.
- Settings: `ui/views/settings.rs:79-117` — switches mutate `st.dark`, `st.fmt.*` directly; persistence only in the "Save preferences" handler at `:220-247`, which builds a `settings` JSON from `st.books.workspace["settings"]` and calls `st.run("settings", settings, Some("Preferences saved"))`. `ui/shell.rs:101-116` (sidebar language/theme rows) and `main.rs:44-52` (⌘D, ⌘L) only mutate state. `app.rs:260-263`: `set_lang` sets `persian_digits = lang == Lang::Fa`.
- Buttons that need a selection are always enabled: Sales "Print"/"Return", Purchases "Return", Accounting "Ledger"/"Reverse"/"Lock fiscal period". `Ui::button`/`btn` (`ui/glass.rs:74-86`) have no disabled parameter; check rune-kit `Button` for `.disabled(bool)` (`grep -n "pub fn disabled" ~/.cargo/git/checkouts/rune-*/b556a38/crates/rune-kit/src/button.rs`).

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-desktop/src/**` (views, `app.rs`, `i18n.rs`, `fa_messages.rs`, `ui/glass.rs`, `ui/shell.rs`, `main.rs`).

**Out of scope**: translating the **engine's** error messages (they are Persian strings from `sarv-quote`; showing them is by design); changing text already stored in existing books.

## Git workflow

- Branch `advisor/016-copy-validation-and-settings`; commits `fix(desktop): say what is wrong in validation toasts`, `fix(desktop): write Persian reasons into Persian books`, `feat(desktop): preferences save as they change`, `test(desktop): every UI string has a Persian entry`.

## Steps

### Step 1: Real validation messages

Replace each label-as-error with a sentence key and add its Persian text:

| Situation | Key |
|---|---|
| Amount missing/zero | `Enter an amount greater than zero.` (if plan 005 landed, reuse its `Enter a valid amount.` for parse errors) |
| Cheque fields incomplete | `Enter the cheque's Sayad ID, number and due date.` |
| Sayad ID not 16 digits (new check, advisory wording, still block: the engine requires it — confirm in `crates/sarv-quote/src/cheques.rs`; if the engine does not require 16 digits, make it a warning toast and continue) | `A Sayad ID has exactly 16 digits.` |
| Company name empty in setup | `Enter the business name.` |
| Nothing selected (print/return/reverse/ledger/lock) | `Select a row first.` |
| Purchase return on a non-received order | `Only received goods can be returned.` |
| Transfer missing fields | `Choose the product and both warehouses.` |
| Transfer same warehouse | `Choose two different warehouses.` |

### Step 2: Disable actions that need a selection

Add `Ui::btn_enabled(kind, key, glyph, enabled: bool, on_press)` in `glass.rs` using the kit's disabled state if available; otherwise render the button with `text_faint` colour and a no-op handler. Use it for the header actions listed in "Current state", enabled only when the relevant `selected` id is non-empty (and of the right kind: `journal-…`/`period-…` prefixes in Accounting).

### Step 3: Localised defaults written to the books

Replace each hard-coded English default with `st.fmt.t("<same English text>")` and add Persian entries, e.g. `("Manual journal", "سند دستی")`, `("Payment", "پرداخت")`, `("Goods sales order", "سفارش فروش کالا")`, `("Deliver goods and issue invoice", "تحویل کالا و صدور فاکتور")`, `("Restock customer return and issue credit", "برگشت از فروش و صدور اعتبار")`, `("Transfer", "انتقال")`, `("Reverse entry", "سند معکوس")`, `("Piece", "عدد")`, `("Main Cash", "صندوق اصلی")`, `("Main Bank", "بانک اصلی")`, `("Opening balances", "تراز افتتاحیه")`. For `app.rs:328` use `self.fmt.t("Recorded in Sarv")` (`"ثبت در سرو"`).

Check first whether any engine or desktop code compares these strings (e.g. a Drafts status column that matches `"Goods sales order"`, or `"Piece"` used to find an existing unit): `grep -rn '"Goods sales order"\|"Piece"\|"Main Cash"' crates/`. Where a comparison exists, match against both languages (`i18n::tr(Lang::Fa, k)` and `tr(Lang::En, k)`).

Placeholders: replace hard-coded Persian-digit placeholders with `s.fmt.digits("021-88888888")`-style calls so they follow the digits setting.

### Step 4: A test that keeps the dictionary complete

Add a unit test in `i18n.rs`, `every_ui_key_has_a_persian_entry`: it reads the crate's own sources at test time (`std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))` recursively), extracts string literals passed to `.t("…")`, `ui.t("…")`, `fmt.t("…")`, `ui.btn(_, "…"`, `ui.labeled("…"`, `ui.wrap_btn(_, "…"`, `ui.compact_btn(_, "…"`, `ui.empty(_, "…", "…")` and `Page::label` with a simple hand-written scanner (no new dependency; a few `find`/`split` calls are enough), and asserts each is in `FA`. Keep an explicit `ALLOW: &[&str]` for keys that are intentionally identical in both languages (e.g. `"English"`, `"POS"`). Fix every missing key the test reports (`"Number"` is one).

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop i18n::` → passes.

### Step 5: Preferences persist immediately

- Add `State::save_preferences(&mut self)` in `app.rs` containing the JSON-building code now in the "Save preferences" handler, calling `self.engine.call("settings", …)` directly and then `self.refresh()` — **without** a success toast; on error, toast it.
- Call it from: the Settings switches and segmented controls, the sidebar language/theme rows, ⌘D/⌘L in `handle_key`, and the POS feature switch. Keep the "Save preferences" button (it now just calls the same function and toasts "Preferences saved"), or remove it if nothing else on the page needs an explicit save — check what else the page edits.
- `set_lang`: stop forcing digits. Only set `persian_digits` from the language when no saved `display_digits` exists (first run).
- Skip saving when the books are not open yet (`!self.books.has_company()`): the welcome screen's language/theme choice is applied to settings once the company is created — read `ui/views/setup.rs` to see where that happens and make sure the chosen values are passed through.

Test `test_theme_toggle_persists_across_restart`: toggle theme and language via the sidebar keys (`theme-toggle`, `lang-toggle`), build a second `State::with_engine(Engine::with_dir(same_dir), true)`, assert `dark` and `lang` match. Test `test_language_switch_keeps_digit_choice`. `test_export_writes_csv_and_language_persists` must still pass.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

## Done criteria

- [ ] `grep -rn 't("Amount"))' crates/sarv-desktop/src/ui/views | grep toast` returns nothing
- [ ] `grep -rn '"Manual journal".into()\|"Payment".into()\|"Desktop".into()' crates/sarv-desktop/src` returns nothing
- [ ] `every_ui_key_has_a_persian_entry` exists and passes
- [ ] Two new persistence tests pass; full desktop suite passes
- [ ] `plans/README.md` updated

## STOP conditions

- The demo fixture or an engine test depends on one of the English default strings (e.g. looks up the unit named "Piece").
- Saving settings on every toggle triggers a "Books changed since review" error in a dialog that was open at the time (a held `_generation` became stale). Then only save when no dialog is open, and report.

## Maintenance notes

- The dictionary test is deliberately simple; when it produces a false positive, add the key to `ALLOW` with a comment rather than weakening the scanner.
- Unused FA keys left from the Tauri app can be pruned later with the inverse check (FA key never referenced).
