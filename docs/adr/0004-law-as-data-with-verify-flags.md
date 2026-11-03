# 0004 — Law as data, flagged `verify`, not wired into calculations

**Status:** accepted · **Date:** 2026-11-03

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
