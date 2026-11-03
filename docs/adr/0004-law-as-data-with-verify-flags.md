# 0004 — Law as data, flagged `verify`, gated until reviewed

**Status:** accepted; review gate added on the owner's instruction (2026-11-03) · **Date:** 2026-11-03

## Context
Spec principle 10: tax rates, salary-tax brackets and insurance rates are effective-dated data, not
constants. The spec marks several values `⚠ verify`. `decisions.md` (IR-STAT) says no current
legal rule is selected and statutory claims need effective primary sources and professional review.

## Decision
`iran::law` holds `LawValue { value, from_year, to_year, verify, note }` keyed by Jalali year,
resolved through the engine's own calendar. It returns `None` outside listed years instead of guessing.
- Standard VAT 10 %, 1403–1405: `verify = false` for the rate (the owner confirmed 10 %, not 12 %).
- Salary-tax 1405 brackets and the 7/20/3 insurance split: `verify = true`.
- **Interpretation recorded:** the spec's thresholds ("10 % to 80M toman, 15 % to 100M …") are read as
  cumulative *monthly income* with a 0 % first slice (the 40M-toman exemption), not as income after
  the exemption. Hand-calculated goldens pin this reading. If the budget law reads otherwise, change
  the table and the goldens together.
- `payroll()` takes the insurable and taxable bases from the caller; which allowances are exempt is a
  yearly decision, not a code constant. The insurance ceiling/floor is not shipped.

## Consequences
Nothing in the app calls `iran::law` yet. It must not be used for a real invoice, payslip or tax
filing until a professional has reviewed the `verify` items (spec §9 "Open verify items").

## Review gate (owner decision, 2026-11-03)
Every `LawValue` also carries `reviewed: bool`, **false for all values** until the owner says a value is
reviewed. `law::require_reviewed(&value)` passes only when `reviewed && !verify`; any code path that
would use a legal value for a real invoice, payslip or filing must call it. A read-only **Settings → Law
data** tab (`law_data` action) lists every value with "Not reviewed" and "Needs verification" badges and
the notice in English and Persian; `LawNotice` is the component to show wherever a legal value appears.
Tests pin that all values are unreviewed and that the gate blocks sign-off without clearing `verify`.
To record a review: flip `reviewed` for that value in `crates/sarv-quote/src/iran/law.rs` (and clear
`verify` once the source is confirmed) in a reviewed commit.
