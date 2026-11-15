# Plan 014: Users can date their documents — one Jalali-aware date field in every posting dialog

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/i18n.rs crates/sarv-desktop/src/ui/glass.rs crates/sarv-desktop/src/ui/ops.rs crates/sarv-desktop/src/app.rs crates/sarv-desktop/src/ui/views`

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED (dates interact with fiscal periods and stock ordering; the engine enforces both)
- **Depends on**: 004 (dialog `open_*` functions), 005 recommended
- **Category**: direction (feature)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

Every document the desktop posts is dated today: sales, purchases, payments, journals, banking movements, transfers, returns. A bookkeeper cannot enter last week's invoices, cannot post opening-month journals, and cannot record anything once today falls outside the open fiscal year. The one place a date *is* typed — a cheque's due date — sends the raw text to the engine, so a Persian user typing `۱۴۰۵/۰۹/۰۱` gets an error or a misread date. The engine already validates dates against the fiscal period and already ships Jalali parsing; only the UI is missing.

## Current state

- `crates/sarv-desktop/src/app.rs:324-326` (`fill_books_context`): `if blank(&payload["date"]) { payload["date"] = Value::String(crate::data::today()); }`.
- Hard-coded today: `crates/sarv-desktop/src/ui/ops.rs:59` (`let date = crate::data::today();` used for both `date` and `due_date`), `ui/views/sales.rs` (~line 95, draft `document_date`), `ui/views/purchases.rs` (~line 125). Journal dialog (`ui/views/accounting.rs:274-282`), banking dialog (`ui/views/banking.rs` ~179) and transfer (`ui/views/inventory.rs:486-493`) send no date at all.
- Cheque due date: `ui/views/payments.rs:229` (placeholder is an ISO date) and `:322-333` (`"due_date": due.trim()` sent raw).
- The parser exists but is private to setup — `ui/views/setup.rs:29-42`:
  ```rust
  fn iso_or_parsed(s: &State, raw: &str, fallback: &str) -> String {
      let raw = raw.trim();
      if raw.is_empty() { return fallback.to_string(); }
      let cal = if s.fmt.calendar == CalendarKind::Jalali { sarv_quote::calendar::Calendar::Jalali } else { sarv_quote::calendar::Calendar::Gregorian };
      sarv_quote::calendar::parse_date(raw, cal).map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|_| raw.to_string())
  }
  ```
  Note the silent fallback to the raw text on a parse error.
- Display: `Fmt::date(iso)` (`i18n.rs:201-212`) renders an ISO date in the chosen calendar and digits, with `/` separators.
- Engine: `sarv_quote::calendar::{Calendar, parse_date, format_date, from_gregorian}` (`crates/sarv-quote/src/calendar.rs`, supports years 1200–1499 Jalali). The engine rejects a date outside the selected period (`accounting.rs:1348-1351`) with a Persian message.
- The selected period's bounds: `s.books.range()` → `(from_iso, to_iso)` (`data.rs:115-122`).
- Field helpers: `Ui::field(s, key, placeholder)`, `Ui::labeled(label_key, control)` in `ui/glass.rs`.
- Books store Gregorian ISO dates; Jalali is display/entry only (docs/FRONTEND-UI-GUIDE.md). Keep that.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |

## Scope

**In scope**: `crates/sarv-desktop/src/i18n.rs`, `src/ui/glass.rs`, `src/ui/ops.rs`, `src/ui/views/{setup,sales,purchases,payments,accounting,banking,inventory}.rs`, `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: a calendar pop-up picker (typed entry with live feedback first; a picker can follow); changing stored date formats; engine changes.

## Git workflow

- Branch `advisor/014-jalali-date-field`; commits `feat(desktop): shared date field`, then one per dialog family.

## Steps

### Step 1: Strict date parsing in `i18n.rs`

Move the logic out of `setup.rs` into `Fmt`:

```rust
/// What the user typed, in the active calendar, as ISO `YYYY-MM-DD`.
/// Accepts Persian/Arabic/Latin digits and `/`, `-` or `.` separators.
pub fn parse_date(&self, raw: &str) -> Result<String, String>
/// An ISO date as the user would type it in the active calendar (for pre-filling a field).
pub fn date_input(&self, iso: &str) -> String
```

`parse_date` returns `Err(self.t("Enter a valid date."))` instead of echoing the raw text. Normalise digits with `latin_digits` and separators to `-` before calling `sarv_quote::calendar::parse_date` (check what separators it accepts by reading its doc comment and tests in `crates/sarv-quote/tests/calendar.rs`). Also accept a two-part shorthand `MM/DD` meaning "in the current year of the active calendar" only if that is trivial; otherwise skip it.

Unit tests in `i18n.rs`: Jalali `"۱۴۰۵/۰۸/۲۲"` → the right ISO date (compute the expected value with `sarv_quote::calendar` in the test rather than hard-coding); Gregorian mode `"2026-11-13"` → same string; garbage → Err; round trip `parse_date(date_input(iso)) == iso`.

Rewrite `setup.rs::iso_or_parsed` to call `fmt.parse_date` (keep its "empty → fallback" behaviour; on Err show a toast and stop instead of sending raw text).

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass (`test_first_run_is_a_welcome_then_the_books` covers setup dates).

### Step 2: `date_field` helper

In `glass.rs`:

```rust
/// A typed date in the active calendar. Under the field: the parsed date echoed back
/// (muted), or the validation message (danger), or nothing when empty.
pub fn date_field(&self, s: &State, key: &str, label_key: &str) -> El
```

Placeholder = `s.fmt.date_input(&crate::data::today())`. The echo line shows `s.fmt.date(&iso)` plus, when the date is outside `s.books.range()`, the warning `self.t("Outside the selected fiscal year")` in `tone_fg(Tone::Warning)`.

Add to `State` (in `app.rs`): `pub fn form_date(&mut self, key: &str) -> Result<String, ()>` — empty field → `Ok(today())`; valid → `Ok(iso)`; invalid → toast `Enter a valid date.` and `Err(())`.

FA entries: `("Date", …)` probably exists — check; add `("Enter a valid date.", "تاریخ معتبر وارد کنید."),` `("Outside the selected fiscal year", "خارج از سال مالی انتخاب‌شده"),` `("Due date", "تاریخ سررسید"),` if missing.

### Step 3: Add the date to each posting dialog

For each dialog: add `ui.date_field(s, "<prefix>-date", "Date")` near the top of the form; in `open_*` pre-fill nothing (empty = today); in Save call `st.form_date("<prefix>-date")` (return early, resetting `posting`, on `Err`) and put the ISO value in the payload.

| Dialog | Field key | Where the date goes |
|---|---|---|
| Journal (`accounting.rs`) | `journal-date` | `payload["date"]` |
| Banking movement (`banking.rs`) | `bank-date` | `payload["date"]` |
| Payment / cheque (`payments.rs`) | `pay-date`; cheque due date keeps `pay-due` but switches to `date_field` + `form_date` | `payload["date"]`; `cheque.due_date` |
| Stock transfer (`inventory.rs`) | `transfer-date` | `payload["date"]` |
| Sale (`sales.rs`) | `sale-date` | draft `document_date`, and pass the same date down the chain (next step) |
| Purchase (`purchases.rs`) | `buy-date` | purchase payload `date`, and down the chain |

Before each, read the engine arm to confirm the payload key (`grep -n '"journal_preview"\|"banking_preview"\|"transfer_preview"\|"purchase_preview"' -A12 crates/sarv-quote/src/desktop_bridge.rs`; most use `context(&store.state, &p)` which reads `p["date"]`).

### Step 4: Carry the date through chained operations

`ops::post_operation` hard-codes today for `date` and `due_date`. Add a `date: &str` parameter (ISO). Sale flow passes the sale date for fulfil and for the settle payment; purchase flow passes the purchase date for receive and bill. Returns (`return_selected` in sales/purchases) keep today for now (they have no dialog; plan 020 adds one). Due date for bills/invoices stays equal to the document date as today.

### Step 5: Tests (`main.rs`)

1. `test_journal_posts_on_the_typed_jalali_date`: open journal, set `journal-date` to a Jalali date inside the demo fiscal year (build it with `rt.state.fmt.date_input("2026-09-01")`), post, find the journal in `rt.state.books.list("journals")` and assert its `date == "2026-09-01"`.
2. `test_invalid_date_blocks_the_post`: `journal-date` = `"13/45"` → Danger toast, dialog still open, journal count unchanged.
3. `test_cheque_due_date_accepts_jalali` (pattern: `test_cheque_strip_labels_are_translated_keys`).
4. `test_backdated_sale_chain_uses_one_date`: a sale dated 2026-09-01 produces an invoice and payment with that date.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass, plus the i18n unit tests.

## Done criteria

- [ ] `grep -n "fn iso_or_parsed" -A12 crates/sarv-desktop/src/ui/views/setup.rs` shows it delegating to `fmt.parse_date`
- [ ] `grep -rn "date_field(" crates/sarv-desktop/src/ui/views | wc -l` ≥ 7
- [ ] `grep -n '"due_date": due.trim()' crates/sarv-desktop/src/ui/views/payments.rs` returns nothing
- [ ] Desktop tests pass with 4 new flow tests
- [ ] `plans/README.md` updated

## STOP conditions

- The engine rejects a back-dated sale chain for stock-ordering reasons in the demo fixture (a later stock movement already exists): pick a date after the fixture's last movement for the test; if no date works, report.
- `sarv_quote::calendar::parse_date` accepts ambiguous input in a surprising way (e.g. treats a Gregorian-looking year as Jalali). Report with examples rather than adding heuristics.

## Maintenance notes

- Plan 017 adds a fiscal-year selector; once it exists, the "outside the selected fiscal year" hint should offer to switch year.
- A pop-up calendar is a natural follow-up; build it on `date_field` so call sites do not change.
