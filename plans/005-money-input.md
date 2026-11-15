# Plan 005: One strict money parser and a money field that shows its unit

> **Executor instructions**: Follow this plan step by step. Run every verification command. If a STOP condition occurs, stop and report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat a4776749..HEAD -- crates/sarv-desktop/src/ui/resources.rs crates/sarv-desktop/src/ui/ops.rs crates/sarv-desktop/src/i18n.rs crates/sarv-desktop/src/ui/views`

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED (touches every amount the user types)
- **Depends on**: 004 recommended first (same files)
- **Category**: bug
- **Planned at**: commit `a4776749`, 2026-11-13

## Why this matters

The books store integer rials; users mostly think in toman (1 toman = 10 rials). The desktop has three money-entry defects, all confirmed in code:

- **Silent zero**: `parse_money_input("1234.5")` or `"100 تومان"` returns 0. On first-run setup, a typed opening balance that parses to 0 routes to `initialize_zero`, permanently opening the books empty.
- **Silent ×10**: `typed_display_amount("12.5")` strips the dot and returns `"125"`.
- **Silent truncation**: catalog prices are converted rial→toman with integer division, so a 12,345-rial product posts as 12,340 from POS while the on-screen total says 12,345.

Also, no money input shows which unit it expects.

## Current state

- `crates/sarv-desktop/src/ui/resources.rs:153-157`:
  ```rust
  pub fn parse_money_input(&self, raw: &str) -> i64 {
      let latin = crate::i18n::latin_digits(raw).replace(',', "");
      let amt: i64 = latin.trim().parse().unwrap_or(0);
      if self.fmt.unit == crate::i18n::Unit::Toman { amt * 10 } else { amt }
  }
  ```
- `crates/sarv-desktop/src/ui/ops.rs:24-28`:
  ```rust
  /// Digits the user typed, in the active display unit. Do not convert toman to rial here.
  pub fn typed_display_amount(raw: &str) -> String {
      let digits: String = crate::i18n::latin_digits(raw).chars().filter(|c| c.is_ascii_digit()).collect();
      if digits.is_empty() { "0".into() } else { digits }
  }
  ```
- `crates/sarv-desktop/src/ui/views/pos.rs:72-74`:
  ```rust
  fn display_digits(s: &State, rials: i64) -> String {
      if s.fmt.unit == crate::i18n::Unit::Toman { (rials / 10).to_string() } else { rials.to_string() }
  }
  ```
  Similar truncation: `ui/views/sales.rs` near line 599 (catalog price fill) and `ui/views/inventory.rs` `money_field_from_rial` (near lines 104-113).
- `crates/sarv-desktop/src/i18n.rs:105-116` — `latin_digits` maps Persian/Arabic digits to ASCII, `٫`→`.`, `٬`/`،`→`,`. `i18n.rs:145-164` — `Fmt::money` displays toman with one decimal digit when rials are not a multiple of 10. `Fmt::unit_label()` returns "تومان"/"toman"/"ریال"/"IRR".
- `ui/views/setup.rs:208-213` — "Start fresh" parses `opening-cash`/`opening-bank` and calls `initialize_zero` when both are 0.
- **Wire rules (AGENTS.md, do not change)**: a sales draft sends the typed toman string with `display_unit: Toman`; purchases, payments, journals and banking send **rials**, so those screens convert toman→rial before sending.
- Engine side: `Money::parse(text, MoneyUnit)` in `crates/sarv-quote/src/accounting.rs` — read it (`grep -n "pub fn parse" crates/sarv-quote/src/accounting.rs`) to learn whether a toman string with one decimal (`"1234.5"`) is accepted for `display_unit: Toman`. This decides Step 3.
- Field helper: `Ui::field(s, key, placeholder)` in `ui/glass.rs:151-160`; labelled with `Ui::labeled(label_key, control)`.

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| Desktop tests | `cargo test -p sarv-desktop --bin sarv-desktop` | all pass |
| Unit tests only | `cargo test -p sarv-desktop --bin sarv-desktop i18n::` | pass |

## Scope

**In scope**: `crates/sarv-desktop/src/i18n.rs`, `src/ui/resources.rs`, `src/ui/ops.rs`, `src/ui/glass.rs`, all `src/ui/views/*.rs` call sites of the two parsers, `src/fa_messages.rs`, `src/main.rs` tests.

**Out of scope**: engine crate; changing which unit a given action expects on the wire.

## Git workflow

- Branch `advisor/005-money-input`; commits like `fix(desktop): reject unparseable amounts instead of posting zero`.

## Steps

### Step 1: Strict parser in `i18n.rs`

Add (with unit tests in the existing `#[cfg(test)] mod` at the bottom of `i18n.rs`):

```rust
/// What the user typed, in the active display unit, as exact rials.
/// Accepts Persian/Arabic/Latin digits and grouping separators. In toman, one
/// decimal digit is allowed (0.5 toman = 5 rials). Anything else is an error.
pub fn parse_money(raw: &str, unit: Unit) -> Result<i64, MoneyError>
pub enum MoneyError { Empty, NotANumber, TooPrecise, TooLarge, Negative }
```

Rules: trim; `latin_digits`; remove `,` and spaces; reject any character other than digits and at most one `.`; in `Unit::Rial` a `.` is `TooPrecise`; in `Unit::Toman` at most one fractional digit, result = whole×10 + frac using `checked_mul`/`checked_add` (`TooLarge` on overflow). Empty input → `Empty`.

Tests (table-driven): `"۱۲٬۳۴۵"` toman → 123450; `"1234.5"` toman → 12345; `"1234.56"` toman → `TooPrecise`; `"12.5"` rial → `TooPrecise`; `"100 تومان"` → `NotANumber`; `""` → `Empty`; `"-5"` → `Negative`; 19-digit input → `TooLarge`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop i18n::` → all pass.

### Step 2: Exact rial→field formatting

Add `pub fn money_input(&self, rials: i64) -> String` on `Fmt`: ungrouped, in the active unit and digits, with the one-digit toman fraction when `rials % 10 != 0` (so parse∘format is the identity). Test the round trip for 12345, 12340, 0.

Replace the three truncating conversions (`pos.rs` `display_digits`, the catalog fill in `sales.rs`, `money_field_from_rial` in `inventory.rs`) with it.

### Step 3: Replace the lenient parsers

- Change `State::parse_money_input` to return `Result<i64, MoneyError>` (delegating to `parse_money(raw, self.fmt.unit)`), and fix every caller (`grep -rn "parse_money_input" crates/sarv-desktop/src`). The caller pattern:
  ```rust
  let amt = match st.parse_money_input(&st.forms.get("journal-amount")) {
      Ok(v) if v > 0 => v,
      _ => { st.toast(Tone::Danger, st.fmt.t("Enter a valid amount.")); /* reset posting flag */ return; }
  };
  ```
  Where an empty field legitimately means zero (optional amounts such as "received" on a sale, opening cash/bank), treat `Err(MoneyError::Empty)` as 0 and every other error as a validation failure.
- `typed_display_amount` (sales drafts, POS): if the engine accepts a decimal toman string (see "Current state"), return the validated typed string normalised to Latin digits without grouping; otherwise convert to rials with `parse_money` and send `display_unit: Rial` for that draft. Whichever you choose, apply it to **all** lines of a draft consistently and keep `test_rial_sale_is_not_scaled` green.
- Setup "Start fresh": a parse error must stop with the validation toast; only a true 0/empty goes to `initialize_zero`.

Add to `fa_messages.rs`: `("Enter a valid amount.", "مبلغ معتبر وارد کنید."),`.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass.

### Step 4: `money_field` helper with unit suffix

In `glass.rs` add `pub fn money_field(&self, s: &State, key: &str) -> El`: a `row()` containing `self.field(s, key, &placeholder)` (grow 1) and `self.muted(self.f.unit_label())`, where the placeholder is `self.f.digits("0")`. Replace hard-coded Persian placeholders for amounts (`grep -rn '"۰"\|"۱۵۰٬۰۰۰"' crates/sarv-desktop/src/ui/views`) by using `money_field` for: journal amount, banking amount, payment amount, sale/purchase line price, sale "received", product price/cost (create and profile), opening cash/bank.

**Verify**: `cargo test -p sarv-desktop --bin sarv-desktop` → all pass (field keys are unchanged, so `rt.find("field-…")` lookups still work).

### Step 5: End-to-end tests in `main.rs`

1. `test_pos_posts_exact_rials_for_odd_price`: set a product `sale_price` of `"12345"` (see the `pos` scene in `main.rs:66-89` for how to seed a price), charge it via POS in toman mode, assert the posted sale total is 12345 rials (pattern: `test_pos_charges_the_catalog_and_keeps_the_books`).
2. `test_bad_opening_amount_does_not_open_empty_books`: on a fresh (non-demo) temp profile run through setup to the opening step (pattern: `test_first_run_is_a_welcome_then_the_books`), set `opening-cash` to `"12.345.6"`, click "Start fresh", assert `!rt.state.books.initialized()` and a Danger toast.
3. `test_product_profile_roundtrip_keeps_price`: open a product profile whose price is 12345 rials, save without edits, assert the stored price is still `"12345"`.

## Done criteria

- [ ] `grep -rn "unwrap_or(0)" crates/sarv-desktop/src/ui/resources.rs` no longer matches inside a money parser
- [ ] `grep -rn "rials / 10" crates/sarv-desktop/src` returns nothing
- [ ] Desktop tests pass, including the new parser table test and the 3 flow tests
- [ ] `plans/README.md` updated

## STOP conditions

- The engine's draft `Money::parse` rejects both a decimal toman string and a mixed-unit draft, so odd-rial catalog prices cannot be represented in a toman draft. Report; the fix then belongs in the engine.
- `test_rial_sale_is_not_scaled` or `test_payment_allocates_across_open_items` fails and the cause is not an obvious call-site mistake.

## Maintenance notes

- Any new amount input must use `money_field` + `parse_money_input`. Reviewers: grep the diff for `.parse::<i64>()` on user text.
- Quantities have the same class of problem (`latin_digits(..).replace(',', "")` sent raw); the engine validates them, so they are left alone here.
