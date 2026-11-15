# Plan 023: Amount in Persian words, advisory Iranian ID checks, and a readable law-data table

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/i18n.rs crates/sarv-desktop/src/ui/views/contacts.rs crates/sarv-desktop/src/ui/views/sales.rs crates/sarv-desktop/src/ui/views/settings.rs crates/sarv-quote/src/iran`

## Status

- **Priority**: P2
- **Effort**: M (three independent S-sized parts)
- **Risk**: LOW
- **Depends on**: none
- **Category**: direction (feature)
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

`docs/SPEC-COVERAGE.md` lists as delivered, with tests: "Amount in Persian words under the invoice total", "Iranian ID validators … advisory in the UI", and a "visible Settings → Law data tab". Those lived in the React shell that was removed. In the Rune desktop: there is no amount-in-words anywhere; contact forms are plain fields and the engine's validators (`iran::ids`, `iran::sheba`) are never called; and "Law data" shows a single notice string and none of the entries. These are the inexpensive details that make an Iranian accountant feel the product was built for them.

## Current state

- Engine validators (the desktop already depends on the engine crate, see `crates/sarv-desktop/src/ui/documents.rs` calling `sarv_quote::sales_export`):
  - `sarv_quote::iran::ids::{normalize_digits(&str) -> String, national_id(&str) -> Result<(), IdError>, legal_id(..), postal_code(..), sayad_id(..)}` (`crates/sarv-quote/src/iran/ids.rs:26-82`)
  - `sarv_quote::iran::sheba::{validate(&str) -> Result<(), IdError>, bank_name(&str) -> Option<&'static str>, canonical(&str) -> String}` (`iran/sheba.rs:29-54`)
  - Check that `iran` is a public module (`crates/sarv-quote/src/lib.rs:14`: `pub mod iran;`) and read `IdError`'s variants.
- **ADR-0003** (`docs/adr/0003-advisory-identifier-validation.md`): "Iranian identifier checks are advisory; stored contact data stays free text." Validation must never block saving.
- **ADR-0004** (`docs/adr/0004-law-as-data-with-verify-flags.md`): law values are effective-dated, flagged `verify`, with an owner `reviewed` gate; all are currently unreviewed and "not wired into any calculation". AGENTS.md: "Law data is a read-only notice. It is not an input to a journal." The UI may display them; it must not use them.
- Contact form: `crates/sarv-desktop/src/ui/views/contacts.rs:248-314` `contact_form_fields(s, ui, prefix)` — fields built with `ui.labeled("Label", ui.field(s, &format!("{prefix}-…"), ""))`. List the field keys: `grep -n 'format!("{prefix}-' crates/sarv-desktop/src/ui/views/contacts.rs`. Expect national/tax id, postal code, phone, and possibly a bank account/Sheba field.
- Business profile form (company's own ids): `crates/sarv-desktop/src/ui/views/settings.rs` `mount_business` (~line 255-310).
- Sale dialog total: find where the total is rendered in `crates/sarv-desktop/src/ui/views/sales.rs` (`grep -n "Total" crates/sarv-desktop/src/ui/views/sales.rs`); totals are rial strings formatted with `s.fmt.money_with_unit(..)`.
- Sales PDF: `crates/sarv-quote/src/sales_print.rs` / `sales_export.rs` (`printable_document_pdf`) — check whether the PDF already prints the amount in words (`grep -n "words\|حروف" crates/sarv-quote/src/*.rs`).
- Law data action: `"law_data"` arm in `crates/sarv-quote/src/desktop_bridge.rs` (read its response: `notice_fa`, `notice_en`, `entries`); source `crates/sarv-quote/src/iran/law.rs`. Desktop: `crates/sarv-desktop/src/ui/views/settings.rs:209-217` stores one string in `st.settings.law_notice`.
- Units: money is integer rials; display unit is toman or rial per `s.fmt.unit`; 1 toman = 10 rials.
- i18n: English keys; Persian in `crates/sarv-desktop/src/fa_messages.rs`.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Engine tests | `cargo test -p sarv-quote` | all pass |

## Scope

**In scope**: new `crates/sarv-quote/src/iran/words.rs` (+ `pub mod words;` in `iran/mod.rs`) with tests; `crates/sarv-desktop/src/ui/glass.rs` (hint helper), `src/ui/views/{contacts,settings,sales,pos,payments}.rs`, `src/fa_messages.rs`, `src/main.rs` (tests).

**Out of scope**: making any ID check mandatory; using law values in calculations; economic-code validation (not specified); English number-to-words.

## Git workflow

- Branch `advisor/023-persian-affordances`; commits `feat(engine): Persian number words`, `feat(desktop): amount in words on documents`, `feat(desktop): advisory Iranian ID hints`, `feat(desktop): law data table`.

## Steps

### Step 1: Number to Persian words (engine, pure function)

`crates/sarv-quote/src/iran/words.rs`:

```rust
/// 1_234_567 → "یک میلیون و دویست و سی و چهار هزار و پانصد و شصت و هفت". Zero → "صفر".
pub fn persian_words(n: u128) -> String
/// Amount in words with its unit: rials → "… ریال"; toman uses whole tomans and, when
/// rials % 10 != 0, appends "و N ریال".
pub fn amount_in_words(rials: i128, toman: bool) -> String
```

Rules: groups of three joined by " و "; scale words هزار، میلیون، میلیارد، هزار میلیارد (use "تریلیون" for 10¹² and "کوادریلیون" for 10¹⁵; stop there and fall back to digits beyond); 100–900 irregulars (صد، دویست، سیصد، چهارصد، پانصد، ششصد، هفتصد، هشتصد، نهصد); 10–19 irregulars; "یک هزار" is written "هزار" only when it is the leading group (۱۰۰۰ → "هزار", ۱۰۰۱۰۰۰ → "یک میلیون و هزار" — accept "یک هزار" there if simpler, but pick one and test it); negative amounts prefixed with "منفی ".

Tests (table-driven, ≥20 cases): 0, 1, 10, 11, 19, 20, 21, 100, 101, 110, 999, 1000, 1001, 2000, 10_000, 100_000, 1_000_000, 1_234_567, 1_000_000_000, 12_345_678_901, toman with a 5-rial remainder, negative.

**Verify**: `cargo test -p sarv-quote iran::words` → all pass.

### Step 2: Show the amount in words

- Sale dialog: under the total, a muted line `amount_in_words(total_rials, unit_is_toman)` — only when the UI language is Persian (`s.lang() == Lang::Fa`).
- POS total and the payment dialog's amount field (live, from the parsed field — use plan 005's `parse_money_input`; if 005 has not landed, the existing parser).
- If the sales PDF does not print the amount in words, add one line under the totals in the Persian locale using the same function (file `crates/sarv-quote/src/sales_print.rs` or wherever totals are painted — follow the existing text-drawing calls; the PDF tests in `tests/sales_exports.rs` use `pdftotext`, so add an assertion that the words appear). If touching the PDF requires more than ~20 lines, skip it and note it.

### Step 3: Advisory ID hints

In `glass.rs` add `pub fn hinted(&self, label_key: &str, control: El, hint: Option<(Tone, String)>) -> El` — like `labeled` but with a small line under the control coloured by `tone_fg`.

In `contact_form_fields` and `mount_business`, for each of these fields compute a hint from the current text (empty text → no hint):

| Field | Check | Hint when OK | Hint when not |
|---|---|---|---|
| National ID (10 digits) / legal ID (11 digits) — choose by digit count after `normalize_digits` | `national_id` / `legal_id` | none | Warning: "This national ID does not pass its check digit." / "A national ID has 10 digits; a legal ID has 11." |
| Postal code | `postal_code` | none | Warning: "A postal code has 10 digits." |
| Sheba / IBAN (if the form has one) | `sheba::validate` | Info: bank name from `sheba::bank_name` | Warning: "This Sheba number is not valid." |

Never block Save (ADR-0003). Use the `IdError` variant to pick the message rather than inventing rules.

### Step 4: Law-data table

Replace the single notice string: clicking "Law data" opens a dialog (width 760) with the notice in the UI language at the top (Persian when `lang() == Fa`), then a table of `entries` — name/key, value, effective-from date (`fmt.date`), and a badge "Not reviewed" (Warning) or "Reviewed" per the entry's flags (read `iran/law.rs` for field names). Footer note (muted): "Shown for reference. Sarv does not use these values in any calculation." Persian: «برای اطلاع نمایش داده می‌شود. سرو از این مقادیر در هیچ محاسبه‌ای استفاده نمی‌کند.»

### Step 5: Strings and tests

Add all Persian strings. Tests (`main.rs`; `test_backup_and_law_notice` at line 680 is the existing law test — update it to the dialog):

1. `test_sale_total_shows_amount_in_words_in_persian` and not in English.
2. `test_invalid_national_id_shows_hint_but_saves` — type a 10-digit id with a wrong check digit in the new-contact dialog; the hint element exists; Save still creates the contact.
3. `test_law_data_lists_entries_as_not_reviewed`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass; `cargo test -p sarv-quote` → all pass.

## Done criteria

- [ ] `grep -rn "iran::" crates/sarv-desktop/src | wc -l` ≥ 3
- [ ] `cargo test -p sarv-quote iran::words` passes with ≥20 cases
- [ ] Desktop suite passes with 3 new tests
- [ ] No ID check blocks a save: `grep -n "national_id\|sheba::validate" -A4 crates/sarv-desktop/src/ui/views/contacts.rs` shows no `return` on failure
- [ ] `plans/README.md` updated

## STOP conditions

- The contact profile has no distinct field for an id type you were going to validate — only add hints to fields that exist; do not add fields (that changes the engine's `PartyProfile`).
- Law entries carry values the UI would have to compute (e.g. derive a rate) — display raw data only.

## Maintenance notes

- Have a native speaker review the wording of `persian_words` output for 1000-family edge cases before a release.
- Update `docs/SPEC-COVERAGE.md` rows that still cite `lib/words-fa.ts` and `iran-ids.ts` (plan 032 owns the doc clean-up; mention this plan there).
